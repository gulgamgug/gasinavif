//! metadata.rs — Pembacaan dan penulisan tag EXIF secara in-process
//!
//! Menggantikan pemanggilan `exiftool` eksternal dengan pembacaan native,
//! sehingga pengecekan metadata menjadi ~100-500x lebih cepat.

use anyhow::{Context, Result};
use chrono::Utc;
use std::path::Path;

/// Prefix tag identitas yang ditanamkan di field UserComment.
pub const TAG_PREFIX: &str = "COMPRESSED_BY_SCRIPT=true";

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

/// Mengecek apakah sebuah file sudah memiliki tag `COMPRESSED_BY_SCRIPT` di metadata EXIF.
///
/// Mengembalikan `true` jika file sudah pernah diproses (harus di-skip).
pub fn is_already_tagged(path: &Path) -> Result<bool> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Gagal membuka file: {}", path.display()))?;
    let mut buf_reader = std::io::BufReader::new(&file);

    let exif_reader = exif::Reader::new();
    let exif = match exif_reader.read_from_container(&mut buf_reader) {
        Ok(exif) => exif,
        Err(_) => return Ok(false), // Tidak ada EXIF → belum ditandai
    };

    if let Some(field) = exif.get_field(exif::Tag::UserComment, exif::In::PRIMARY) {
        let value = field.display_value().to_string();
        return Ok(value.contains(TAG_PREFIX));
    }

    Ok(false)
}

/// Menyalin metadata EXIF dari file sumber ke file tujuan.
///
/// Menggunakan `img-parts` untuk memindahkan segmen EXIF antar container JPEG.
pub fn copy_exif_jpg_to_jpg(src: &Path, dst: &Path) -> Result<()> {
    use img_parts::jpeg::Jpeg;
    use img_parts::{DynImage, ImageEXIF};

    // Baca EXIF dari sumber
    let src_bytes = std::fs::read(src)
        .with_context(|| format!("Gagal membaca file sumber: {}", src.display()))?;
    let src_img = DynImage::from_bytes(src_bytes.into())
        .with_context(|| format!("Gagal parse image sumber: {}", src.display()))?
        .ok_or_else(|| anyhow::anyhow!("Format image tidak dikenali: {}", src.display()))?;
    let exif_data = src_img.exif();

    // Baca file tujuan
    let dst_bytes = std::fs::read(dst)
        .with_context(|| format!("Gagal membaca file tujuan: {}", dst.display()))?;
    let mut dst_jpg = Jpeg::from_bytes(dst_bytes.into())
        .with_context(|| format!("Gagal parse JPEG tujuan: {}", dst.display()))?;

    // Sisipkan EXIF
    if let Some(exif_bytes) = exif_data {
        dst_jpg.set_exif(Some(exif_bytes));
    }

    // Tulis kembali
    let mut output = std::fs::File::create(dst)
        .with_context(|| format!("Gagal menulis file tujuan: {}", dst.display()))?;
    dst_jpg.encoder().write_to(&mut output)?;

    Ok(())
}

/// Menyimpan timestamp modifikasi dari file sumber ke file tujuan.
pub fn preserve_mtime(src: &Path, dst: &Path) -> Result<()> {
    let src_meta = std::fs::metadata(src)
        .with_context(|| format!("Gagal membaca metadata: {}", src.display()))?;
    let mtime = filetime::FileTime::from_last_modification_time(&src_meta);
    filetime::set_file_mtime(dst, mtime)
        .with_context(|| format!("Gagal set mtime: {}", dst.display()))?;
    Ok(())
}
