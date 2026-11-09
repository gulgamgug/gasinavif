mod cli;
mod interactive;
mod metadata;
mod processor;
mod scanner;
mod state;
mod tree;
mod ui;

use anyhow::{Context, Result};
use clap::Parser;
use console::style;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    let start_time = Instant::now();

    // Default thread: jumlah logical core CPU jika tidak dispesifikasikan
    let initial_threads = args.threads.unwrap_or_else(num_cpus_available);

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
        "   Quality : {} | Speed: {} | Initial Threads: {}",
        style(args.quality).yellow(),
        style(args.speed).yellow(),
        style(initial_threads).yellow(),
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

    // ── State management & Resume ──
    if args.fresh {
        println!("{} Menghapus state lama (--fresh)...", style("🗑").bold());
        state::AppState::reset(&target_dir)?;
    }

    let app_state = Arc::new(Mutex::new(state::AppState::load(&target_dir)?));

    // ── Inisialisasi Live Controller & Keyboard Listener ──
    let controller = Arc::new(interactive::LiveController::new(initial_threads, args.verbose));
    let _raw_guard = controller.start_keyboard_listener();

    // Setup signal handler Ctrl+C (fallback di samping hotkey listener)
    {
        let ctrl = Arc::clone(&controller);
        let target_dir_clone = target_dir.clone();
        let app_state_clone = Arc::clone(&app_state);

        let _ = ctrlc::set_handler(move || {
            ctrl.is_interrupted.store(true, Ordering::SeqCst);
            let _ = crossterm::terminal::disable_raw_mode();
            eprintln!();
            eprintln!("{}", style("==================================================================").yellow().bold());
            eprintln!("{}", style("⚠️  [DIHENTIKAN] Proses dihentikan oleh pengguna (Ctrl+C).").yellow().bold());
            eprintln!("Progres tersimpan di '.gasinavif_state/progress.json'.");
            eprintln!("Jalankan ulang perintah yang sama kapan saja untuk melanjutkan!");
            eprintln!("{}", style("==================================================================").yellow().bold());

            if let Ok(state) = app_state_clone.lock() {
                let _ = state.save(&target_dir_clone);
            }
            let _ = scanner::clean_temp_files(&target_dir_clone);
            std::process::exit(130);
        });
    }

    // ── Scanning direktori ──
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

    // Verbose: daftar file per kategori jika diminta di awal
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
        let _ = crossterm::terminal::disable_raw_mode();
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
                    println!("{}", ui::badge_deleted_raw(raw_path));
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

    // ── TAHAP 1: Konversi RAW → JPG dengan Live Controls ──
    if !stage1.is_empty() && !args.dry_run && !controller.is_interrupted.load(Ordering::Relaxed) {
        println!("{}", style("==> [Tahap 1/2] Memproses File RAW ke JPG...").bold().cyan());
        let tree_stage1 = Arc::new(tree::build_folder_tree(&target_dir, &stage1));

        interactive::execute_with_dynamic_workers(
            stage1,
            Arc::clone(&controller),
            Arc::clone(&tree_stage1),
            "RAW → JPG",
            {
                let newly_created_jpgs = Arc::clone(&newly_created_jpgs);
                let stage1_processed = Arc::clone(&stage1_processed);
                let skipped_count = Arc::clone(&skipped_count);
                let app_state = Arc::clone(&app_state);
                let ctrl = Arc::clone(&controller);

                move |raw_path: &Path, node: &tree::FolderNode| {
                    let res = processor::process_raw_to_jpg(raw_path, 98);
                    let parent = raw_path.parent().unwrap_or(Path::new("."));
                    node.record_file_completed(parent);

                    match res {
                        Ok(processor::ProcessResult::Success { saved }) => {
                            stage1_processed.fetch_add(1, Ordering::Relaxed);
                            let dir = raw_path.parent().unwrap_or(Path::new("."));
                            let stem = raw_path.file_stem().and_then(|s| s.to_str()).unwrap_or("raw");
                            let created_jpg = dir.join(format!("{stem}.jpg"));
                            if created_jpg.exists() {
                                newly_created_jpgs.lock().unwrap().push(created_jpg);
                            }

                            if ctrl.is_verbose.load(Ordering::Relaxed) {
                                ctrl.log(ui::badge_ok_raw(raw_path, saved));
                            }
                            let mut state = app_state.lock().unwrap();
                            state.mark_completed(raw_path.to_path_buf());
                        }
                        Ok(processor::ProcessResult::SkippedAlreadyTagged) => {
                            skipped_count.fetch_add(1, Ordering::Relaxed);
                            if ctrl.is_verbose.load(Ordering::Relaxed) {
                                ctrl.log(ui::badge_skipped_tagged(raw_path));
                            }
                            let mut state = app_state.lock().unwrap();
                            state.mark_skipped(raw_path.to_path_buf());
                        }
                        Ok(processor::ProcessResult::SkippedLarger) => {
                            skipped_count.fetch_add(1, Ordering::Relaxed);
                            if ctrl.is_verbose.load(Ordering::Relaxed) {
                                ctrl.log(ui::badge_skipped_larger(raw_path, "JPG >= RAW"));
                            }
                            let mut state = app_state.lock().unwrap();
                            state.mark_skipped(raw_path.to_path_buf());
                        }
                        Ok(processor::ProcessResult::Failed(err)) => {
                            ctrl.log(ui::badge_error(raw_path, &err));
                        }
                        Err(err) => {
                            ctrl.log(ui::badge_error(raw_path, &err.to_string()));
                        }
                    }
                }
            },
        );

        // Simpan checkpoint state
        let state = app_state.lock().unwrap();
        let _ = state.save(&target_dir);
    }

    // ── TAHAP 2: Konversi JPG → AVIF dengan Live Controls ──
    let mut all_stage2 = stage2;
    {
        let new_jpgs = newly_created_jpgs.lock().unwrap();
        all_stage2.extend(new_jpgs.clone());
    }

    if !all_stage2.is_empty() && !args.dry_run && !controller.is_interrupted.load(Ordering::Relaxed) {
        println!("{}", style("==> [Tahap 2/2] Memproses File JPG ke AVIF...").bold().green());
        let tree_stage2 = Arc::new(tree::build_folder_tree(&target_dir, &all_stage2));
        let quality = args.quality;
        let speed = args.speed;

        interactive::execute_with_dynamic_workers(
            all_stage2,
            Arc::clone(&controller),
            Arc::clone(&tree_stage2),
            "JPG → AVIF",
            {
                let stage2_processed = Arc::clone(&stage2_processed);
                let skipped_count = Arc::clone(&skipped_count);
                let app_state = Arc::clone(&app_state);
                let ctrl = Arc::clone(&controller);

                move |jpg_path: &Path, node: &tree::FolderNode| {
                    let res = processor::process_jpg_to_avif(jpg_path, quality, speed);
                    let parent = jpg_path.parent().unwrap_or(Path::new("."));
                    node.record_file_completed(parent);

                    match res {
                        Ok(processor::ProcessResult::Success { saved }) => {
                            stage2_processed.fetch_add(1, Ordering::Relaxed);
                            if ctrl.is_verbose.load(Ordering::Relaxed) {
                                ctrl.log(ui::badge_ok_avif(jpg_path, saved));
                            }
                            let mut state = app_state.lock().unwrap();
                            state.mark_completed(jpg_path.to_path_buf());
                        }
                        Ok(processor::ProcessResult::SkippedAlreadyTagged) => {
                            skipped_count.fetch_add(1, Ordering::Relaxed);
                            if ctrl.is_verbose.load(Ordering::Relaxed) {
                                ctrl.log(ui::badge_skipped_tagged(jpg_path));
                            }
                            let mut state = app_state.lock().unwrap();
                            state.mark_skipped(jpg_path.to_path_buf());
                        }
                        Ok(processor::ProcessResult::SkippedLarger) => {
                            skipped_count.fetch_add(1, Ordering::Relaxed);
                            if ctrl.is_verbose.load(Ordering::Relaxed) {
                                ctrl.log(ui::badge_skipped_larger(jpg_path, "AVIF >= JPG"));
                            }
                            let mut state = app_state.lock().unwrap();
                            state.mark_skipped(jpg_path.to_path_buf());
                        }
                        Ok(processor::ProcessResult::Failed(err)) => {
                            ctrl.log(ui::badge_error(jpg_path, &err));
                        }
                        Err(err) => {
                            ctrl.log(ui::badge_error(jpg_path, &err.to_string()));
                        }
                    }
                }
            },
        );

        // Simpan checkpoint state
        let state = app_state.lock().unwrap();
        let _ = state.save(&target_dir);
    }

    // Kembalikan terminal dari raw mode
    let _ = crossterm::terminal::disable_raw_mode();

    // Periksa apakah keluar karena interupsi
    if controller.is_interrupted.load(Ordering::Relaxed) {
        eprintln!();
        eprintln!("{}", style("==================================================================").yellow().bold());
        eprintln!("{}", style("⚠️  [DIHENTIKAN] Proses dihentikan oleh pengguna (Ctrl+C / q).").yellow().bold());
        eprintln!("Progres tersimpan di '.gasinavif_state/progress.json'.");
        eprintln!("Jalankan ulang perintah yang sama kapan saja untuk melanjutkan!");
        eprintln!("{}", style("==================================================================").yellow().bold());

        let _ = scanner::clean_temp_files(&target_dir);
        std::process::exit(130);
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

/// Mendeteksi jumlah core CPU logis yang tersedia
fn num_cpus_available() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
