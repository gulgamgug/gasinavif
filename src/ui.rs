//! ui.rs — Progress bar dan laporan akhir.
//!
//! Menggunakan `indicatif` untuk menampilkan multi-progress bar modern
//! dan rekap statistik efisiensi kompresi di akhir pemrosesan.

use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Membuat progress bar untuk satu tahap pemrosesan.
pub fn create_progress_bar(total: u64, label: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(&format!(
            "{{spinner:.green}} {label} [{{bar:40.cyan/blue}}] {{pos}}/{{len}} ({{eta}})"
        ))
        .unwrap()
        .progress_chars("█▉▊▋▌▍▎▏  "),
    );
    pb.enable_steady_tick(Duration::from_millis(120));
    pb
}

/// Mencetak laporan ringkasan akhir.
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

    println!();
    println!("╔══════════════════════════════════════════╗");
    println!("║         📊 LAPORAN AKHIR GASINAVIF       ║");
    println!("╠══════════════════════════════════════════╣");
    println!("║  RAW → JPG diproses : {:>6}             ║", stage1_count);
    println!("║  JPG → AVIF diproses: {:>6}             ║", stage2_count);
    println!("║  File di-skip       : {:>6}             ║", skipped_count);
    println!("╠══════════════════════════════════════════╣");
    println!(
        "║  Ukuran awal  : {:>10}             ║",
        format_size(original_size)
    );
    println!(
        "║  Ukuran akhir : {:>10}             ║",
        format_size(final_size)
    );
    println!(
        "║  Dihemat      : {:>10} ({:.1}%)       ║",
        format_size(saved),
        pct
    );
    println!(
        "║  Durasi       : {:>10}             ║",
        format_duration(duration)
    );
    println!("╚══════════════════════════════════════════╝");
}

/// Format bytes ke satuan yang mudah dibaca (KB, MB, GB).
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
fn format_duration(d: Duration) -> String {
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
