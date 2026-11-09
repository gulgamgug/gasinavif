//! settings.rs — Antarmuka TUI interaktif untuk konfigurasi global gasinavif (--settings).
//!
//! Menavigasi menu dan submenu menggunakan tombol panah keyboard:
//!   - Slider kecepatan (Speed: 1 - 10) dengan navigasi kiri/kanan.
//!   - Pengatur kualitas (Quality: 1 - 100) dengan navigasi atas/bawah (step 1).
//!   - Pemilih format chroma subsampling (YUV: 420, 422, 444, 400).
//!   - Auto-save otomatis ke disk (~/.config/gasinavif/config.json) saat keluar/berpindah.

use console::style;
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
    println!("{}", style("║              ✅  PENGATURAN BERHASIL DISIMPAN                ║").green().bold());
    println!("{}", style("╠══════════════════════════════════════════════════════════════╣").green());
    println!("║  • Kecepatan (Speed)         : {:<29} ║", style(format!("{} (1-10)", config.speed)).yellow().bold());
    println!("║  • Kualitas (Quality)        : {:<29} ║", style(format!("{} / 100", config.quality)).cyan().bold());
    println!("║  • Format Chroma (YUV)       : {:<29} ║", style(format!("YUV {}", config.yuv)).magenta().bold());
    println!("║  • Lokasi File Config        : {:<29} ║", style("~/.config/gasinavif/config.json").dim());
    println!("{}", style("╚══════════════════════════════════════════════════════════════╝").green().bold());
    println!();

    Ok(())
}

fn render_main_menu(config: &AppConfig, selected: usize, lines: &mut Vec<String>) {
    lines.push(style("╔══════════════════════════════════════════════════════════════╗").cyan().bold().to_string());
    lines.push(style("║                ⚙️   PENGATURAN GLOBAL GASINAVIF               ║").bold().white().to_string());
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push("║                                                              ║".to_string());

    let items = [
        format!("1. Kecepatan Encoder (Speed)  : [ {:>2} ]", config.speed),
        format!("2. Kualitas Gambar (Quality)  : [ {:>3} ]", config.quality),
        format!("3. Format Chroma Subsampling  : [ YUV {} ]", config.yuv),
        "4. Simpan & Keluar (Save & Exit)".to_string(),
    ];

    for (i, item) in items.iter().enumerate() {
        if i == selected {
            let row = format!("▶  {:<54}", item);
            lines.push(format!("║  {}  ║", style(row).green().bold().underlined()));
        } else {
            let row = format!("   {:<54}", item);
            lines.push(format!("║  {}  ║", style(row).white()));
        }
    }

    lines.push("║                                                              ║".to_string());
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push("║  [↑/↓] Pilih Menu  |  [Enter] Masuk  |  [Esc/q] Simpan & Out ║".to_string());
    lines.push(style("╚══════════════════════════════════════════════════════════════╝").cyan().bold().to_string());
}

fn render_speed_menu(config: &AppConfig, lines: &mut Vec<String>) {
    lines.push(style("╔══════════════════════════════════════════════════════════════╗").cyan().bold().to_string());
    lines.push(style("║             ⚡  PENGATURAN KECEPATAN (SPEED: 1 - 10)          ║").bold().white().to_string());
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push("║                                                              ║".to_string());

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
    lines.push(format!("║   Kecepatan Aktif: {:>2} / 10                                 ║", style(config.speed).yellow().bold()));
    lines.push("║                                                              ║".to_string());
    lines.push(format!("║   Slider: {:<50} ║", slider_str));
    lines.push("║                                                              ║".to_string());
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push("║  Penjelasan Kecepatan yang Dipilih:                          ║".to_string());

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
            "Keseimbangan ideal antara kecepatan dan penghematan ukuran.",
        ),
        _ => (
            "⚡ Speed 8 - 10: Sangat Cepat (Rekomendasi Default)",
            "Encoding instan (~0.5s/foto) dengan kompresi hemat ~70-85%.",
        ),
    };

    lines.push(format!("║  {:<60}║", style(desc1).yellow().bold()));
    lines.push(format!("║  {:<60}║", style(desc2).dim()));
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push("║  [←/→] Geser Slider  |  [Enter/Esc] Kembali & Simpan Otomatis║".to_string());
    lines.push(style("╚══════════════════════════════════════════════════════════════╝").cyan().bold().to_string());
}

fn render_quality_menu(config: &AppConfig, lines: &mut Vec<String>) {
    lines.push(style("╔══════════════════════════════════════════════════════════════╗").cyan().bold().to_string());
    lines.push(style("║            🎯  PENGATURAN KUALITAS (QUALITY: 1 - 100)        ║").bold().white().to_string());
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push("║                                                              ║".to_string());

    let bar_len = 36;
    let filled = ((config.quality as usize) * bar_len) / 100;
    let empty = bar_len.saturating_sub(filled);
    let bar_visual = format!("{}{}", style("█".repeat(filled)).cyan(), style("░".repeat(empty)).dim());

    lines.push(format!("║   Kualitas Aktif: {:>3} / 100                                ║", style(config.quality).yellow().bold()));
    lines.push("║                                                              ║".to_string());
    lines.push(format!("║   Bar: [{}] {:>3}%             ║", bar_visual, config.quality));
    lines.push("║                                                              ║".to_string());
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push("║  Penjelasan Rentang Kualitas:                                ║".to_string());

    let (desc1, desc2) = match config.quality {
        1..=20 => (
            "🎯 Rentang 1 - 20 (Sangat Rendah / Kompresi Ekstrem)",
            "Ukuran sangat kecil, artefak kompresi terlihat jelas.",
        ),
        21..=40 => (
            "🎯 Rentang 21 - 40 (Rendah / Hemat Ukuran)",
            "Ukuran file hemat, cocok untuk thumbnail web / transmisi cepat.",
        ),
        41..=60 => (
            "🎯 Rentang 41 - 60 (Seimbang / Rekomendasi Default)",
            "Keseimbangan emas antara efisiensi hemat dan ketajaman alami.",
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

    lines.push(format!("║  {:<60}║", style(desc1).yellow().bold()));
    lines.push(format!("║  {:<60}║", style(desc2).dim()));
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push("║  [↑/↓] Atur Step 1  |  [PgUp/PgDn] Step 10  |  [Esc] Kembali ║".to_string());
    lines.push(style("╚══════════════════════════════════════════════════════════════╝").cyan().bold().to_string());
}

fn render_yuv_menu(config: &AppConfig, selected: usize, lines: &mut Vec<String>) {
    lines.push(style("╔══════════════════════════════════════════════════════════════╗").cyan().bold().to_string());
    lines.push(style("║          📺  PENGATURAN FORMAT CHROMA (YUV FORMAT)           ║").bold().white().to_string());
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push("║                                                              ║".to_string());

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

        if i == selected {
            let row = format!("▶  {:<43}{}", label, active_badge);
            lines.push(format!("║  {}  ║", style(row).green().bold().underlined()));
        } else {
            let row = format!("   {:<43}{}", label, active_badge);
            lines.push(format!("║  {}  ║", style(row).white()));
        }
    }

    lines.push("║                                                              ║".to_string());
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push("║  Penjelasan Format Chroma yang Dipilih:                      ║".to_string());

    let (desc1, desc2) = match selected {
        0 => (
            "📺 YUV 4:2:0 (Standar Industri & Rekomendasi)",
            "Subsampling 2x2. Kompatibilitas universal & kompresi paling hemat.",
        ),
        1 => (
            "📺 YUV 4:2:2 (Subsampling Horizontal)",
            "Resolusi warna separuh horizontal. Bagus untuk tepi teks tajam.",
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

    lines.push(format!("║  {:<60}║", style(desc1).yellow().bold()));
    lines.push(format!("║  {:<60}║", style(desc2).dim()));
    lines.push(style("╠══════════════════════════════════════════════════════════════╣").cyan().to_string());
    lines.push("║  [↑/↓] Pilih  |  [Enter] Terapkan  |  [Esc] Kembali & Simpan ║".to_string());
    lines.push(style("╚══════════════════════════════════════════════════════════════╝").cyan().bold().to_string());
}
