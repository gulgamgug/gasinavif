//! interactive.rs — Sistem kontrol interaktif keyboard on-the-fly dan pemantauan thread dinamis.
//!
//! Menangani hotkey live:
//!   - `s`: Toggle tampilan worker active per core/thread
//!   - `v`: Toggle mode verbose
//!   - `t`: Pengaturan thread dinamis (bisa juga via `+` dan `-`)
//!   - `l`: Toggle tampilan folder tree progress
//!   - `q`: Keluar dengan aman (graceful stop)

use console::style;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::collections::VecDeque;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::tree::FolderNode;
use crate::ui;

/// Frame animasi spinner Unicode
pub const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Guard RAII untuk memastikan terminal raw mode selalu dikembalikan ke normal saat keluar.
pub struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// State global untuk interaktivitas terminal
pub struct LiveController {
    pub show_workers: Arc<AtomicBool>,
    pub is_verbose: Arc<AtomicBool>,
    pub show_tree: Arc<AtomicBool>,
    pub target_threads: Arc<AtomicUsize>,
    pub is_interrupted: Arc<AtomicBool>,
    pub worker_slots: Arc<RwLock<Vec<Option<PathBuf>>>>,
    pub log_queue: Arc<Mutex<VecDeque<String>>>,
}

impl LiveController {
    pub fn new(initial_threads: usize, initial_verbose: bool) -> Self {
        let max_slots = 64.max(initial_threads * 2);
        Self {
            show_workers: Arc::new(AtomicBool::new(false)),
            is_verbose: Arc::new(AtomicBool::new(initial_verbose)),
            show_tree: Arc::new(AtomicBool::new(false)),
            target_threads: Arc::new(AtomicUsize::new(initial_threads.max(1))),
            is_interrupted: Arc::new(AtomicBool::new(false)),
            worker_slots: Arc::new(RwLock::new(vec![None; max_slots])),
            log_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Menambahkan pesan log verbose ke antrean buffer
    pub fn log(&self, msg: String) {
        let mut q = self.log_queue.lock().unwrap();
        q.push_back(msg);
    }

    /// Menjalankan background listener untuk menangkap penekanan tombol keyboard non-blocking.
    pub fn start_keyboard_listener(&self) -> Option<RawModeGuard> {
        if !std::io::stdin().is_terminal() {
            return None;
        }

        let _ = crossterm::terminal::enable_raw_mode();

        let show_workers = Arc::clone(&self.show_workers);
        let is_verbose = Arc::clone(&self.is_verbose);
        let show_tree = Arc::clone(&self.show_tree);
        let target_threads = Arc::clone(&self.target_threads);
        let is_interrupted = Arc::clone(&self.is_interrupted);

        std::thread::spawn(move || {
            while !is_interrupted.load(Ordering::Relaxed) {
                if let Ok(true) = event::poll(Duration::from_millis(50)) {
                    if let Ok(Event::Key(key_event)) = event::read() {
                        if key_event.kind != KeyEventKind::Press {
                            continue;
                        }

                        // Tangani Ctrl+C
                        if key_event.modifiers.contains(KeyModifiers::CONTROL)
                            && key_event.code == KeyCode::Char('c')
                        {
                            is_interrupted.store(true, Ordering::SeqCst);
                            break;
                        }

                        match key_event.code {
                            // 's' : Toggle Show Workers per Core
                            KeyCode::Char('s') | KeyCode::Char('S') => {
                                let cur = show_workers.load(Ordering::Relaxed);
                                show_workers.store(!cur, Ordering::Relaxed);
                            }
                            // 'v' : Toggle Verbose Mode
                            KeyCode::Char('v') | KeyCode::Char('V') => {
                                let cur = is_verbose.load(Ordering::Relaxed);
                                is_verbose.store(!cur, Ordering::Relaxed);
                            }
                            // 'l' : Toggle Tree List View
                            KeyCode::Char('l') | KeyCode::Char('L') => {
                                let cur = show_tree.load(Ordering::Relaxed);
                                show_tree.store(!cur, Ordering::Relaxed);
                            }
                            // 't' : Cycle jumlah threads
                            KeyCode::Char('t') | KeyCode::Char('T') => {
                                let cur = target_threads.load(Ordering::Relaxed);
                                let next = match cur {
                                    1..=3 => 4,
                                    4..=7 => 8,
                                    8..=11 => 12,
                                    12..=15 => 16,
                                    16..=23 => 24,
                                    24..=31 => 32,
                                    _ => 2,
                                };
                                target_threads.store(next, Ordering::Relaxed);
                            }
                            // '+' : Tambah 1 thread
                            KeyCode::Char('+') | KeyCode::Char('=') => {
                                let cur = target_threads.load(Ordering::Relaxed);
                                target_threads.store((cur + 1).min(64), Ordering::Relaxed);
                            }
                            // '-' : Kurangi 1 thread
                            KeyCode::Char('-') | KeyCode::Char('_') => {
                                let cur = target_threads.load(Ordering::Relaxed);
                                target_threads.store((cur.saturating_sub(1)).max(1), Ordering::Relaxed);
                            }
                            // 'q' : Graceful Quit
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                is_interrupted.store(true, Ordering::SeqCst);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }

            let _ = crossterm::terminal::disable_raw_mode();
        });

        Some(RawModeGuard)
    }

    /// Menghitung kecepatan saat ini (foto/detik) dan estimasi sisa waktu (ETA).
    pub fn calculate_speed_and_eta(
        start_time: Instant,
        processed: usize,
        total: usize,
    ) -> (f64, String) {
        let elapsed_secs = start_time.elapsed().as_secs_f64();
        let speed = if elapsed_secs > 0.05 {
            processed as f64 / elapsed_secs
        } else {
            0.0
        };

        let remaining = total.saturating_sub(processed);
        let eta_secs = if speed > 0.01 {
            (remaining as f64 / speed) as u64
        } else {
            0
        };

        let eta_formatted = ui::format_duration(Duration::from_secs(eta_secs));
        (speed, eta_formatted)
    }

    /// Merender status baris worker (mode 's')
    pub fn render_worker_lines(&self) -> Vec<String> {
        let slots = self.worker_slots.read().unwrap();
        let limit = self.target_threads.load(Ordering::Relaxed);
        let mut lines = Vec::new();

        for i in 0..limit {
            let label = style(format!("[Core {:>2}]", i)).cyan().bold();
            match slots.get(i).and_then(|s| s.as_ref()) {
                Some(path) => {
                    lines.push(format!("    {} {}", label, path.display()));
                }
                None => {
                    lines.push(format!("    {} {}", label, style("(idle)").dim()));
                }
            }
        }

        lines
    }

    /// Baris navigasi panduan shortcut interaktif
    pub fn render_control_hint(&self) -> String {
        let s_on = self.show_workers.load(Ordering::Relaxed);
        let v_on = self.is_verbose.load(Ordering::Relaxed);
        let l_on = self.show_tree.load(Ordering::Relaxed);
        let threads = self.target_threads.load(Ordering::Relaxed);

        let s_label = if s_on { style("s:show (ON)").green().bold() } else { style("s:show (OFF)").dim() };
        let l_label = if l_on { style("l:tree (ON)").green().bold() } else { style("l:tree (OFF)").dim() };
        let v_label = if v_on { style("v:verbose (ON)").green().bold() } else { style("v:verbose (OFF)").dim() };
        let t_label = style(format!("t:threads ({threads})")).yellow().bold();
        let q_label = style("q:quit").red().bold();

        format!("    [ {} | {} | {} | {} | {} ]", s_label, l_label, v_label, t_label, q_label)
    }
}

/// Merender satu baris progress bar dengan styling modern
fn render_bar_line(
    stage_label: &str,
    spinner_char: char,
    elapsed: Duration,
    processed: usize,
    total: usize,
    speed: f64,
    eta: &str,
) -> String {
    let elapsed_str = ui::format_duration(elapsed);
    let pct = if total > 0 {
        (processed as f64 / total as f64) * 100.0
    } else {
        100.0
    };

    let bar_width = 30;
    let filled = if total > 0 {
        ((processed as f64 / total as f64) * bar_width as f64).round() as usize
    } else {
        bar_width
    };
    let filled = filled.min(bar_width);
    let unfilled = bar_width.saturating_sub(filled);

    let bar_str = format!(
        "{}{}",
        style("█".repeat(filled)).cyan(),
        style(" ".repeat(unfilled)).dim()
    );

    format!(
        "  {} {} [{}] [{}] {}/{} ({:.0}%) | {:.1} foto/s | ETA: {}",
        style(spinner_char).green().bold(),
        style(stage_label).bold(),
        elapsed_str,
        bar_str,
        processed,
        total,
        pct,
        speed,
        eta,
    )
}

/// Dynamic worker dispatcher: menjalankan fungsi `on_complete` pada sekumpulan file
/// dengan jumlah worker thread yang dapat diatur secara dinamis (`t`).
pub fn execute_with_dynamic_workers<F>(
    files: Vec<PathBuf>,
    controller: Arc<LiveController>,
    folder_tree: Arc<FolderNode>,
    stage_label: &str,
    on_complete: F,
) where
    F: Fn(&Path, &FolderNode) + Send + Sync + 'static,
{
    if files.is_empty() {
        return;
    }

    let total = files.len();
    let queue = Arc::new(Mutex::new(VecDeque::from(files)));
    let processed_counter = Arc::new(AtomicUsize::new(0));
    let on_complete = Arc::new(on_complete);
    let start_time = Instant::now();

    let max_pool_size = 64;
    let mut handles = Vec::new();

    for worker_id in 0..max_pool_size {
        let q = Arc::clone(&queue);
        let ctrl = Arc::clone(&controller);
        let tree = Arc::clone(&folder_tree);
        let on_comp = Arc::clone(&on_complete);
        let p_counter = Arc::clone(&processed_counter);

        let handle = std::thread::spawn(move || {
            while !ctrl.is_interrupted.load(Ordering::Relaxed) {
                let next_file = {
                    let mut locked_queue = q.lock().unwrap();
                    if locked_queue.is_empty() {
                        break;
                    }
                    let target = ctrl.target_threads.load(Ordering::Relaxed);
                    if worker_id >= target {
                        None
                    } else {
                        locked_queue.pop_front()
                    }
                };

                match next_file {
                    Some(file) => {
                        // Catat slot aktif worker
                        {
                            let mut slots = ctrl.worker_slots.write().unwrap();
                            if worker_id < slots.len() {
                                slots[worker_id] = Some(file.clone());
                            }
                        }

                        // Jalankan pemrosesan secara paralel tanpa lock
                        on_comp(&file, &tree);

                        // Kosongkan slot worker
                        {
                            let mut slots = ctrl.worker_slots.write().unwrap();
                            if worker_id < slots.len() {
                                slots[worker_id] = None;
                            }
                        }

                        p_counter.fetch_add(1, Ordering::Relaxed);
                    }
                    None => {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                }
            }
        });

        handles.push(handle);
    }

    let is_tty = std::io::stdout().is_terminal();
    let mut prev_lines = 0;
    let mut spinner_idx = 0;

    while processed_counter.load(Ordering::Relaxed) < total
        && !controller.is_interrupted.load(Ordering::Relaxed)
    {
        let processed = processed_counter.load(Ordering::Relaxed);
        let (speed, eta) = LiveController::calculate_speed_and_eta(start_time, processed, total);
        let spinner_char = SPINNER_FRAMES[spinner_idx % SPINNER_FRAMES.len()];
        spinner_idx += 1;

        let pending_logs: Vec<String> = {
            let mut q = controller.log_queue.lock().unwrap();
            q.drain(..).collect()
        };

        if is_tty {
            // Bersihkan frame sebelumnya jika ada
            if prev_lines > 0 {
                print!("\x1B[{}A\x1B[0J", prev_lines);
            }

            // Cetak log verbose yang masuk
            for msg in pending_logs {
                println!("{}", msg);
            }

            // Bangun baris-baris frame aktif
            let mut active_lines = Vec::new();

            // Mode 'l': Hierarki folder tree
            if controller.show_tree.load(Ordering::Relaxed) {
                active_lines.push(format!("  {}", style("── 📁 Hierarki Folder & Progres ──").bold()));
                for tl in folder_tree.render(spinner_char) {
                    active_lines.push(format!("  {}", tl));
                }
            }

            // Mode 's': Status pekerja per core
            if controller.show_workers.load(Ordering::Relaxed) {
                let cur_threads = controller.target_threads.load(Ordering::Relaxed);
                active_lines.push(format!("  {}", style(format!("── ⚡ Status Pekerja (Active Threads: {}) ──", cur_threads)).bold()));
                for wl in controller.render_worker_lines() {
                    active_lines.push(wl);
                }
            }

            // Baris progress bar
            let bar_line = render_bar_line(
                stage_label,
                spinner_char,
                start_time.elapsed(),
                processed,
                total,
                speed,
                &eta,
            );
            active_lines.push(bar_line);

            // Baris petunjuk tombol interaktif
            active_lines.push(controller.render_control_hint());

            // Batasi baris agar tidak overflow dari terminal
            let (_cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
            let max_lines = (rows.saturating_sub(2) as usize).max(3);
            if active_lines.len() > max_lines {
                let excess = active_lines.len() - max_lines + 1;
                active_lines.truncate(max_lines - 1);
                active_lines.push(format!("    {}", style(format!("... (+ {} baris lainnya)", excess)).dim()));
            }

            prev_lines = active_lines.len();
            for line in &active_lines {
                println!("{}", line);
            }
            let _ = std::io::stdout().flush();
        } else {
            // Fallback non-TTY (pipe atau headless)
            for msg in pending_logs {
                println!("{}", msg);
            }
            if spinner_idx % 10 == 0 {
                println!(
                    "  {} {}/{} ({:.0}%) | {:.1} foto/s | ETA: {}",
                    stage_label,
                    processed,
                    total,
                    if total > 0 { (processed as f64 / total as f64) * 100.0 } else { 100.0 },
                    speed,
                    eta
                );
            }
        }

        std::thread::sleep(Duration::from_millis(80));
    }

    // Tunggu semua thread pekerja selesai
    for h in handles {
        let _ = h.join();
    }

    if is_tty && prev_lines > 0 {
        print!("\x1B[{}A\x1B[0J", prev_lines);
        let _ = std::io::stdout().flush();
    }

    // Drain sisa log yang belum tercetak
    let remaining_logs: Vec<String> = {
        let mut q = controller.log_queue.lock().unwrap();
        q.drain(..).collect()
    };
    for msg in remaining_logs {
        println!("{}", msg);
    }

    let final_processed = processed_counter.load(Ordering::Relaxed);
    let elapsed = ui::format_duration(start_time.elapsed());
    println!(
        "  {} {} [{}] {} file selesai.",
        style("✔").green().bold(),
        style(stage_label).bold(),
        elapsed,
        style(final_processed).green().bold(),
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speed_and_eta() {
        let start = Instant::now() - Duration::from_secs(10);
        let (speed, eta) = LiveController::calculate_speed_and_eta(start, 50, 100);
        assert!(speed > 4.0 && speed < 6.0);
        assert!(!eta.is_empty());
    }

    #[test]
    fn test_live_controller_defaults_and_render() {
        let ctrl = LiveController::new(4, false);
        assert!(!ctrl.show_workers.load(Ordering::Relaxed));
        assert!(!ctrl.is_verbose.load(Ordering::Relaxed));
        assert!(!ctrl.show_tree.load(Ordering::Relaxed));
        assert_eq!(ctrl.target_threads.load(Ordering::Relaxed), 4);

        let lines = ctrl.render_worker_lines();
        assert_eq!(lines.len(), 4);
        assert!(lines[0].contains("(idle)"));

        let hint = ctrl.render_control_hint();
        assert!(hint.contains("threads (4)"));
    }

    #[test]
    fn test_bar_line_rendering() {
        let bar = render_bar_line("TEST", '⠋', Duration::from_secs(5), 10, 20, 2.0, "00:05");
        assert!(bar.contains("TEST"));
        assert!(bar.contains("10/20"));
        assert!(bar.contains("50%"));
        assert!(bar.contains("2.0 foto/s"));
        assert!(bar.contains("ETA: 00:05"));
    }
}
