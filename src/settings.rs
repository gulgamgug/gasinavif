//! settings.rs — Antarmuka TUI interaktif untuk konfigurasi global gasinavif (--settings).
//!
//! Menavigasi menu dan submenu menggunakan tombol panah keyboard:
//!   - Slider kecepatan (Speed: 1 - 10) dengan navigasi kiri/kanan.
//!   - Pengatur kualitas (Quality: 1 - 100) dengan navigasi atas/bawah (step 1).
//!   - Pemilih format chroma subsampling (YUV: 420, 422, 444, 400).
//!   - Auto-save otomatis ke disk (~/.config/gasinavif/config.json) saat keluar/berpindah.

use console::{measure_text_width, style};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::io::{stdout, IsTerminal, Write};
use std::time::Duration;

use crate::config::{get_config_path, AppConfig};

enum ActiveScreen {
    MainMenu,
    SpeedMenu,
    QualityMenu,
    YuvMenu,
    Exit,
}

/// Menjalankan antarmuka interaktif TUI settings
pub fn run_settings_menu() -> anyhow::Result<()> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        let cfg = AppConfig::load();
        println!("Pengaturan Global gasinavif saat ini:");
        println!("  • Speed   : {}", cfg.speed);
        println!("  • Quality : {}", cfg.quality);
        println!("  • YUV     : {}", cfg.yuv);
        println!("File: {}", get_config_path().display());
        return Ok(());
    }

    let mut config = AppConfig::load();
    let mut screen = ActiveScreen::MainMenu;
    let mut main_selected: usize = 0;
    let mut yuv_selected: usize = match config.yuv.as_str() {
        "422" => 1,
        "444" => 2,
        "400" => 3,
        _ => 0,
    };

    let _ = crossterm::terminal::enable_raw_mode();
    let mut prev_lines = 0;

    while !matches!(screen, ActiveScreen::Exit) {
        let mut lines = Vec::new();

        match screen {
            ActiveScreen::MainMenu => {
                render_main_menu(&config, main_selected, &mut lines);
            }
            ActiveScreen::SpeedMenu => {
                render_speed_menu(&config, &mut lines);
            }
            ActiveScreen::QualityMenu => {
                render_quality_menu(&config, &mut lines);
            }
            ActiveScreen::YuvMenu => {
                render_yuv_menu(&config, yuv_selected, &mut lines);
            }
            ActiveScreen::Exit => break,
        }

        // Render frame ke terminal dengan proteksi raw mode (selalu \r dan \x1B[K\r\n)
        if prev_lines > 0 {
            print!("\r\x1B[{}A\r\x1B[J", prev_lines);
        }
        prev_lines = lines.len();
        for line in &lines {
            print!("\r{}\x1B[K\r\n", line);
        }
        let _ = stdout().flush();

        // Tangkap input keyboard
        if let Ok(true) = event::poll(Duration::from_millis(100)) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match screen {
                    ActiveScreen::MainMenu => match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            main_selected = (main_selected + 3) % 4;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            main_selected = (main_selected + 1) % 4;
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => match main_selected {
                            0 => screen = ActiveScreen::SpeedMenu,
                            1 => screen = ActiveScreen::QualityMenu,
                            2 => {
                                yuv_selected = match config.yuv.as_str() {
                                    "422" => 1,
                                    "444" => 2,
                                    "400" => 3,
                                    _ => 0,
                                };
                                screen = ActiveScreen::YuvMenu;
                            }
                            _ => screen = ActiveScreen::Exit,
                        },
                        KeyCode::Esc | KeyCode::Char('q') => {
                            screen = ActiveScreen::Exit;
                        }
                        _ => {}
                    },

                    ActiveScreen::SpeedMenu => match key.code {
                        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('-') => {
                            config.speed = config.speed.saturating_sub(1).max(1);
                            let _ = config.save();
                        }
                        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('+') => {
                            config.speed = (config.speed + 1).min(10);
                            let _ = config.save();
                        }
                        KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => {
                            let _ = config.save();
                            screen = ActiveScreen::MainMenu;
                        }
                        _ => {}
                    },

                    ActiveScreen::QualityMenu => match key.code {
                        KeyCode::Down | KeyCode::Left | KeyCode::Char('j') | KeyCode::Char('-') => {
                            config.quality = config.quality.saturating_sub(1).max(1);
                            let _ = config.save();
                        }
                        KeyCode::Up | KeyCode::Right | KeyCode::Char('k') | KeyCode::Char('+') => {
                            config.quality = (config.quality + 1).min(100);
                            let _ = config.save();
                        }
                        KeyCode::PageDown => {
                            config.quality = config.quality.saturating_sub(10).max(1);
                            let _ = config.save();
                        }
                        KeyCode::PageUp => {
                            config.quality = (config.quality + 10).min(100);
                            let _ = config.save();
                        }
                        KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => {
                            let _ = config.save();
                            screen = ActiveScreen::MainMenu;
                        }
                        _ => {}
                    },

                    ActiveScreen::YuvMenu => match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            yuv_selected = (yuv_selected + 3) % 4;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            yuv_selected = (yuv_selected + 1) % 4;
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            config.yuv = match yuv_selected {
                                1 => "422".to_string(),
                                2 => "444".to_string(),
                                3 => "400".to_string(),
                                _ => "420".to_string(),
                            };
                            let _ = config.save();
                            screen = ActiveScreen::MainMenu;
                        }
                        KeyCode::Esc | KeyCode::Char('q') => {
                            screen = ActiveScreen::MainMenu;
                        }
                        _ => {}
                    },

                    ActiveScreen::Exit => break,
                }
            }
        }
    }

    let _ = crossterm::terminal::disable_raw_mode();
    if prev_lines > 0 {
        print!("\r\x1B[{}A\r\x1B[J", prev_lines);
        let _ = stdout().flush();
    }

    let _ = config.save();

    println!();
    println!("{}", style("╔══════════════════════════════════════════════════════════════╗").green().bold());
    println!("{}", box_center(&style("✅  PENGATURAN BERHASIL DISIMPAN").green().bold().to_string()));
    println!("{}", style("╠══════════════════════════════════════════════════════════════╣").green());
    println!("{}", box_row(&format!("• Kecepatan (Speed)         : {}", style(format!("{} (1-10)", config.speed)).yellow().bold()), 2));
    println!("{}", box_row(&format!("• Kualitas (Quality)        : {}", style(format!("{} / 100", config.quality)).cyan().bold()), 2));
    println!("{}", box_row(&format!("• Format Chroma (YUV)       : {}", style(format!("YUV {}", config.yuv)).magenta().bold()), 2));
    println!("{}", box_row(&format!("• File Konfigurasi          : {}", style("~/.config/gasinavif/config.json").dim()), 2));
    println!("{}", style("╚══════════════════════════════════════════════════════════════╝").green().bold());
    println!();

    Ok(())
}

fn render_main_menu(config: &AppConfig, selected: usize, lines: &mut Vec<String>) {
    lines.push(style("╔══════════════════════════════════════════════════════════════╗").cyan().bold().to_string());
    lines.push(box_center(&style("⚙️   PENGATURAN GLOBAL GASINAVIF").bold().white().to_string()));
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push(box_empty());

    let items = [
        format!("1. Kecepatan Encoder (Speed)  : [ {:>2} ]", config.speed),
        format!("2. Kualitas Gambar (Quality)  : [ {:>3} ]", config.quality),
        format!("3. Format Chroma Subsampling  : [ YUV {} ]", config.yuv),
        "4. Simpan & Keluar (Save & Exit)".to_string(),
    ];

    for (i, item) in items.iter().enumerate() {
        if i == selected {
            let row = format!("▶  {item}");
            lines.push(box_row(&style(row).green().bold().underlined().to_string(), 2));
        } else {
            let row = format!("   {item}");
            lines.push(box_row(&style(row).white().to_string(), 2));
        }
    }

    lines.push(box_empty());
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push(box_center("[↑/↓] Pilih Menu  |  [Enter] Masuk  |  [Esc/q] Simpan & Out"));
    lines.push(style("╚══════════════════════════════════════════════════════════════╝").cyan().bold().to_string());
}

fn render_speed_menu(config: &AppConfig, lines: &mut Vec<String>) {
    lines.push(style("╔══════════════════════════════════════════════════════════════╗").cyan().bold().to_string());
    lines.push(box_center(&style("⚡  PENGATURAN KECEPATAN (SPEED: 1 - 10)").bold().white().to_string()));
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push(box_empty());

    // Slider baris
    let mut slider_str = String::new();
    for s in 1..=10 {
        if s == config.speed {
            slider_str.push_str(&format!(" [{}] ", style(s).green().bold()));
        } else {
            slider_str.push_str(&format!(" {} ", style(s).dim()));
        }
        if s < 10 {
            slider_str.push('─');
        }
    }
    lines.push(box_row(&format!("Kecepatan Aktif: {} / 10", style(config.speed).yellow().bold()), 3));
    lines.push(box_empty());
    lines.push(box_row(&format!("Slider: {slider_str}"), 3));
    lines.push(box_empty());
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push(box_row(&style("Penjelasan Kecepatan yang Dipilih:").bold().to_string(), 2));

    let (desc1, desc2) = match config.speed {
        1..=2 => (
            "⚡ Speed 1 - 2: Sangat Lambat (Kompresi Ekstrem)",
            "Efisiensi ukuran maksimal, waktu kompresi sangat lama.",
        ),
        3..=5 => (
            "⚡ Speed 3 - 5: Lambat - Sedang (Efisiensi Tinggi)",
            "Kompresi sangat padat, bagus untuk arsip permanen.",
        ),
        6..=7 => (
            "⚡ Speed 6 - 7: Seimbang (Optimal)",
            "Keseimbangan ideal antara kecepatan dan efisiensi ukuran.",
        ),
        _ => (
            "⚡ Speed 8 - 10: Sangat Cepat (Rekomendasi Default)",
            "Encoding instan (~0.5s/foto), hemat ukuran ~70-85%.",
        ),
    };

    lines.push(box_row(&style(desc1).yellow().bold().to_string(), 2));
    lines.push(box_row(&style(desc2).dim().to_string(), 2));
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push(box_center("[←/→] Geser Slider  |  [Enter/Esc] Simpan & Kembali"));
    lines.push(style("╚══════════════════════════════════════════════════════════════╝").cyan().bold().to_string());
}

fn render_quality_menu(config: &AppConfig, lines: &mut Vec<String>) {
    lines.push(style("╔══════════════════════════════════════════════════════════════╗").cyan().bold().to_string());
    lines.push(box_center(&style("🎯  PENGATURAN KUALITAS (QUALITY: 1 - 100)").bold().white().to_string()));
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push(box_empty());

    let bar_len = 36;
    let filled = ((config.quality as usize) * bar_len) / 100;
    let empty = bar_len.saturating_sub(filled);
    let bar_visual = format!("{}{}", style("█".repeat(filled)).cyan(), style("░".repeat(empty)).dim());

    lines.push(box_row(&format!("Kualitas Aktif: {} / 100", style(config.quality).yellow().bold()), 3));
    lines.push(box_empty());
    lines.push(box_row(&format!("Bar: [{bar_visual}] {:>3}%", config.quality), 3));
    lines.push(box_empty());
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push(box_row(&style("Penjelasan Rentang Kualitas:").bold().to_string(), 2));

    let (desc1, desc2) = match config.quality {
        1..=20 => (
            "🎯 Rentang 1 - 20 (Sangat Rendah / Kompresi Ekstrem)",
            "Ukuran sangat kecil, artefak kompresi terlihat jelas.",
        ),
        21..=40 => (
            "🎯 Rentang 21 - 40 (Rendah / Hemat Ukuran)",
            "Ukuran file hemat, cocok untuk web atau thumbnail.",
        ),
        41..=60 => (
            "🎯 Rentang 41 - 60 (Seimbang / Rekomendasi Default)",
            "Keseimbangan emas antara efisiensi dan ketajaman foto.",
        ),
        61..=80 => (
            "🎯 Rentang 61 - 80 (Kualitas Tinggi)",
            "Detail tekstur dan gradasi warna dipertahankan penuh.",
        ),
        _ => (
            "🎯 Rentang 81 - 100 (Maksimal / Near Lossless)",
            "Kualitas mendekati lossless murni, ukuran file lebih besar.",
        ),
    };

    lines.push(box_row(&style(desc1).yellow().bold().to_string(), 2));
    lines.push(box_row(&style(desc2).dim().to_string(), 2));
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push(box_center("[↑/↓] Step 1  |  [PgUp/PgDn] Step 10  |  [Esc] Kembali"));
    lines.push(style("╚══════════════════════════════════════════════════════════════╝").cyan().bold().to_string());
}

fn render_yuv_menu(config: &AppConfig, selected: usize, lines: &mut Vec<String>) {
    lines.push(style("╔══════════════════════════════════════════════════════════════╗").cyan().bold().to_string());
    lines.push(box_center(&style("📺  PENGATURAN FORMAT CHROMA (YUV FORMAT)").bold().white().to_string()));
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push(box_empty());

    let options = [
        ("420", "YUV 4:2:0  (Default & Paling Kompatibel)"),
        ("422", "YUV 4:2:2  (Keseimbangan Separuh Warna)"),
        ("444", "YUV 4:4:4  (Ketajaman Penuh Tanpa Subsampling)"),
        ("400", "YUV 4:0:0  (Monokrom / Hitam Putih Murni)"),
    ];

    for (i, (code, label)) in options.iter().enumerate() {
        let is_active = config.yuv == *code;
        let active_badge = if is_active {
            style(" [AKTIF] ").green().bold().to_string()
        } else {
            "         ".to_string()
        };

        let row = format!("{:<43}{}", label, active_badge);
        if i == selected {
            let item_str = format!("▶  {row}");
            lines.push(box_row(&style(item_str).green().bold().underlined().to_string(), 2));
        } else {
            let item_str = format!("   {row}");
            lines.push(box_row(&style(item_str).white().to_string(), 2));
        }
    }

    lines.push(box_empty());
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push(box_row(&style("Penjelasan Format Chroma yang Dipilih:").bold().to_string(), 2));

    let (desc1, desc2) = match selected {
        0 => (
            "📺 YUV 4:2:0 (Standar Industri & Rekomendasi)",
            "Subsampling 2x2: kompatibel universal & kompresi hemat.",
        ),
        1 => (
            "📺 YUV 4:2:2 (Subsampling Horizontal)",
            "Subsampling horizontal: separuh warna, tepi teks tajam.",
        ),
        2 => (
            "📺 YUV 4:4:4 (Penuh Tanpa Subsampling)",
            "Warna 1:1 penuh. Ketajaman sempurna untuk grafis digital.",
        ),
        _ => (
            "📺 YUV 4:0:0 (Monokrom / Grayscale Murni)",
            "Kanal warna dibuang seluruhnya, sangat ringkas untuk B&W.",
        ),
    };

    lines.push(box_row(&style(desc1).yellow().bold().to_string(), 2));
    lines.push(box_row(&style(desc2).dim().to_string(), 2));
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push(box_center("[↑/↓] Pilih  |  [Enter] Terapkan  |  [Esc] Kembali"));
    lines.push(style("╚══════════════════════════════════════════════════════════════╝").cyan().bold().to_string());
}

pub const BOX_INTERIOR_WIDTH: usize = 62;

pub fn box_row(content: &str, left_pad: usize) -> String {
    let width = measure_text_width(content);
    let max_allowed = BOX_INTERIOR_WIDTH.saturating_sub(left_pad);
    if width > max_allowed {
        let plain = console::strip_ansi_codes(content);
        let truncated = if plain.len() > max_allowed {
            format!("{}...", &plain[..max_allowed.saturating_sub(3)])
        } else {
            plain.to_string()
        };
        let t_width = measure_text_width(&truncated);
        let right_pad = BOX_INTERIOR_WIDTH.saturating_sub(left_pad + t_width);
        format!("║{}{truncated}{}║", " ".repeat(left_pad), " ".repeat(right_pad))
    } else {
        let right_pad = BOX_INTERIOR_WIDTH.saturating_sub(left_pad + width);
        format!("║{}{content}{}║", " ".repeat(left_pad), " ".repeat(right_pad))
    }
}

pub fn box_center(content: &str) -> String {
    let width = measure_text_width(content);
    if width > BOX_INTERIOR_WIDTH {
        let plain = console::strip_ansi_codes(content);
        let truncated = if plain.len() > BOX_INTERIOR_WIDTH {
            format!("{}...", &plain[..BOX_INTERIOR_WIDTH.saturating_sub(3)])
        } else {
            plain.to_string()
        };
        let t_width = measure_text_width(&truncated);
        let total_pad = BOX_INTERIOR_WIDTH.saturating_sub(t_width);
        let left_pad = total_pad / 2;
        let right_pad = total_pad - left_pad;
        format!("║{}{truncated}{}║", " ".repeat(left_pad), " ".repeat(right_pad))
    } else {
        let total_pad = BOX_INTERIOR_WIDTH.saturating_sub(width);
        let left_pad = total_pad / 2;
        let right_pad = total_pad - left_pad;
        format!("║{}{content}{}║", " ".repeat(left_pad), " ".repeat(right_pad))
    }
}

pub fn box_empty() -> String {
    format!("║{}║", " ".repeat(BOX_INTERIOR_WIDTH))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_row_width() {
        let text1 = style("▶  1. Kecepatan Encoder (Speed)  : [  8 ]").green().bold().underlined().to_string();
        let row1 = box_row(&text1, 2);
        assert_eq!(measure_text_width(&row1), 64);

        let text2 = "Subsampling 2x2: kompatibel universal & kompresi hemat.";
        let row2 = box_row(text2, 2);
        assert_eq!(measure_text_width(&row2), 64);

        let text_long = "Ini adalah teks yang sangat panjang sekali melebihi kapasitas interior border sehingga harus dipotong aman";
        let row_long = box_row(text_long, 2);
        assert_eq!(measure_text_width(&row_long), 64);

        let center = box_center(&style("⚙️  PENGATURAN GLOBAL GASINAVIF").bold().white().to_string());
        assert_eq!(measure_text_width(&center), 64);

        let empty = box_empty();
        assert_eq!(measure_text_width(&empty), 64);
    }

    #[test]
    fn test_all_screens_exact_64_width() {
        let config = AppConfig::default();

        let mut lines = Vec::new();
        render_main_menu(&config, 0, &mut lines);
        for (idx, line) in lines.iter().enumerate() {
            assert_eq!(
                measure_text_width(line),
                64,
                "Main menu line {idx} width is {} != 64: {line}",
                measure_text_width(line)
            );
        }

        let mut lines = Vec::new();
        render_speed_menu(&config, &mut lines);
        for (idx, line) in lines.iter().enumerate() {
            assert_eq!(
                measure_text_width(line),
                64,
                "Speed menu line {idx} width is {} != 64: {line}",
                measure_text_width(line)
            );
        }

        let mut lines = Vec::new();
        render_quality_menu(&config, &mut lines);
        for (idx, line) in lines.iter().enumerate() {
            assert_eq!(
                measure_text_width(line),
                64,
                "Quality menu line {idx} width is {} != 64: {line}",
                measure_text_width(line)
            );
        }

        let mut lines = Vec::new();
        render_yuv_menu(&config, 0, &mut lines);
        for (idx, line) in lines.iter().enumerate() {
            assert_eq!(
                measure_text_width(line),
                64,
                "YUV menu line {idx} width is {} != 64: {line}",
                measure_text_width(line)
            );
        }

        // Test Success Box Lines
        let success_lines = [
            style("╔══════════════════════════════════════════════════════════════╗").green().bold().to_string(),
            box_center(&style("✅  PENGATURAN BERHASIL DISIMPAN").green().bold().to_string()),
            style("╠══════════════════════════════════════════════════════════════╣").green().to_string(),
            box_row(&format!("• Kecepatan (Speed)         : {}", style(format!("{} (1-10)", config.speed)).yellow().bold()), 2),
            box_row(&format!("• Kualitas (Quality)        : {}", style(format!("{} / 100", config.quality)).cyan().bold()), 2),
            box_row(&format!("• Format Chroma (YUV)       : {}", style(format!("YUV {}", config.yuv)).magenta().bold()), 2),
            box_row(&format!("• File Konfigurasi          : {}", style("~/.config/gasinavif/config.json").dim()), 2),
            style("╚══════════════════════════════════════════════════════════════╝").green().bold().to_string(),
        ];
        for (idx, line) in success_lines.iter().enumerate() {
            assert_eq!(
                measure_text_width(line),
                64,
                "Success box line {idx} width is {} != 64: {line}",
                measure_text_width(line)
            );
        }
    }
}
