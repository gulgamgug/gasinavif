//! ui.rs — Progress bar, log badge formatting, dan laporan akhir.
//!
//! Menggunakan `indicatif` dan `console` untuk menghadirkan antarmuka terminal
//! yang modern, mulus, dan bebas flicker.

use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Membuat progress bar modern untuk satu tahap pemrosesan.
pub fn create_progress_bar(total: u64, label: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(&format!(
            "{{spinner:.green}} {label} [{{elapsed_precise}}] [{{bar:35.cyan/blue}}] {{pos}}/{{len}} ({{percent}}%) {{msg}}"
        ))
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .progress_chars("█▉▊▋▌▍▎▏  "),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Format badge [OK-RAW]
pub fn badge_ok_raw(file: &std::path::Path, saved: u64) -> String {
    format!(
        "  {} {} (hemat {})",
        style("[OK-RAW]").cyan().bold(),
        file.display(),
        style(format_size(saved)).green().bold()
    )
}

/// Format badge [OK-AVIF]
pub fn badge_ok_avif(file: &std::path::Path, saved: u64) -> String {
    format!(
        "  {} {} (hemat {})",
        style("[OK-AVIF]").green().bold(),
        file.display(),
        style(format_size(saved)).green().bold()
    )
}

/// Format badge [BATAL] (ukuran hasil kompresi >= file asli)
pub fn badge_skipped_larger(file: &std::path::Path, msg: &str) -> String {
    format!(
        "  {} {} ({})",
        style("[BATAL]").yellow().bold(),
        file.display(),
        style(msg).yellow()
    )
}

/// Format badge [SKIP-TAG] (file sudah pernah diproses)
pub fn badge_skipped_tagged(file: &std::path::Path) -> String {
    format!(
        "  {} {}",
        style("[SKIP-TAG]").dim(),
        style(file.display()).dim()
    )
}

/// Format badge [HAPUS-RAW] (pasangan JPG kamera sudah ada)
pub fn badge_deleted_raw(file: &std::path::Path) -> String {
    format!(
        "  {} {}",
        style("[HAPUS-RAW]").magenta().bold(),
        file.display()
    )
}

/// Format badge [GAGAL] / [ERROR]
pub fn badge_error(file: &std::path::Path, err: &str) -> String {
    format!(
        "  {} {}: {}",
        style("[GAGAL]").red().bold(),
        file.display(),
        err
    )
}

/// Mencetak laporan ringkasan akhir dalam format tabel berbingkai warna.
pub fn print_summary(
    original_size: u64,
    final_size: u64,
    duration: Duration,
    stage1_count: usize,
    stage2_count: usize,
    skipped_count: usize,
) {
    let saved = original_size.saturating_sub(final_size);
    let pct = if original_size > 0 {
        (saved as f64 / original_size as f64) * 100.0
    } else {
        0.0
    };

    let total_files = stage1_count + stage2_count;
    let secs = duration.as_secs_f64().max(0.001);
    let throughput = total_files as f64 / secs;

    let border_color = style("╔══════════════════════════════════════════════════╗").cyan();
    let divider_color = style("╠══════════════════════════════════════════════════╣").cyan();
    let bottom_color = style("╚══════════════════════════════════════════════════╝").cyan();

    println!();
    println!("{border_color}");
    println!(
        "║{:^50}║",
        style("📊 LAPORAN RINGKASAN GASINAVIF").bold().white()
    );
    println!("{divider_color}");
    println!(
        "║  RAW → JPG diproses : {:>25}  ║",
        style(stage1_count).cyan().bold()
    );
    println!(
        "║  JPG → AVIF diproses: {:>25}  ║",
        style(stage2_count).green().bold()
    );
    println!(
        "║  File di-skip       : {:>25}  ║",
        style(skipped_count).dim()
    );
    println!("{divider_color}");
    println!(
        "║  Ukuran awal folder : {:>25}  ║",
        style(format_size(original_size)).white().bold()
    );
    println!(
        "║  Ukuran akhir folder: {:>25}  ║",
        style(format_size(final_size)).white().bold()
    );
    println!(
        "║  Total ruang hemat  : {:>17} ({:.1}%)  ║",
        style(format_size(saved)).green().bold(),
        pct
    );
    println!(
        "║  Durasi eksekusi    : {:>25}  ║",
        style(format_duration(duration)).yellow()
    );
    println!(
        "║  Kecepatan rata-rata: {:>21.1} file/s  ║",
        throughput
    );
    println!("{bottom_color}");
    println!();
}

/// Format bytes ke satuan yang mudah dibaca (B, KB, MB, GB).
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Format durasi ke bentuk mm:ss atau hh:mm:ss.
pub fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    if hours > 0 {
        format!("{hours:02}:{mins:02}:{secs:02}")
    } else {
        format!("{mins:02}:{secs:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1024 * 1024 * 5), "5.00 MB");
        assert_eq!(format_size(1024 * 1024 * 1024 * 2), "2.00 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(45)), "00:45");
        assert_eq!(format_duration(Duration::from_secs(125)), "02:05");
        assert_eq!(format_duration(Duration::from_secs(3665)), "01:01:05");
    }

    #[test]
    fn test_badge_strings() {
        let p = std::path::Path::new("test.jpg");
        let b = badge_ok_avif(p, 5000);
        assert!(b.contains("[OK-AVIF]"));
        assert!(b.contains("test.jpg"));
    }
}
