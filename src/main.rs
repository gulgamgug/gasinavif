mod cli;
mod metadata;
mod processor;
mod scanner;
mod state;
mod ui;

use anyhow::{Context, Result};
use clap::Parser;
use console::style;
use std::time::Instant;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    let start_time = Instant::now();

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

    // ── Fase 6: State management ──
    if args.fresh {
        println!("{} Menghapus state lama (--fresh)...", style("🗑").bold());
        state::AppState::reset(&target_dir)?;
    }

    let mut app_state = state::AppState::load(&target_dir)?;

    // ── Fase 3: Scanning direktori ──
    println!(
        "{} Memindai direktori...",
        style("🔍").bold()
    );

    let scan = scanner::scan_directory(&target_dir, args.verbose)?;

    // Filter file yang sudah pernah diproses (resume)
    let stage1: Vec<_> = scan
        .stage1_raw_to_jpg
        .into_iter()
        .filter(|p| !app_state.is_done(p))
        .collect();

    let stage2: Vec<_> = scan
        .stage2_jpg_to_avif
        .into_iter()
        .filter(|p| !app_state.is_done(p))
        .collect();

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

    // ── Hapus RAW duplikat (ada pasangan JPG dari kamera) ──
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

    // ── Simpan state awal ──
    app_state.original_total_size = scan.total_original_size;
    app_state.save(&target_dir)?;

    // TODO: Fase 5 — Jalankan Tahap 1 (RAW → JPG) dan Tahap 2 (JPG → AVIF) di sini.
    if !stage1.is_empty() {
        println!(
            "\n{} Tahap 1: Konversi RAW → JPG ({} file) — belum diimplementasi",
            style("⏳").bold(),
            stage1.len()
        );
    }

    if !stage2.is_empty() {
        println!(
            "{} Tahap 2: Konversi JPG → AVIF ({} file) — belum diimplementasi",
            style("⏳").bold(),
            stage2.len()
        );
    }

    // Laporan durasi
    let elapsed = start_time.elapsed();
    println!(
        "\n{} Pemindaian selesai dalam {:.2}s",
        style("⏱").bold(),
        elapsed.as_secs_f64()
    );

    Ok(())
}
