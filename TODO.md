# 📋 Roadmap & To-Do List: Gasinavif (Rust Edition)

> **Gasinavif**: Alat CLI performa tinggi berbasis Rust untuk otomasi kompresi foto kamera (RAW → JPG → AVIF), pembersihan cerdas format ganda, penyalinan metadata EXIF, dan sistem *checkpoint/resume*.

---

## 📌 Status Proyek
- [x] **Fase 1: Inisialisasi Proyek & Setup Environment** ✅
- [x] **Fase 2: Antarmuka CLI & Parsing Argumen (`clap`)** ✅
- [x] **Fase 3: File Scanner & Logika Pairing Cerdas (RAW + JPG)** ✅
- [x] **Fase 4: Manajemen Metadata & EXIF In-Process** ✅
- [x] **Fase 5: Core Processing Engine (Tahap 1 & Tahap 2)** ✅
- [x] **Fase 6: State Management, Resume, & Signal Handling (`Ctrl+C`)** ✅
- [x] **Fase 7: Tampilan Progress Bar & Laporan Akhir (`indicatif`)** ✅
- [x] **Fase 8: Pengujian, Benchmarking, & Distribusi Binary** ✅
- [x] **Sprint 9: Kontrol Interaktif & Monitoring Real-Time (Live TUI)** ✅
- [x] **Sprint 10: Performa AVIF SIMD, Bug Fixing Terminal & Perapihan UI** ✅
- [x] **Sprint 11: Pengaturan Interaktif (--settings) & Batasan Thread CPU** ✅

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

### Fase 5: Core Processing Engine ✅
- [x] **Worker Tahap 1: RAW ke JPG**
  - Ekstrak embedded preview resolusi tinggi via `rexiv2` (zero demosaic CPU).
  - Fallback ke ImageMagick (`magick`) jika tidak ada preview beresolusi tinggi.
  - Simpan sebagai JPG sementara (`<nama>.jpg.tmp`) kualitas tinggi (q98).
  - Salin seluruh metadata EXIF dari RAW ke `.jpg.tmp`.
  - Pengecekan ukuran:
    - Jika `ukuran_jpg < ukuran_raw`: rename `.jpg.tmp` → `.jpg`, hapus file RAW asli.
    - Jika `ukuran_jpg >= ukuran_raw`: hapus `.jpg.tmp`, beri tag `SKIPPED_LARGER` pada RAW asli.
- [x] **Worker Tahap 2: JPG ke AVIF**
  - Cek apakah JPG sudah memiliki tag `COMPRESSED_BY_SCRIPT` di awal.
  - Encode JPG ke AVIF sementara (`<nama>.avif.tmp`) dengan setting (q60, speed 8).
  - Salin metadata EXIF dari JPG dan tanamkan tag `COMPRESSED_BY_SCRIPT` langsung ke AVIF container via `ravif`.
  - Pengecekan ukuran:
    - Jika `ukuran_avif < ukuran_jpg`:
      - Salin timestamp modifikasi dari JPG asli ke `.avif.tmp`.
      - Hapus JPG asli, rename `.avif.tmp` → `.avif`.
    - Jika `ukuran_avif >= ukuran_jpg` (kompresi gagal menghemat ukuran):
      - Hapus `.avif.tmp`.
      - Tandai JPG asli dengan `STATUS=SKIPPED_LARGER` (agar tidak diproses ulang di masa mendatang).
- [x] Parallel processing pipeline di `main.rs` via `rayon` dengan progress bar interaktif dan laporan akhir.

---

### Fase 6: State Management, Resume, & Signal Handling (`Ctrl+C`) ✅
- [x] Struktur state checkpoint (`.gasinavif_state/progress.json`):
  - Daftar file yang selesai (`completed`), file dilewati (`skipped`), dan ukuran awal.
  - Serialisasi/deserialisasi cepat via `serde_json`.
- [x] Penanganan sinyal interupsi graceful (`SIGINT` / `Ctrl+C`):
  - Tangkap sinyal melalui handler `ctrlc`.
  - Beri sinyal ke worker `rayon` untuk segera berhenti (`AtomicBool`).
  - Scan dan bersihkan file residu `*.tmp` yang belum tuntas.
  - Simpan snapshot state terakhir ke disk sebelum proses keluar (exit code 130).
- [x] Logika Resume:
  - Jika file state ditemukan, muat daftar progres dan langsung lewati file-file yang sudah sukses.
  - Jika argumen `--fresh` diberikan, hapus file state dan mulai dari awal.
  - Bersihkan folder state otomatis saat seluruh pemrosesan tuntas 100%.
- [x] Unit tests untuk siklus hidup `AppState` dan pembersihan file temporer (`clean_temp_files`).

---

### Fase 7: Tampilan Progress Bar & Laporan Akhir (`indicatif`) ✅
- [x] Implementasi UI progress bar:
  - Bar 1: Progres Tahap 1 (RAW → JPG) dengan timer elapsed, spinner, dan counter.
  - Bar 2: Progres Tahap 2 (JPG → AVIF) dengan timer elapsed, spinner, dan counter.
- [x] Format log badge berwarna yang konsisten dan bebas flicker (`pb.println`):
  - `[OK-RAW]` (cyan) / `[HAPUS-RAW]` (magenta) / `[BATAL]` (yellow)
  - `[OK-AVIF]` (green) / `[SKIP-TAG]` (dim) / `[GAGAL]` (red)
- [x] Ringkasan statistik akhir berbingkai warna (*boxed colorful summary table*):
  - Ukuran folder awal vs ukuran folder akhir.
  - Total megabyte/gigabyte yang berhasil dihemat dan persentase efisiensi.
  - Durasi eksekusi dan kecepatan rata-rata (file/detik).
- [x] Unit tests untuk formatting durasi, ukuran bytes, dan string badges.

---

### Fase 8: Pengujian, Benchmarking, & Distribusi ✅
- [x] Uji coba dengan berbagai skenario (RAW+JPG dual-shoot, single JPG, and fallback rendering).
- [x] Uji coba proteksi anti-loop (verifikasi file ber-tag `COMPRESSED_BY_SCRIPT` langsung dilewati instan).
- [x] Uji coba simulasi interupsi `Ctrl+C` dan kemampuan *resume*.
- [x] Kompilasi binary rilis teroptimasi: `cargo build --release` (LTO, strip symbols, opt-level 3, binary size ~6.2 MB).
- [x] Instalasi binary ke `~/.local/bin/gasinavif`.
- [x] Benchmark throughput: ~13.8 file/detik dengan penghematan ukuran > 85%.

---

### Sprint 9: Kontrol Interaktif & Monitoring Real-Time (Live TUI) ✅
- [x] **Indikator Kecepatan & Estimasi ETA**:
  - Kecepatan pemrosesan dinamis (*rolling average* foto/detik).
  - Estimasi waktu tersisa (ETA) format `hh:mm:ss` / `mm:ss` yang diperbarui secara langsung.
- [x] **Live Interactive Keyboard Controls (Non-blocking)**:
  - Mode raw terminal via `crossterm` dengan pengembalian mode otomatis (*RAII guard*).
  - `s` (Show Workers): Toggle tampilan baris per worker thread/core dan nama file yang sedang aktif dikompresi.
  - `v` (Verbose): Toggle pesan log detail secara *on-the-fly* tanpa perlu restart program.
  - `t` (Threads): Pengaturan jumlah worker paralel dinamis saat kompresi sedang berjalan (`+` dan `-` juga didukung).
  - `l` (List/Folder Tree): Tampilan struktur direktori pohon (*tree hierarchy*) dengan karakter `├── `, `└── `, indikator progres `x/y` per folder, serta animasi spinner / centang hijau (`✔`) saat folder tuntas.
- [x] **Distribusi & Rilis**:
  - Kompilasi ulang release binary dengan fitur interaktif baru dan instalasi ke `~/.local/bin/gasinavif`.

---

### Sprint 10: Performa AVIF SIMD, Bug Fixing Terminal & Perapihan UI ✅
- [x] **Akselerasi Encoding AVIF (SIMD Fast Path)**:
  - Integrasi langsung ke `avifenc` berbasis `libaom` + `libjpeg-turbo` dengan akselerasi instruksi AVX2/AVX-512.
  - Peningkatan performa drastis dari 6.4 detik/foto menjadi 0.5 detik/foto (12x lebih kencang).
  - Mekanisme *auto-fallback* ke `ravif` jika `avifenc` gagal atau tidak ditemukan agar tidak ada file yang gagal diproses.
- [x] **Perbaikan Output Terminal & Eliminasi Tangga Diagonal (*Staircase Bug*)**:
  - Penataan mode terminal: mode raw hanya diaktifkan saat worker pemrosesan aktif, membiarkan tahap scan awal tetap dalam mode *cooked* normal.
  - Setiap baris keluaran pada mode raw dipastikan diawali `\r` (kolom 0) dan diakhiri `\x1B[K\r\n` agar kursor tidak bergeser miring.
  - Penyesuaian lebar dinamis progress bar dan status line berdasarkan ukuran terminal untuk mencegah *line wrapping* liar.
- [x] **Peredaman Warning Eksternal (*Noise Reduction*)**:
  - Konfigurasi `rexiv2::set_log_level(rexiv2::LogLevel::MUTE)` agar warning GLib/Exiv2 tidak bocor mengotori terminal.
- [x] **Perapihan Tabel Kotak Ringkasan (*Box Alignment Bug*)**:
  - Koreksi lebar kolom tabel `HASIL PEMINDAIAN` (tepat 42 kolom visual).
  - Koreksi judul dan baris `Kecepatan rata-rata` pada `LAPORAN RINGKASAN GASINAVIF` (tepat 50 kolom visual tanpa ada border kanan `║` yang mencuat keluar).
- [x] **Kompilasi & Instalasi Rilis**:
  - Kompilasi `cargo build --release` dan pemasangan ulang ke `~/.local/bin/gasinavif`.

---

### Sprint 11: Pengaturan Interaktif (--settings) & Batasan Thread CPU ✅
- [x] **Sistem Konfigurasi Global Persisten**:
  - Simpan & muat konfigurasi di `~/.config/gasinavif/config.json`.
  - Field: `speed` (1-10), `quality` (1-100), `yuv` ("420", "422", "444", "400").
  - Auto-save saat keluar atau mengubah pengaturan, menjadi nilai default eksekusi selanjutnya.
- [x] **Menu TUI Interaktif (`--settings`)**:
  - Menu utama navigasi panah atas/bawah dengan highlight warna dan pemilih Enter.
  - Submenu Speed: slider visual digeser kiri/kanan (`1..10`) dengan kotak penjelasan dinamis.
  - Submenu Kualitas (q): penyesuaian atas/bawah per-step 1 (`1..100`) dengan bar visual dan penjelasan rentang kualitas.
  - Submenu YUV Format: pilihan 444, 422, 420, 400 dengan penjelasan chroma subsampling.
  - Tombol kembali (Esc / q) dan auto-save otomatis.
- [x] **Proteksi & Pembatasan Thread CPU**:
  - Cap jumlah thread paralel maksimal sebesar jumlah core/thread logis CPU komputer (`available_parallelism`).
  - Cegah penambahan thread via argumen `-j` maupun hotkey interaktif `t`/`+` melampaui batas kemampuan CPU fisik/logis.
