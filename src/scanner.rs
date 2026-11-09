//! scanner.rs — Rekursif file traversal dan logika pairing RAW + JPG
//!
//! Memindai direktori target, mengelompokkan file berdasarkan nama dasar (stem),
//! dan menghasilkan antrean kerja terpisah untuk Tahap 1 (RAW→JPG) dan Tahap 2 (JPG→AVIF).

use std::collections::HashMap;
use std::path::PathBuf;

/// Ekstensi file RAW kamera yang didukung (lowercase).
pub const RAW_EXTENSIONS: &[&str] = &[
    "cr2", "cr3", "arw", "srf", "sr2", "nef", "nrw", "dng", "raf", "rw2", "orf", "pef",
];

/// Ekstensi file JPG yang didukung (lowercase).
pub const JPG_EXTENSIONS: &[&str] = &["jpg", "jpeg"];

/// Ekstensi file AVIF (lowercase).
pub const AVIF_EXTENSION: &str = "avif";

/// Representasi satu grup file berdasarkan stem yang sama dalam satu direktori.
#[derive(Debug, Default)]
pub struct FileGroup {
    pub raw: Option<PathBuf>,
    pub jpg: Option<PathBuf>,
    pub avif: Option<PathBuf>,
}

/// Hasil pemindaian: antrean kerja untuk pipeline dua tahap.
#[derive(Debug, Default)]
pub struct ScanResult {
    /// Tahap 1: File RAW yang perlu di-decode menjadi JPG (karena tidak ada JPG pasangannya).
    pub stage1_raw_to_jpg: Vec<PathBuf>,

    /// Tahap 2: File JPG yang perlu dikompresi menjadi AVIF.
    pub stage2_jpg_to_avif: Vec<PathBuf>,

    /// File RAW yang bisa dihapus (karena JPG kamera sudah ada).
    pub raw_to_delete: Vec<PathBuf>,

    /// File yang dilewati (sudah ada AVIF-nya atau sudah ditandai).
    pub skipped: Vec<PathBuf>,

    /// Total ukuran awal semua file yang dipindai (bytes).
    pub total_original_size: u64,
}

/// Memindai direktori secara rekursif dan menghasilkan antrean kerja.
pub fn scan_directory(dir: &std::path::Path, _verbose: bool) -> anyhow::Result<ScanResult> {
    use walkdir::WalkDir;

    // Kunci: (parent_dir, stem) → FileGroup
    let mut groups: HashMap<(PathBuf, String), FileGroup> = HashMap::new();
    let mut total_size: u64 = 0;

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| {
            // Abaikan direktori state internal
            !e.file_name().to_string_lossy().starts_with(".gasinavif")
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path().to_path_buf();

        // Hitung ukuran
        if let Ok(meta) = std::fs::metadata(&path) {
            total_size += meta.len();
        }

        let ext = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => e.to_lowercase(),
            None => continue,
        };

        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        let parent = path.parent().unwrap_or(dir).to_path_buf();
        let key = (parent, stem);

        let group = groups.entry(key).or_default();

        if RAW_EXTENSIONS.contains(&ext.as_str()) {
            group.raw = Some(path);
        } else if JPG_EXTENSIONS.contains(&ext.as_str()) {
            group.jpg = Some(path);
        } else if ext == AVIF_EXTENSION {
            group.avif = Some(path);
        }
    }

    // Klasifikasikan setiap grup menjadi aksi
    let mut result = ScanResult {
        total_original_size: total_size,
        ..Default::default()
    };

    for (_key, group) in groups {
        // Jika sudah ada AVIF, skip semua
        if group.avif.is_some() {
            if let Some(ref jpg) = group.jpg {
                result.skipped.push(jpg.clone());
            }
            if let Some(ref raw) = group.raw {
                result.skipped.push(raw.clone());
            }
            continue;
        }

        match (group.raw, group.jpg) {
            // Kondisi 1: Ada RAW + JPG → hapus RAW, antrean JPG ke AVIF
            (Some(raw), Some(jpg)) => {
                result.raw_to_delete.push(raw);
                result.stage2_jpg_to_avif.push(jpg);
            }
            // Kondisi 2: Hanya RAW → antrean RAW ke JPG (lalu JPG ke AVIF nanti)
            (Some(raw), None) => {
                result.stage1_raw_to_jpg.push(raw);
            }
            // Kondisi 3: Hanya JPG → antrean JPG ke AVIF
            (None, Some(jpg)) => {
                result.stage2_jpg_to_avif.push(jpg);
            }
            // Tidak ada RAW atau JPG (mungkin file lain)
            (None, None) => {}
        }
    }

    Ok(result)
}

/// Menghapus file sementara (*.tmp) yang mungkin tertinggal dari sesi crash / interupsi.
pub fn clean_temp_files(dir: &std::path::Path) -> anyhow::Result<usize> {
    use walkdir::WalkDir;
    let mut count = 0;

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext.eq_ignore_ascii_case("tmp") {
                    if std::fs::remove_file(path).is_ok() {
                        count += 1;
                    }
                }
            }
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_temp_files() {
        let temp_dir = std::env::temp_dir().join("gasinavif_test_cleanup");
        let _ = std::fs::create_dir_all(&temp_dir);

        let tmp1 = temp_dir.join("photo1.avif.tmp");
        let tmp2 = temp_dir.join("photo2.jpg.tmp");
        let normal = temp_dir.join("normal.jpg");

        std::fs::write(&tmp1, b"temp").unwrap();
        std::fs::write(&tmp2, b"temp").unwrap();
        std::fs::write(&normal, b"normal").unwrap();

        let removed = clean_temp_files(&temp_dir).unwrap();
        assert_eq!(removed, 2);
        assert!(!tmp1.exists());
        assert!(!tmp2.exists());
        assert!(normal.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
