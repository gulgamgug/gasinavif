# 📸 gasinavif

**High-performance CLI tool for camera photo compression** — mengotomasi konversi foto kamera dari format RAW dan JPG ke AVIF dengan kompresi optimal, metadata EXIF terjaga, dan sistem checkpoint/resume.

Ditulis sepenuhnya dalam **Rust** 🦀 untuk kecepatan maksimal.

---

## ✨ Fitur Utama

- **🔄 Pipeline Dua Tahap**
  - **Tahap 1**: RAW → JPG (decode format kamera: CR2, CR3, ARW, NEF, DNG, dll.)
  - **Tahap 2**: JPG → AVIF (kompresi AV1 modern, hemat 40-80% ukuran file)

- **🧠 Deteksi Pasangan Cerdas (RAW + JPG)**
  - Mendeteksi file RAW yang sudah memiliki pasangan JPG dari kamera (dual-format shooting)
  - RAW duplikat otomatis dihapus, JPG langsung masuk antrean AVIF

- **🏷️ Metadata EXIF Terjaga**
  - Semua metadata EXIF (aperture, shutter speed, ISO, GPS, dll.) disalin ke file hasil
  - Tag `COMPRESSED_BY_SCRIPT` ditanamkan untuk proteksi anti-loop

- **⚡ Multi-Threaded & Blazingly Fast**
  - Work-stealing threadpool via `rayon` — memanfaatkan seluruh core CPU
  - Pembacaan metadata EXIF secara native (tanpa spawn `exiftool`) — 100-500x lebih cepat

- **💾 Checkpoint & Resume**
  - Progres disimpan ke `.gasinavif_state/progress.json`
  - Jika dihentikan (`Ctrl+C`), proses bisa langsung dilanjutkan tanpa mengulang file yang sudah selesai
  - File sementara (`*.tmp`) otomatis dibersihkan saat interupsi

- **📊 Laporan Statistik**
  - Ukuran folder sebelum & sesudah
  - Total penghematan dalam MB/GB dan persentase
  - Durasi eksekusi

---

## 📦 Instalasi

### Prasyarat

- [Rust toolchain](https://rustup.rs/) (rustc ≥ 1.75)
- NASM assembler (opsional, untuk optimasi encoding AV1):
  ```bash
  # Arch Linux
  sudo pacman -S nasm

  # Ubuntu / Debian
  sudo apt install nasm

  # macOS
  brew install nasm
  ```

### Build dari Source

```bash
git clone https://github.com/raihanlazuardi/gasinavif.git
cd gasinavif
cargo build --release
```

Binary akan tersedia di `target/release/gasinavif`.

### Instalasi ke PATH

```bash
cp target/release/gasinavif ~/.local/bin/
```

---

## 🚀 Penggunaan

### Perintah Dasar

```bash
# Proses semua foto di direktori saat ini
gasinavif

# Proses direktori tertentu
gasinavif ~/Pictures/2024-Vacation

# Mode verbose (lihat detail per file)
gasinavif -v ~/Pictures

# Dry-run (analisis tanpa mengubah file)
gasinavif --dry-run ~/Pictures
```

### Opsi Lengkap

```
Usage: gasinavif [OPTIONS] [DIRECTORY]

Arguments:
  [DIRECTORY]  Target directory to process [default: .]

Options:
      --settings           Buka antarmuka interaktif pengaturan global (TUI Settings)
  -f, --fresh              Reset checkpoint state dan mulai dari awal
  -v, --verbose            Tampilkan log detail per file
  -q, --quality <QUALITY>  Kualitas AVIF, 1-100 (default: 60 / dari config)
  -s, --speed <SPEED>      Kecepatan encoder, 1-10 (default: 8 / dari config)
      --yuv <YUV>          Format chroma subsampling: 420, 422, 444, 400 (default: 420)
  -j, --threads <THREADS>  Jumlah thread paralel (dibatasi maksimal core CPU)
      --dry-run            Simulasi tanpa menulis/menghapus file
      --keep-raw           Pertahankan file RAW meskipun JPG sudah ada
  -h, --help               Tampilkan bantuan
  -V, --version            Tampilkan versi
```

### ⚙️ Pengaturan Global Interaktif (`gasinavif --settings`)

Gasinavif menyediakan menu TUI interaktif untuk mengatur konfigurasi global yang disimpan permanen di `~/.config/gasinavif/config.json`:

```bash
gasinavif --settings
```

Fitur menu pengaturan:
- **Navigasi Intuitif**: Panah Atas/Bawah (`↑`/`↓`) untuk berpindah menu dengan highlight warna, `Enter` untuk membuka submenu, dan `Esc` / `q` untuk kembali.
- **Slider Kecepatan (Speed 1–10)**: Geser kiri/kanan (`←`/`→`) dengan panduan dinamis kompromi kecepatan vs kompresi.
- **Pengatur Kualitas (Quality 1–100)**: Penyesuaian step 1 (`↑`/`↓`) atau step 10 (`PgUp`/`PgDn`) dengan bar visual dan penjelasan rentang kualitas.
- **Pemilih Format Chroma (YUV)**: Pilihan format `420` (default kompatibel), `422`, `444` (full color), atau `400` (monokrom).
- **Auto-Save Otomatis**: Setiap perubahan langsung tersimpan ke disk dan menjadi default untuk semua sesi berikutnya tanpa perlu konfigurasi ulang.
- **Proteksi Alokasi Thread**: Penggunaan worker thread dibatasi secara otomatis sebesar jumlah thread logis CPU perangkat (tidak akan membebani CPU secara berlebihan).


### 🎮 Kontrol Interaktif Saat Program Berjalan (Live Hotkeys)

Saat proses kompresi sedang berjalan di terminal, Anda dapat menekan tombol pintas (*hotkey*) berikut secara langsung tanpa perlu menekan Enter:

| Tombol | Aksi | Deskripsi |
| :---: | :--- | :--- |
| **`s`** | **Show Active Workers** | Toggle tampilan nama file yang sedang aktif diproses per core/thread. |
| **`v`** | **Toggle Verbose** | Mengaktifkan/menonaktifkan log detail per file secara *on-the-fly*. |
| **`t`** / **`+`** / **`-`** | **Adjust Threads** | Mengatur jumlah worker thread secara dinamis saat proses berjalan (`+` tambah, `-` kurang). |
| **`l`** | **Folder Tree Progress** | Toggle tampilan struktur pohon folder beserta progres file dan animasi status per folder. |
| **`q`** | **Quit / Graceful Stop** | Menghentikan proses dengan aman, membersihkan file `.tmp`, dan menyimpan state. |

#### Contoh Tampilan Tree Mode (`l`):
```text
  ── 📁 Hierarki Folder & Progres ──
  Vacation                         ⠋ 12/45
  ├── Beach                        ✔ 20/20
  └── Mountain                     ⠙ 5/15
  Studio                           ✔ 10/10
  ⠋ JPG → AVIF [00:00:15] [██████████████████████████████] 47/90 (52%) | 4.8 foto/s | ETA: 00:09
    [ s:show (OFF) | l:tree (ON) | v:verbose (OFF) | t:threads (12) | q:quit ]
```

#### Contoh Tampilan Active Workers Mode (`s`):
```text
  ── ⚡ Status Pekerja (Active Threads: 12) ──
    [Core  0] /photos/Vacation/Beach/beach_01.jpg
    [Core  1] /photos/Vacation/Beach/beach_02.jpg
    [Core  2] /photos/Studio/model_05.jpg
    [Core  3] (idle)
    ...
  ⠋ JPG → AVIF [00:00:15] [██████████████████████████████] 47/90 (52%) | 4.8 foto/s | ETA: 00:09
    [ s:show (ON) | l:tree (OFF) | v:verbose (OFF) | t:threads (12) | q:quit ]
```

### Contoh Skenario

#### Foto liburan dengan kamera DSLR (dual-format RAW+JPG)
```bash
# gasinavif otomatis mendeteksi pasangan RAW+JPG,
# menghapus RAW duplikat, dan mengkompresi JPG ke AVIF
gasinavif ~/Pictures/2024-Bali/
```

#### Koleksi RAW saja (studio / profesional)
```bash
# RAW di-decode ke JPG dulu, lalu JPG dikompresi ke AVIF
# Kualitas lebih tinggi untuk hasil cetak
gasinavif -q 75 ~/Pictures/Studio-Shoot/
```

#### Preview cepat tanpa mengubah file
```bash
gasinavif --dry-run -v ~/Pictures/
```

---

## 📁 Format RAW yang Didukung

| Merek Kamera | Ekstensi |
|---|---|
| Canon | `.cr2`, `.cr3` |
| Sony | `.arw`, `.srf`, `.sr2` |
| Nikon | `.nef`, `.nrw` |
| Adobe (DNG) | `.dng` |
| Fujifilm | `.raf` |
| Panasonic | `.rw2` |
| Olympus | `.orf` |
| Pentax | `.pef` |

---

## ⚙️ Cara Kerja

```
                    ┌─────────────────────┐
                    │   Scan Direktori    │
                    │   (walkdir, O(N))   │
                    └────────┬────────────┘
                             │
                    ┌────────▼────────────┐
                    │  Grouping by Stem   │
                    │  (HashMap in-RAM)   │
                    └────────┬────────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
     RAW + JPG ada      Hanya RAW      Hanya JPG
              │              │              │
        Hapus RAW     Tahap 1: Decode    Langsung
              │        RAW → JPG            │
              │              │              │
              └──────────────┼──────────────┘
                             │
                    ┌────────▼────────────┐
                    │   Tahap 2: Encode   │
                    │   JPG → AVIF        │
                    │   (ravif / rav1e)   │
                    └────────┬────────────┘
                             │
                    ┌────────▼────────────┐
                    │ Validasi Ukuran     │
                    │ avif < jpg? Simpan  │
                    │ avif ≥ jpg? Skip    │
                    └────────┬────────────┘
                             │
                    ┌────────▼────────────┐
                    │   Laporan Akhir     │
                    │   📊 Statistik      │
                    └─────────────────────┘
```

---

## 🛡️ Proteksi & Keamanan

- **Anti-loop**: File yang sudah ditandai `COMPRESSED_BY_SCRIPT` tidak akan diproses ulang
- **Validasi ukuran**: Jika hasil kompresi lebih besar dari aslinya, file asli dipertahankan
- **Graceful shutdown**: `Ctrl+C` membersihkan file `.tmp` yang belum selesai
- **Resume**: Proses bisa dilanjutkan kapan saja tanpa kehilangan progres

---

## 🏗️ Tech Stack

| Komponen | Library |
|---|---|
| CLI Parser | `clap` v4 (derive) |
| Parallelism | `rayon` |
| Progress Bar | `indicatif` |
| Dir Traversal | `walkdir` |
| RAW Decoding | `rawloader` (pure Rust) |
| AVIF Encoding | `ravif` + `rav1e` (pure Rust) |
| EXIF Metadata | `kamadak-exif` + `img-parts` |
| State / Resume | `serde_json` |
| Signal Handling | `ctrlc` |

---

## 📄 Lisensi

MIT License — Silakan digunakan dan dimodifikasi sesuai kebutuhan.
