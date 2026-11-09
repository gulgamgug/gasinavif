//! processor.rs — Core processing engine: RAW→JPG (Tahap 1) dan JPG→AVIF (Tahap 2).
//!
//! Placeholder — implementasi penuh akan diisi di Fase 5.

use anyhow::Result;
use std::path::Path;

/// Hasil pemrosesan satu file.
#[derive(Debug, Clone)]
pub enum ProcessResult {
    /// Berhasil dikonversi, ukuran berkurang sebanyak `saved` bytes.
    Success { saved: u64 },
    /// Dilewati karena sudah ditandai sebelumnya.
    SkippedAlreadyTagged,
    /// Dibatalkan karena hasil konversi lebih besar dari aslinya.
    SkippedLarger,
    /// Gagal dengan pesan error.
    Failed(String),
}

/// Tahap 1: Konversi satu file RAW menjadi JPG.
pub fn process_raw_to_jpg(_raw_path: &Path, _quality: u8) -> Result<ProcessResult> {
    // TODO: Fase 5 — implementasi decode RAW + encode JPG + salin EXIF
    Ok(ProcessResult::Failed("Belum diimplementasi".to_string()))
}

/// Tahap 2: Konversi satu file JPG menjadi AVIF.
pub fn process_jpg_to_avif(
    _jpg_path: &Path,
    _quality: u8,
    _speed: u8,
) -> Result<ProcessResult> {
    // TODO: Fase 5 — implementasi encode AVIF + salin EXIF + validasi ukuran
    Ok(ProcessResult::Failed("Belum diimplementasi".to_string()))
}
