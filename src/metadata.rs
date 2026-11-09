//! metadata.rs — Pembacaan dan penulisan tag EXIF secara in-process via rexiv2 (libexiv2)
//!
//! Menggantikan pemanggilan `exiftool` eksternal dengan pembacaan native C++/Rust,
//! sehingga pengecekan metadata menjadi ~100-500x lebih cepat (sub-milidetik).

use anyhow::{Context, Result};
use chrono::Utc;
use img_parts::ImageEXIF;
use std::path::Path;

/// Prefix tag identitas yang ditanamkan di field UserComment / Description.
pub const TAG_PREFIX: &str = "COMPRESSED_BY_SCRIPT=true";
pub const STATUS_SKIPPED_LARGER: &str = "SKIPPED_LARGER";

/// Membuat payload metadata lengkap untuk ditanamkan.
pub fn build_tag_payload(status: Option<&str>) -> String {
    let date = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let host = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let mut tag = format!("{TAG_PREFIX};DATE={date};HOST={host}");
    if let Some(s) = status {
        tag.push_str(&format!(";STATUS={s}"));
    }
    tag
}

/// Mengecek apakah sebuah file sudah memiliki tag `COMPRESSED_BY_SCRIPT` di metadata EXIF/XMP.
///
/// Mengembalikan `true` jika file sudah pernah diproses atau ditandai batal.
pub fn is_already_tagged(path: &Path) -> Result<bool> {
    // 1. Coba baca via rexiv2 (mendukung JPG, AVIF, RAW, TIFF, PNG, dll)
    if let Ok(meta) = rexiv2::Metadata::new_from_path(path) {
        // Cek Exif UserComment
        if let Ok(comment) = meta.get_tag_string("Exif.Photo.UserComment") {
            if comment.contains(TAG_PREFIX) {
                return Ok(true);
            }
        }

        // Cek Exif ImageDescription (beberapa kamera/tool menyimpan komentar di sini)
        if let Ok(desc) = meta.get_tag_string("Exif.Image.ImageDescription") {
            if desc.contains(TAG_PREFIX) {
                return Ok(true);
            }
        }

        // Cek Xmp CreatorTool atau Xmp Description
        if let Ok(tool) = meta.get_tag_string("Xmp.xmp.CreatorTool") {
            if tool.contains("gasinavif") || tool.contains("fish-avif-compressor") {
                return Ok(true);
            }
        }
    }

    // 2. Fallback via kamadak-exif untuk file container standard
    if let Ok(file) = std::fs::File::open(path) {
        let mut buf_reader = std::io::BufReader::new(&file);
        let exif_reader = exif::Reader::new();
        if let Ok(exif) = exif_reader.read_from_container(&mut buf_reader) {
            if let Some(field) = exif.get_field(exif::Tag::UserComment, exif::In::PRIMARY) {
                let value = field.display_value().to_string();
                if value.contains(TAG_PREFIX) {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

/// Menyuntikkan tag identitas `COMPRESSED_BY_SCRIPT` ke dalam file gambar.
/// Jika `status` diisi (misal `STATUS_SKIPPED_LARGER`), tag batal ditambahkan.
pub fn tag_file(path: &Path, status: Option<&str>, keep_mtime: bool) -> Result<()> {
    let original_mtime = if keep_mtime {
        std::fs::metadata(path)
            .ok()
            .map(|m| filetime::FileTime::from_last_modification_time(&m))
    } else {
        None
    };

    let meta = rexiv2::Metadata::new_from_path(path)
        .with_context(|| format!("Gagal memuat metadata untuk penandaan: {}", path.display()))?;

    let payload = build_tag_payload(status);
    let _ = meta.set_tag_string("Exif.Photo.UserComment", &payload);

    let host = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let _ = meta.set_tag_string(
        "Xmp.xmp.CreatorTool",
        &format!("gasinavif-rust ({host})"),
    );

    meta.save_to_file(path)
        .with_context(|| format!("Gagal menyimpan tag metadata: {}", path.display()))?;

    if let Some(mtime) = original_mtime {
        let _ = filetime::set_file_mtime(path, mtime);
    }

    Ok(())
}

/// Menyalin seluruh metadata dari file sumber (RAW/JPG) ke file tujuan (JPG/AVIF),
/// sekaligus menyuntikkan tag identitas kompresi.
pub fn copy_and_tag_metadata(src: &Path, dst: &Path, status: Option<&str>) -> Result<()> {
    let src_meta = rexiv2::Metadata::new_from_path(src);
    let payload = build_tag_payload(status);

    let host = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    if let Ok(meta) = src_meta {
        // Tanamkan tag di objek metadata yang akan disimpan ke dst
        let _ = meta.set_tag_string("Exif.Photo.UserComment", &payload);
        let _ = meta.set_tag_string(
            "Xmp.xmp.CreatorTool",
            &format!("gasinavif-rust ({host})"),
        );
        let _ = meta.save_to_file(dst);
    } else {
        // Jika pembacaan src gagal, minimal beri tag ke file tujuan dst
        let _ = tag_file(dst, status, false);
    }

    Ok(())
}

/// Mengambil raw EXIF bytes dari file (berguna jika ingin di-embed langsung ke AVIF encoder).
pub fn extract_raw_exif_bytes(path: &Path) -> Option<Vec<u8>> {
    // Gunakan img-parts (sangat cepat untuk JPG)
    if let Ok(bytes) = std::fs::read(path) {
        if let Ok(Some(dyn_img)) = img_parts::DynImage::from_bytes(bytes.into()) {
            if let Some(exif_bytes) = dyn_img.exif() {
                return Some(exif_bytes.to_vec());
            }
        }
    }

    None
}

/// Menyimpan timestamp modifikasi dari file sumber ke file tujuan (`touch -r`).
pub fn preserve_mtime(src: &Path, dst: &Path) -> Result<()> {
    let src_meta = std::fs::metadata(src)
        .with_context(|| format!("Gagal membaca metadata mtime: {}", src.display()))?;
    let mtime = filetime::FileTime::from_last_modification_time(&src_meta);
    filetime::set_file_mtime(dst, mtime)
        .with_context(|| format!("Gagal set mtime: {}", dst.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_format() {
        let payload = build_tag_payload(None);
        assert!(payload.starts_with(TAG_PREFIX));
        assert!(payload.contains("DATE="));
        assert!(payload.contains("HOST="));

        let skipped = build_tag_payload(Some(STATUS_SKIPPED_LARGER));
        assert!(skipped.contains("STATUS=SKIPPED_LARGER"));
    }

    #[test]
    fn test_tag_and_check_jpeg() {
        use image::{Rgb, RgbImage};
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("gasinavif_meta_test.jpg");

        let img = RgbImage::from_pixel(10, 10, Rgb([255, 0, 0]));
        img.save(&test_path).expect("Failed to create test image");

        // Awalnya belum ditandai
        assert!(!is_already_tagged(&test_path).unwrap());

        // Suntikkan tag
        tag_file(&test_path, None, true).expect("Failed to tag file");

        // Sekarang harus terdeteksi sudah ditandai
        assert!(is_already_tagged(&test_path).unwrap());

        let _ = std::fs::remove_file(&test_path);
    }
}
