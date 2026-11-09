mod cli;
mod metadata;
mod processor;
mod scanner;
mod state;
mod ui;

use anyhow::{Context, Result};
use clap::Parser;
use console::style;
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    let start_time = Instant::now();

    // Inisialisasi custom threadpool jika user menentukan flag -j / --threads
    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .ok();
    }

    // Resolve direktori target
    let target_dir = std::fs::canonicalize(&args.directory)
        .with_context(|| format!("Direktori tidak ditemukan: {}", args.directory.display()))?;

    println!(
        "{} gasinavif v{}",
        style("🚀").bold(),
        env!("CARGO_PKG_VERSION")
    );
    println!(
        "   Target  : {}",
        style(target_dir.display()).cyan()
    );
    println!(
        "   Quality : {} | Speed: {} | Threads: {}",
        style(args.quality).yellow(),
        style(args.speed).yellow(),
        style(
            args.threads
                .map(|t| t.to_string())
                .unwrap_or_else(|| "auto".to_string())
        )
        .yellow(),
    );

    if args.dry_run {
        println!("   Mode    : {}", style("DRY-RUN (tanpa menulis/menghapus file)").red().bold());
    }

    println!();

    // Bersihkan file sementara (*.tmp) dari sesi sebelumnya jika ada
    let cleaned_tmps = scanner::clean_temp_files(&target_dir)?;
    if cleaned_tmps > 0 && args.verbose {
        println!(
            "{} Membersihkan {} file temporary (.tmp) dari sesi sebelumnya.",
            style("🧹").bold(),
            cleaned_tmps
        );
    }

    // ── Fase 6: State management & Resume ──
    if args.fresh {
        println!("{} Menghapus state lama (--fresh)...", style("🗑").bold());
        state::AppState::reset(&target_dir)?;
    }

    let app_state = Arc::new(Mutex::new(state::AppState::load(&target_dir)?));

    // Setup graceful shutdown handler untuk sinyal Ctrl+C (SIGINT / SIGTERM)
    let interrupted = Arc::new(AtomicBool::new(false));
    {
        let interrupted_clone = Arc::clone(&interrupted);
        let target_dir_clone = target_dir.clone();
        let app_state_clone = Arc::clone(&app_state);

        ctrlc::set_handler(move || {
            interrupted_clone.store(true, Ordering::SeqCst);
            eprintln!();
            eprintln!("{}", style("==================================================================").yellow().bold());
            eprintln!("{}", style("⚠️  [DIHENTIKAN] Proses dihentikan oleh pengguna (Ctrl+C).").yellow().bold());
            eprintln!("Progres tersimpan di '.gasinavif_state/progress.json'.");
            eprintln!("Jalankan ulang perintah yang sama kapan saja untuk melanjutkan!");
            eprintln!("{}", style("==================================================================").yellow().bold());

            // Simpan snapshot state terakhir
            if let Ok(state) = app_state_clone.lock() {
                let _ = state.save(&target_dir_clone);
            }

            // Bersihkan file .tmp yang setengah jadi
            let _ = scanner::clean_temp_files(&target_dir_clone);

            std::process::exit(130);
        })
        .context("Gagal mengaktifkan signal handler Ctrl+C")?;
    }

    // ── Fase 3: Scanning direktori ──
    println!(
        "{} Memindai direktori...",
        style("🔍").bold()
    );

    let scan = scanner::scan_directory(&target_dir, args.verbose)?;

    // Filter file yang sudah pernah diproses (resume checkpoint)
    let stage1: Vec<_> = {
        let state = app_state.lock().unwrap();
        scan.stage1_raw_to_jpg
            .into_iter()
            .filter(|p| !state.is_done(p))
            .collect()
    };

    let stage2: Vec<_> = {
        let state = app_state.lock().unwrap();
        scan.stage2_jpg_to_avif
            .into_iter()
            .filter(|p| !state.is_done(p))
            .collect()
    };

    let raw_to_delete: Vec<_> = scan.raw_to_delete;

    // Tampilkan ringkasan pemindaian
    println!();
    println!(
        "{}",
        style("╔══════════════════════════════════════════╗").cyan()
    );
    println!(
        "{}",
        style("║         📋 HASIL PEMINDAIAN              ║").cyan()
    );
    println!(
        "{}",
        style("╠══════════════════════════════════════════╣").cyan()
    );
    println!(
        "║  RAW tanpa JPG (→ Tahap 1) : {:>5}        ║",
        style(stage1.len()).green().bold()
    );
    println!(
        "║  JPG untuk AVIF (→ Tahap 2): {:>5}        ║",
        style(stage2.len()).green().bold()
    );
    println!(
        "║  RAW duplikat (ada JPG)    : {:>5}        ║",
        style(raw_to_delete.len()).yellow().bold()
    );
    println!(
        "║  Sudah di-skip / selesai   : {:>5}        ║",
        style(scan.skipped.len()).dim()
    );
    println!(
        "║  Ukuran folder awal        : {:>10}   ║",
        style(ui::format_size(scan.total_original_size)).white().bold()
    );
    println!(
        "{}",
        style("╚══════════════════════════════════════════╝").cyan()
    );
    println!();

    // Verbose: daftar file per kategori
    if args.verbose {
        if !stage1.is_empty() {
            println!("{}", style("── Tahap 1: RAW → JPG ──").bold().underlined());
            for p in &stage1 {
                println!("  📷 {}", p.display());
            }
            println!();
        }

        if !stage2.is_empty() {
            println!("{}", style("── Tahap 2: JPG → AVIF ──").bold().underlined());
            for p in &stage2 {
                println!("  🖼  {}", p.display());
            }
            println!();
        }

        if !raw_to_delete.is_empty() {
            println!(
                "{}",
                style("── RAW duplikat (akan dihapus) ──").bold().underlined()
            );
            for p in &raw_to_delete {
                println!("  🗑  {}", p.display());
            }
            println!();
        }
    }

    // Jika tidak ada pekerjaan
    if stage1.is_empty() && stage2.is_empty() && raw_to_delete.is_empty() {
        println!(
            "{} Tidak ada file yang perlu diproses. Semua sudah selesai!",
            style("✅").bold()
        );
        return Ok(());
    }

    // ── Hapus RAW duplikat (kamera shoot RAW+JPG) ──
    if !raw_to_delete.is_empty() {
        if args.keep_raw {
            println!(
                "{} {} RAW duplikat dipertahankan (--keep-raw)",
                style("📌").bold(),
                raw_to_delete.len()
            );
        } else if !args.dry_run {
            println!(
                "{} Menghapus {} RAW duplikat (JPG kamera sudah ada)...",
                style("🗑").bold(),
                style(raw_to_delete.len()).red().bold()
            );
            for raw_path in &raw_to_delete {
                if args.verbose {
                    println!("  [HAPUS-RAW] {}", raw_path.display());
                }
                std::fs::remove_file(raw_path).with_context(|| {
                    format!("Gagal menghapus RAW: {}", raw_path.display())
                })?;
            }
        } else {
            println!(
                "{} [DRY-RUN] {} RAW duplikat akan dihapus",
                style("🗑").bold(),
                raw_to_delete.len()
            );
        }
    }

    // Simpan state awal
    {
        let mut state = app_state.lock().unwrap();
        state.original_total_size = scan.total_original_size;
        state.save(&target_dir)?;
    }

    // Counter statistik
    let stage1_processed = Arc::new(AtomicUsize::new(0));
    let stage2_processed = Arc::new(AtomicUsize::new(0));
    let skipped_count = Arc::new(AtomicUsize::new(0));
    let newly_created_jpgs = Arc::new(Mutex::new(Vec::<PathBuf>::new()));

    // ── TAHAP 1: Konversi RAW → JPG ──
    if !stage1.is_empty() && !args.dry_run {
        println!("{}", style("==> [Tahap 1/2] Memproses File RAW ke JPG...").bold().cyan());
        let pb = ui::create_progress_bar(stage1.len() as u64, "RAW → JPG");

        stage1.par_iter().for_each(|raw_path| {
            if interrupted.load(Ordering::Relaxed) {
                return;
            }

            let res = processor::process_raw_to_jpg(raw_path, 98);
            pb.inc(1);

            match res {
                Ok(processor::ProcessResult::Success { saved }) => {
                    stage1_processed.fetch_add(1, Ordering::Relaxed);
                    let dir = raw_path.parent().unwrap_or_else(|| std::path::Path::new("."));
                    let stem = raw_path.file_stem().and_then(|s| s.to_str()).unwrap_or("raw");
                    let created_jpg = dir.join(format!("{stem}.jpg"));
                    if created_jpg.exists() {
                        newly_created_jpgs.lock().unwrap().push(created_jpg);
                    }

                    if args.verbose {
                        pb.println(format!(
                            "  {} {} (hemat {})",
                            style("[OK-RAW]").cyan().bold(),
                            raw_path.display(),
                            ui::format_size(saved)
                        ));
                    }

                    let mut state = app_state.lock().unwrap();
                    state.mark_completed(raw_path.clone());
                }
                Ok(processor::ProcessResult::SkippedAlreadyTagged) => {
                    skipped_count.fetch_add(1, Ordering::Relaxed);
                    if args.verbose {
                        pb.println(format!(
                            "  {} {}",
                            style("[SKIP-TAG]").dim(),
                            raw_path.display()
                        ));
                    }
                    let mut state = app_state.lock().unwrap();
                    state.mark_skipped(raw_path.clone());
                }
                Ok(processor::ProcessResult::SkippedLarger) => {
                    skipped_count.fetch_add(1, Ordering::Relaxed);
                    if args.verbose {
                        pb.println(format!(
                            "  {} {} (JPG >= RAW, RAW dipertahankan)",
                            style("[BATAL]").yellow().bold(),
                            raw_path.display()
                        ));
                    }
                    let mut state = app_state.lock().unwrap();
                    state.mark_skipped(raw_path.clone());
                }
                Ok(processor::ProcessResult::Failed(err)) => {
                    pb.println(format!(
                        "  {} {}: {}",
                        style("[GAGAL]").red().bold(),
                        raw_path.display(),
                        err
                    ));
                }
                Err(err) => {
                    pb.println(format!(
                        "  {} {}: {}",
                        style("[ERROR]").red().bold(),
                        raw_path.display(),
                        err
                    ));
                }
            }
        });

        pb.finish_with_message("Tahap 1 selesai.");
        println!();

        // Simpan checkpoint state
        let state = app_state.lock().unwrap();
        let _ = state.save(&target_dir);
    }

    // ── TAHAP 2: Konversi JPG → AVIF (termasuk output JPG dari Tahap 1) ──
    let mut all_stage2 = stage2;
    {
        let new_jpgs = newly_created_jpgs.lock().unwrap();
        all_stage2.extend(new_jpgs.clone());
    }

    if !all_stage2.is_empty() && !args.dry_run {
        println!("{}", style("==> [Tahap 2/2] Memproses File JPG ke AVIF...").bold().green());
        let pb = ui::create_progress_bar(all_stage2.len() as u64, "JPG → AVIF");

        all_stage2.par_iter().for_each(|jpg_path| {
            if interrupted.load(Ordering::Relaxed) {
                return;
            }

            let res = processor::process_jpg_to_avif(jpg_path, args.quality, args.speed);
            pb.inc(1);

            match res {
                Ok(processor::ProcessResult::Success { saved }) => {
                    stage2_processed.fetch_add(1, Ordering::Relaxed);
                    if args.verbose {
                        pb.println(format!(
                            "  {} {} (hemat {})",
                            style("[OK-AVIF]").green().bold(),
                            jpg_path.display(),
                            ui::format_size(saved)
                        ));
                    }

                    let mut state = app_state.lock().unwrap();
                    state.mark_completed(jpg_path.clone());
                }
                Ok(processor::ProcessResult::SkippedAlreadyTagged) => {
                    skipped_count.fetch_add(1, Ordering::Relaxed);
                    if args.verbose {
                        pb.println(format!(
                            "  {} {}",
                            style("[SKIP-TAG]").dim(),
                            jpg_path.display()
                        ));
                    }
                    let mut state = app_state.lock().unwrap();
                    state.mark_skipped(jpg_path.clone());
                }
                Ok(processor::ProcessResult::SkippedLarger) => {
                    skipped_count.fetch_add(1, Ordering::Relaxed);
                    if args.verbose {
                        pb.println(format!(
                            "  {} {} (AVIF >= JPG, JPG dipertahankan)",
                            style("[BATAL]").yellow().bold(),
                            jpg_path.display()
                        ));
                    }
                    let mut state = app_state.lock().unwrap();
                    state.mark_skipped(jpg_path.clone());
                }
                Ok(processor::ProcessResult::Failed(err)) => {
                    pb.println(format!(
                        "  {} {}: {}",
                        style("[GAGAL]").red().bold(),
                        jpg_path.display(),
                        err
                    ));
                }
                Err(err) => {
                    pb.println(format!(
                        "  {} {}: {}",
                        style("[ERROR]").red().bold(),
                        jpg_path.display(),
                        err
                    ));
                }
            }
        });

        pb.finish_with_message("Tahap 2 selesai.");
        println!();

        // Simpan checkpoint state
        let state = app_state.lock().unwrap();
        let _ = state.save(&target_dir);
    }

    // ── Hitung ukuran akhir folder ──
    let final_scan = scanner::scan_directory(&target_dir, false)?;
    let final_size = final_scan.total_original_size;
    let initial_size = scan.total_original_size;

    // Bersihkan folder state karena seluruh pekerjaan tuntas
    if !args.dry_run {
        let _ = state::AppState::reset(&target_dir);
    }

    // Cetak laporan statistik akhir
    ui::print_summary(
        initial_size,
        final_size,
        start_time.elapsed(),
        stage1_processed.load(Ordering::Relaxed),
        stage2_processed.load(Ordering::Relaxed),
        skipped_count.load(Ordering::Relaxed),
    );

    Ok(())
}
