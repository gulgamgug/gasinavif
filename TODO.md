# 📋 Roadmap & To-Do List: Gasinavif (Rust Edition)

> **Gasinavif**: Alat CLI performa tinggi berbasis Rust untuk otomasi kompresi foto kamera (RAW → JPG → AVIF), pembersihan cerdas format ganda, penyalinan metadata EXIF, dan sistem *checkpoint/resume*.

---

## 📌 Status Proyek
- [x] **Fase 1: Inisialisasi Proyek & Setup Environment** ✅
- [x] **Fase 2: Antarmuka CLI & Parsing Argumen (`clap`)** ✅
- [x] **Fase 3: File Scanner & Logika Pairing Cerdas (RAW + JPG)** ✅
- [x] **Fase 4: Manajemen Metadata & EXIF In-Process** ✅
- [ ] **Fase 5: Core Processing Engine (Tahap 1 & Tahap 2)**
- [ ] **Fase 6: State Management, Resume, & Signal Handling (`Ctrl+C`)**
- [ ] **Fase 7: Tampilan Progress Bar & Laporan Akhir (`indicatif`)**
- [ ] **Fase 8: Pengujian, Benchmarking, & Distribusi Binary**

---

## 🚀 Rincian To-Do List

### Fase 1: Inisialisasi Proyek & Setup Environment ✅
- [x] Inisialisasi Cargo binary project (`cargo init --bin`).
- [x] Konfigurasi `Cargo.toml`:
  - `clap` (fitur `derive`): CLI parser.
  - `rayon`: Work-stealing threadpool untuk konkurensi multi-core.
  - `indicatif` & `console`: Multi-progress bar modern dan warna terminal.
  - `walkdir`: Pemindaian direktori rekursif yang sangat cepat.
  - `image` & crate decoding/encoding gambar.
  - `rawloader` (pure Rust): Decoding RAW kamera.
  - `ravif` (pure Rust via `rav1e`): Encoding AVIF.
  - `kamadak-exif` & `img-parts`: Metadata EXIF native.
  - `serde` & `serde_json`: Serialisasi state untuk fitur resume.
  - `ctrlc`: Penanganan sinyal interupsi terminal.
  - `filetime`: Mempertahankan timestamp modifikasi file (`touch -r`).
- [x] Buat struktur folder modular di `src/`:
  - `src/main.rs`: Entry point program.
  - `src/cli.rs`: Definisi argumen dan opsi CLI.
  - `src/scanner.rs`: Logika pemindaian direktori dan deteksi pasangan RAW+JPG.
  - `src/metadata.rs`: Pembacaan, injeksi, dan verifikasi tag EXIF.
  - `src/processor.rs`: Engine konversi (RAW → JPG dan JPG → AVIF).
  - `src/state.rs`: Penyimpanan state antrean dan resume checkpoint.
  - `src/ui.rs`: Progress bar dan rekap statistik laporan.

---

### Fase 2: Antarmuka CLI & Parsing Argumen (`clap`) ✅
- [x] Definisikan struktur argumen CLI di `src/cli.rs`:
  - `directory`: Path direktori target (posisional, default: `.`).
  - `-f, --fresh`: Hapus state checkpoint lama dan mulai pemindaian dari nol.
  - `-v, --verbose`: Tampilkan log detail per file secara real-time.
  - `-q, --quality`: Kualitas kompresi AVIF (default: `60`).
  - `-s, --speed`: Kecepatan encoder AVIF 1–10 (default: `8`).
  - `-j, --threads`: Jumlah worker thread paralel (default: jumlah core CPU).
  - `--dry-run`: Mode simulasi (analisis ukuran tanpa menulis/menghapus file).
  - `--keep-raw`: Opsi tidak menghapus RAW asli meskipun JPG sudah ada/selesai dibuat.
- [x] Buat tampilan pesan `--help` yang informatif dan user-friendly.

---

### Fase 3: File Scanner & Logika Pairing Cerdas (RAW + JPG) ✅
- [x] Definisikan daftar ekstensi yang didukung:
  - RAW: `.cr2`, `.cr3`, `.arw`, `.srf`, `.sr2`, `.nef`, `.nrw`, `.dng`, `.raf`, `.rw2`, `.orf`, `.pef`.
  - JPG: `.jpg`, `.jpeg`.
- [x] Implementasi traversal direktori rekursif dengan `walkdir` (abaikan direktori state internal).
- [x] Logika pengelompokan file (*In-Memory File Grouping*):
  - Kelompokkan file berdasarkan folder induk dan nama dasar (*stem*) tanpa ekstensi.
  - **Kondisi 1 (Pasangan RAW + JPG terdeteksi)**:
    - Tandai file RAW untuk dihapus (kamera sudah menghasilkan JPG).
    - Masukkan file JPG ke antrean Tahap 2 (AVIF).
  - **Kondisi 2 (Hanya file RAW)**:
    - Masukkan ke antrean Tahap 1 (Konversi RAW ke JPG).
  - **Kondisi 3 (Hanya file JPG)**:
    - Masukkan ke antrean Tahap 2 (Konversi JPG ke AVIF).
- [x] Hitung total ukuran awal folder untuk dasar kalkulasi penghematan di akhir.
- [x] Wiring scanner ke `main.rs` dengan ringkasan pemindaian, verbose listing, dan penghapusan RAW duplikat.

---

### Fase 4: Manajemen Metadata & EXIF In-Process ✅
- [x] Format spesifikasi payload metadata:
  `COMPRESSED_BY_SCRIPT=true;DATE=<ISO_8601>;HOST=<HOSTNAME>`
- [x] Format tag penanda batal:
  `COMPRESSED_BY_SCRIPT=true;DATE=<ISO_8601>;HOST=<HOSTNAME>;STATUS=SKIPPED_LARGER`
- [x] Fungsi baca cepat tag metadata `UserComment` & `Xmp.xmp.CreatorTool` secara *in-process* via `rexiv2` (tanpa spawn `exiftool`).
- [x] Fungsi duplikasi dan penyalinan seluruh metadata EXIF dari file sumber ke file tujuan.
- [x] Fungsi injeksi tag metadata ke dalam file gambar.
- [x] Fungsi preservasi timestamp file (*mtime*) agar tanggal foto tetap asli.
- [x] Unit tests untuk validasi payload dan read/write tag JPEG in-memory.

---

### Fase 5: Core Processing Engine
- [ ] **Worker Tahap 1: RAW ke JPG**
  - Decode file RAW ke image buffer.
  - Simpan sebagai JPG sementara (`<nama>.jpg.tmp`) kualitas tinggi (q98).
  - Salin metadata EXIF dari RAW ke `.jpg.tmp`.
  - Pengecekan ukuran:
    - Jika `ukuran_jpg < ukuran_raw`: rename `.jpg.tmp` → `.jpg`, hapus file RAW asli.
    - Jika `ukuran_jpg >= ukuran_raw`: hapus `.jpg.tmp`, beri tag `SKIPPED_LARGER` pada RAW asli.
- [ ] **Worker Tahap 2: JPG ke AVIF**
  - Cek apakah JPG sudah memiliki tag `COMPRESSED_BY_SCRIPT` (skip jika sudah).
  - Encode JPG ke AVIF sementara (`<nama>.avif.tmp`) dengan setting (q60, speed 8, YUV 4:2:0).
  - Salin metadata EXIF dari JPG ke `.avif.tmp` beserta injeksi tag identitas script.
  - Pengecekan ukuran:
    - Jika `ukuran_avif < ukuran_jpg`:
      - Salin timestamp modifikasi dari JPG asli ke `.avif.tmp`.
      - Hapus JPG asli, rename `.avif.tmp` → `.avif`.
    - Jika `ukuran_avif >= ukuran_jpg` (kompresi gagal menghemat ukuran):
      - Hapus `.avif.tmp`.
      - Tandai JPG asli dengan `STATUS=SKIPPED_LARGER` (agar tidak diproses ulang di masa mendatang).

---

### Fase 6: State Management, Resume, & Signal Handling (`Ctrl+C`)
- [ ] Buat struktur state checkpoint (disimpan di `.gasinavif_state.json`):
  - Status pemindaian direktori.
  - Daftar file yang selesai diproses beserta statusnya (`Success`, `Skipped`, `Failed`).
  - Catatan total ukuran folder awal.
- [ ] Tangani sinyal interupsi (`SIGINT` / `Ctrl+C`):
  - Tangkap sinyal melalui handler `ctrlc`.
  - Beri tahu semua thread pekerja untuk berhenti dengan aman (*graceful stop*).
  - Scan dan bersihkan file residu `*.tmp` yang belum tuntas.
  - Simpan snapshot state terakhir ke disk sebelum proses keluar.
- [ ] Logika Resume:
  - Jika `.gasinavif_state.json` ditemukan, muat daftar progres dan langsung lewati file-file yang sudah sukses.
  - Jika argumen `--fresh` diberikan, hapus file state dan mulai dari awal.

---

### Fase 7: Tampilan Progress Bar & Laporan Akhir (`indicatif`)
- [ ] Implementasi UI multi-bar:
  - Bar 1: Progres Tahap 1 (RAW → JPG).
  - Bar 2: Progres Tahap 2 (JPG → AVIF).
- [ ] Format log verbose yang bersih (tidak merusak tampilan progress bar):
  - `[OK-RAW]` / `[HAPUS-RAW]` / `[BATAL-RAW]`
  - `[OK-AVIF]` / `[SKIP-TAG]` / `[BATAL-AVIF]`
- [ ] Ringkasan statistik di akhir pemrosesan:
  - Ukuran folder awal vs ukuran folder akhir.
  - Total megabyte/gigabyte yang berhasil dihemat.
  - Persentase efisiensi penghematan (`saved %`).
  - Total durasi eksekusi.

---

### Fase 8: Pengujian, Benchmarking, & Distribusi
- [ ] Uji coba dengan berbagai format file RAW kamera nyata (CR2, CR3, ARW, NEF, DNG).
- [ ] Uji coba proteksi anti-loop (pastikan file yang ditandai tidak pernah diproses ulang).
- [ ] Uji coba simulasi interupsi `Ctrl+C` dan kemampuan *resume*.
- [ ] Kompilasi binary rilis teroptimasi: `cargo build --release`.
- [ ] Tambahkan petunjuk instalasi ke `~/.local/bin/gasinavif`.
