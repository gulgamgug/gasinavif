//! state.rs — State management, checkpoint, dan resume.
//!
//! Menyimpan progress pemrosesan ke `.gasinavif_state.json` agar bisa dilanjutkan
//! jika proses dihentikan di tengah jalan.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const STATE_DIR: &str = ".gasinavif_state";
const STATE_FILE: &str = "progress.json";

/// Status pemrosesan yang disimpan ke disk.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AppState {
    /// Set path file yang sudah berhasil diproses.
    pub completed: HashSet<PathBuf>,

    /// Set path file yang di-skip (sudah ditandai atau ukuran lebih besar).
    pub skipped: HashSet<PathBuf>,

    /// Total ukuran awal folder saat pemindaian dimulai.
    pub original_total_size: u64,
}

impl AppState {
    /// Memuat state dari disk, atau mengembalikan state kosong jika belum ada.
    pub fn load(base_dir: &Path) -> Result<Self> {
        let state_path = base_dir.join(STATE_DIR).join(STATE_FILE);
        if !state_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&state_path)
            .with_context(|| format!("Gagal membaca state: {}", state_path.display()))?;
        let state: Self = serde_json::from_str(&content)
            .with_context(|| "Gagal parse state JSON")?;
        Ok(state)
    }

    /// Menyimpan state ke disk.
    pub fn save(&self, base_dir: &Path) -> Result<()> {
        let state_dir = base_dir.join(STATE_DIR);
        std::fs::create_dir_all(&state_dir)
            .with_context(|| format!("Gagal membuat direktori state: {}", state_dir.display()))?;

        let state_path = state_dir.join(STATE_FILE);
        let content = serde_json::to_string_pretty(self)
            .with_context(|| "Gagal serialize state")?;
        std::fs::write(&state_path, content)
            .with_context(|| format!("Gagal menulis state: {}", state_path.display()))?;
        Ok(())
    }

    /// Menghapus semua state (untuk flag --fresh).
    pub fn reset(base_dir: &Path) -> Result<()> {
        let state_dir = base_dir.join(STATE_DIR);
        if state_dir.exists() {
            std::fs::remove_dir_all(&state_dir)
                .with_context(|| format!("Gagal menghapus state dir: {}", state_dir.display()))?;
        }
        Ok(())
    }

    /// Mengecek apakah file sudah pernah diproses.
    pub fn is_done(&self, path: &Path) -> bool {
        self.completed.contains(path) || self.skipped.contains(path)
    }

    /// Menandai file sebagai selesai diproses.
    pub fn mark_completed(&mut self, path: PathBuf) {
        self.completed.insert(path);
    }

    /// Menandai file sebagai di-skip.
    pub fn mark_skipped(&mut self, path: PathBuf) {
        self.skipped.insert(path);
    }
}
