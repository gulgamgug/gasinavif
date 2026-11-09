//! config.rs — Manajemen konfigurasi global persisten gasinavif.
//!
//! Menyimpan konfigurasi default (quality, speed, yuv) di `~/.config/gasinavif/config.json`.
//! Pengaturan ini dapat diubah melalui antarmuka interaktif `gasinavif --settings`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Konfigurasi default aplikasi yang disimpan secara global
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    /// Kualitas kompresi AVIF (1 - 100)
    pub quality: u8,
    /// Kecepatan encoder AVIF (1 - 10, semakin tinggi semakin cepat)
    pub speed: u8,
    /// Format chroma subsampling ("420", "422", "444", "400")
    pub yuv: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            quality: 60,
            speed: 8,
            yuv: "420".to_string(),
        }
    }
}

impl AppConfig {
    /// Memuat konfigurasi dari disk, atau default jika belum ada/gagal
    pub fn load() -> Self {
        let path = get_config_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<AppConfig>(&content) {
                    return cfg.validated();
                }
            }
        }
        Self::default()
    }

    /// Menyimpan konfigurasi ke disk (membuat direktori jika belum ada)
    pub fn save(&self) -> Result<()> {
        let path = get_config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Gagal membuat direktori config: {}", parent.display()))?;
        }
        let content = serde_json::to_string_pretty(self)
            .with_context(|| "Gagal serialisasi config JSON")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Gagal menulis file config: {}", path.display()))?;
        Ok(())
    }

    /// Memvalidasi batasan nilai konfigurasi
    pub fn validated(mut self) -> Self {
        self.quality = self.quality.clamp(1, 100);
        self.speed = self.speed.clamp(1, 10);
        if !["420", "422", "444", "400"].contains(&self.yuv.as_str()) {
            self.yuv = "420".to_string();
        }
        self
    }
}

/// Menemukan path direktori konfigurasi pengguna (~/.config/gasinavif/config.json)
pub fn get_config_path() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("gasinavif").join("config.json");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config").join("gasinavif").join("config.json");
    }
    PathBuf::from(".gasinavif_config.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default_and_validation() {
        let mut cfg = AppConfig::default();
        assert_eq!(cfg.quality, 60);
        assert_eq!(cfg.speed, 8);
        assert_eq!(cfg.yuv, "420");

        cfg.quality = 150;
        cfg.speed = 0;
        cfg.yuv = "invalid".to_string();

        let valid = cfg.validated();
        assert_eq!(valid.quality, 100);
        assert_eq!(valid.speed, 1);
        assert_eq!(valid.yuv, "420");
    }

    #[test]
    fn test_config_save_and_load_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!("gasinavif_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let config_file = temp_dir.join("config.json");

        let cfg = AppConfig {
            quality: 75,
            speed: 6,
            yuv: "444".to_string(),
        };

        let json = serde_json::to_string_pretty(&cfg).unwrap();
        std::fs::write(&config_file, json).unwrap();

        let read_content = std::fs::read_to_string(&config_file).unwrap();
        let loaded: AppConfig = serde_json::from_str(&read_content).unwrap();

        assert_eq!(loaded.quality, 75);
        assert_eq!(loaded.speed, 6);
        assert_eq!(loaded.yuv, "444");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
