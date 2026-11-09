//! processor.rs — Core processing engine: RAW→JPG (Tahap 1) dan JPG→AVIF (Tahap 2).
//!
//! Menangani konversi gambar performa tinggi, proteksi ukuran, dan integrasi metadata.

use anyhow::{Context, Result};
use std::path::Path;

use crate::metadata;

/// Hasil pemrosesan satu file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessResult {
    /// Berhasil dikonversi, ukuran berkurang sebanyak `saved` bytes.
    Success { saved: u64 },
    /// Dilewati karena sudah ditandai sebelumnya.
    SkippedAlreadyTagged,
    /// Dibatalkan karena hasil konversi lebih besar/sama dari aslinya.
    SkippedLarger,
    /// Gagal dengan pesan error.
    Failed(String),
}

/// Tahap 2: Konversi satu file JPG menjadi AVIF.
///
/// Menggunakan `ravif` untuk encoding AV1 multi-threaded murni di memori,
/// menyalin metadata EXIF, dan memastikan ukuran AVIF < JPG asli.
pub fn process_jpg_to_avif(
    jpg_path: &Path,
    quality: u8,
    speed: u8,
) -> Result<ProcessResult> {
    // 1. Cek metadata tag di awal (anti-loop)
    if metadata::is_already_tagged(jpg_path)? {
        return Ok(ProcessResult::SkippedAlreadyTagged);
    }

    // Pastikan file JPG sudah ditandai sebelum EXIF-nya disalin ke AVIF
    let _ = metadata::tag_file(jpg_path, None, true);

    let orig_size = std::fs::metadata(jpg_path)
        .with_context(|| format!("Gagal membaca metadata JPG: {}", jpg_path.display()))?
        .len();

    // 2. Decode JPG ke image buffer
    let dyn_img = image::open(jpg_path)
        .with_context(|| format!("Gagal decode JPEG: {}", jpg_path.display()))?;
    let rgba = dyn_img.to_rgba8();
    let (width, height) = rgba.dimensions();

    let pixels: Vec<ravif::RGBA8> = rgba
        .chunks_exact(4)
        .map(|c| ravif::RGBA8::new(c[0], c[1], c[2], c[3]))
        .collect();

    let img_view = ravif::Img::new(&pixels[..], width as usize, height as usize);

    // 3. Setup ravif encoder
    let mut encoder = ravif::Encoder::new()
        .with_quality(quality as f32)
        .with_speed(speed);

    // Sisipkan raw EXIF jika tersedia
    let exif_data = metadata::extract_raw_exif_bytes(jpg_path);
    if let Some(ref exif) = exif_data {
        encoder = encoder.with_exif(exif.as_slice());
    }

    // 4. Encode ke AVIF
    let encoded = encoder
        .encode_rgba(img_view)
        .with_context(|| format!("Gagal encode AVIF: {}", jpg_path.display()))?;

    // 5. Tulis ke file temporary: *.avif.tmp
    let dir = jpg_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = jpg_path.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
    let tmp_avif = dir.join(format!("{stem}.avif.tmp"));
    let final_avif = dir.join(format!("{stem}.avif"));

    std::fs::write(&tmp_avif, &encoded.avif_file)
        .with_context(|| format!("Gagal menulis temporary AVIF: {}", tmp_avif.display()))?;

    let new_size = std::fs::metadata(&tmp_avif)
        .with_context(|| format!("Gagal membaca metadata tmp AVIF: {}", tmp_avif.display()))?
        .len();

    // 6. Validasi perbandingan ukuran
    if new_size < orig_size {
        // Berhasil hemat ukuran!
        let _ = metadata::preserve_mtime(jpg_path, &tmp_avif);

        std::fs::remove_file(jpg_path)
            .with_context(|| format!("Gagal menghapus JPG asli: {}", jpg_path.display()))?;

        std::fs::rename(&tmp_avif, &final_avif)
            .with_context(|| format!("Gagal rename {} -> {}", tmp_avif.display(), final_avif.display()))?;

        let saved = orig_size - new_size;
        Ok(ProcessResult::Success { saved })
    } else {
        // Gagal menghemat ukuran (AVIF >= JPG asli)
        let _ = std::fs::remove_file(&tmp_avif);

        // Beri tag STATUS=SKIPPED_LARGER ke JPG asli agar tidak diproses ulang di masa depan
        let _ = metadata::tag_file(jpg_path, Some(metadata::STATUS_SKIPPED_LARGER), true);

        Ok(ProcessResult::SkippedLarger)
    }
}

/// Tahap 1: Konversi satu file RAW menjadi JPG.
///
/// Mencoba mengekstrak preview full-size yang tertanam di RAW via rexiv2 terlebih dahulu.
/// Jika tidak ada atau resolusi terlalu kecil, memanggil `magick` sebagai fallback.
pub fn process_raw_to_jpg(
    raw_path: &Path,
    quality: u8,
) -> Result<ProcessResult> {
    // 1. Cek metadata tag di awal
    if metadata::is_already_tagged(raw_path)? {
        return Ok(ProcessResult::SkippedAlreadyTagged);
    }

    let orig_size = std::fs::metadata(raw_path)
        .with_context(|| format!("Gagal membaca metadata RAW: {}", raw_path.display()))?
        .len();

    let dir = raw_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = raw_path.file_stem().and_then(|s| s.to_str()).unwrap_or("raw");
    let tmp_jpg = dir.join(format!("{stem}.jpg.tmp"));
    let final_jpg = dir.join(format!("{stem}.jpg"));

    // Jika final_jpg sudah ada (kasus file JPG sudah ada sebelumnya), langsung hapus RAW
    if final_jpg.exists() {
        std::fs::remove_file(raw_path)
            .with_context(|| format!("Gagal menghapus RAW duplikat: {}", raw_path.display()))?;
        return Ok(ProcessResult::Success { saved: orig_size });
    }

    // 2. Decode RAW ke JPG:
    let mut rendered = false;

    // Coba ekstrak embedded preview beresolusi tinggi terlebih dahulu
    if let Ok(meta) = rexiv2::Metadata::new_from_path(raw_path) {
        if let Some(previews) = meta.get_preview_images() {
            if let Some(best_preview) = previews.into_iter().max_by_key(|p| p.get_size()) {
                // Gunakan jika ukurannya >= 500 KB dan lebar >= 1200 px (bukan thumbnail kecil)
                if best_preview.get_size() >= 500_000 && best_preview.get_width() >= 1200 {
                    if best_preview.save_to_file(&tmp_jpg).is_ok() {
                        rendered = true;
                    }
                }
            }
        }
    }

    // Jika tidak ada embedded preview yang memadai, gunakan ImageMagick `magick`
    if !rendered {
        let status = std::process::Command::new("magick")
            .arg(raw_path)
            .arg("-quality")
            .arg(quality.to_string())
            .arg(&tmp_jpg)
            .status();

        match status {
            Ok(s) if s.success() && tmp_jpg.exists() => {
                rendered = true;
            }
            _ => {
                let _ = std::fs::remove_file(&tmp_jpg);
                return Ok(ProcessResult::Failed(format!(
                    "Gagal render RAW via magick: {}",
                    raw_path.display()
                )));
            }
        }
    }

    if !rendered || !tmp_jpg.exists() {
        let _ = std::fs::remove_file(&tmp_jpg);
        return Ok(ProcessResult::Failed(format!(
            "Gagal menghasilkan output JPG untuk: {}",
            raw_path.display()
        )));
    }

    // 3. Salin EXIF dari RAW ke tmp_jpg
    let _ = metadata::copy_and_tag_metadata(raw_path, &tmp_jpg, None);

    let new_size = std::fs::metadata(&tmp_jpg)
        .with_context(|| format!("Gagal membaca ukuran tmp JPG: {}", tmp_jpg.display()))?
        .len();

    // 4. Bandingkan ukuran
    if new_size < orig_size {
        let _ = metadata::preserve_mtime(raw_path, &tmp_jpg);
        std::fs::remove_file(raw_path)
            .with_context(|| format!("Gagal menghapus RAW: {}", raw_path.display()))?;
        std::fs::rename(&tmp_jpg, &final_jpg)
            .with_context(|| format!("Gagal rename {} -> {}", tmp_jpg.display(), final_jpg.display()))?;

        let saved = orig_size - new_size;
        Ok(ProcessResult::Success { saved })
    } else {
        let _ = std::fs::remove_file(&tmp_jpg);
        let _ = metadata::tag_file(raw_path, Some(metadata::STATUS_SKIPPED_LARGER), true);
        Ok(ProcessResult::SkippedLarger)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    #[test]
    fn test_jpg_to_avif_compression() {
        let temp_dir = std::env::temp_dir();
        let test_jpg = temp_dir.join("test_convert_avif.jpg");
        let test_avif = temp_dir.join("test_convert_avif.avif");

        // Buat file JPEG gradasi warna (cukup besar agar AVIF lebih hemat)
        let mut img = RgbImage::new(100, 100);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgb([(x * 2) as u8, (y * 2) as u8, ((x + y)) as u8]);
        }
        img.save(&test_jpg).expect("Gagal membuat test JPEG");

        let res = process_jpg_to_avif(&test_jpg, 50, 9).expect("Gagal memproses JPG ke AVIF");

        match res {
            ProcessResult::Success { saved } => {
                assert!(saved > 0);
                assert!(test_avif.exists());
                assert!(!test_jpg.exists()); // JPG asli dihapus jika sukses

                // AVIF yang dihasilkan harus sudah ditandai
                assert!(metadata::is_already_tagged(&test_avif).unwrap());
            }
            ProcessResult::SkippedLarger => {
                // Jika tidak lebih kecil, JPG asli harus tetap ada
                assert!(test_jpg.exists());
            }
            _ => panic!("Hasil tidak terduga: {:?}", res),
        }

        let _ = std::fs::remove_file(&test_jpg);
        let _ = std::fs::remove_file(&test_avif);
    }

    #[test]
    fn test_raw_with_existing_jpg() {
        let temp_dir = std::env::temp_dir();
        let test_raw = temp_dir.join("test_dual_camera.cr2");
        let test_jpg = temp_dir.join("test_dual_camera.jpg");

        // Simulasikan file RAW dan file JPG kamera yang sudah ada
        std::fs::write(&test_raw, b"dummy raw data 1234567890").unwrap();
        std::fs::write(&test_jpg, b"dummy jpg data").unwrap();

        let raw_len = test_raw.metadata().unwrap().len();

        let res = process_raw_to_jpg(&test_raw, 95).expect("Gagal memproses RAW");

        assert_eq!(res, ProcessResult::Success { saved: raw_len });
        assert!(!test_raw.exists()); // RAW dihapus
        assert!(test_jpg.exists());  // JPG dipertahankan

        let _ = std::fs::remove_file(&test_jpg);
    }
}
