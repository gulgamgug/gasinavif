# gasinavif

A high-performance CLI tool for camera photo compression (RAW -> JPG -> AVIF) written in Rust.

`gasinavif` automates the ingestion, deduplication, and modern AVIF compression of large photography archives while preserving complete EXIF metadata, original timestamps, and directory structures.

---

## Key Features

- **Two-Stage Conversion Pipeline**:
  - **Stage 1**: Decodes camera RAW formats (`.cr2`, `.cr3`, `.arw`, `.nef`, `.dng`, etc.) into high-quality JPEG by extracting embedded preview streams via `gexiv2`, with ImageMagick as an automatic fallback.
  - **Stage 2**: Encodes JPEG files to modern AVIF format, typically saving 40% to 85% disk space compared to original JPEGs.
- **Intelligent Pairing & Deduplication**:
  - Automatically detects paired RAW + JPG files produced by camera dual-format shooting.
  - Safely deletes duplicate RAW files while sending the camera-rendered JPG directly to Stage 2.
- **SIMD-Accelerated Performance**:
  - Prioritizes external `avifenc` (SIMD-optimized `libaom` + `libjpeg-turbo`) for encoding speeds (~0.5s per image).
  - Automatically falls back to an embedded pure-Rust `ravif` encoder if `avifenc` is not installed or encounters unusual color profiles.
- **Full EXIF & Timestamp Preservation**:
  - Copies all EXIF metadata (exposure, aperture, ISO, lens data, GPS coordinates, color profiles).
  - Preserves original file modification timestamps (`mtime`).
  - Injects a `COMPRESSED_BY_SCRIPT=true` tag into metadata to eliminate circular re-compression.
- **Size Increase Protection**:
  - Compares the final AVIF size against the original JPEG.
  - If the compressed file is equal to or larger than the source, the temporary AVIF is discarded, the original JPEG is preserved, and marked with a `SKIPPED_LARGER` tag.
- **Crash Recovery & Checkpointing**:
  - Records real-time completion state in `.gasinavif_state/progress.json`.
  - Safely handles interruptions (`Ctrl+C` or `q`): immediately cleans up partial `.tmp` files and preserves progress so execution can resume instantly.
- **Interactive Terminal UI (Live Controls)**:
  - Non-blocking runtime hotkeys to monitor active worker threads, inspect folder tree progress, toggle verbose logs, and dynamically reconfigure thread counts without restarting.
- **Persistent Global Configuration**:
  - Dedicated interactive settings interface (`gasinavif --settings`) with auto-save to `~/.config/gasinavif/config.json`.
- **CPU Core Thread Capping**:
  - Enforces an upper bound on worker threads matching the host machine's logical core capacity (`available_parallelism`), preventing CPU thrashing and system instability.

---

## Dependencies & Prerequisites

### Build Dependencies
Building `gasinavif` requires a standard Rust toolchain (Rust 2024 / rustc >= 1.75) alongside C libraries for EXIF parsing:

- **Rust toolchain** (`cargo`, `rustc`)
- **pkg-config** / **pkgconf**
- **libgexiv2** and **exiv2** (development headers)
- **nasm** (Netwide Assembler, required for SIMD assembly in pure-Rust AV1 routines)

### Runtime Dependencies (Recommended)
While `gasinavif` can run standalone using its built-in Rust encoder, the following system utilities are strongly recommended for maximum encoding throughput:

- **avifenc** (from `libavif`): Provides hardware-accelerated SIMD AVIF encoding (10-15x faster than pure-Rust fallback).
- **imagemagick** (`magick`): Fallback decoder for rare camera RAW files lacking embedded JPEG previews.

---

### Installing Dependencies

#### Arch Linux / Manjaro
```bash
sudo pacman -S --needed base-devel rust pkgconf libgexiv2 exiv2 nasm libavif imagemagick
```

#### Debian / Ubuntu / Linux Mint
```bash
sudo apt update
sudo apt install -y build-essential cargo pkg-config libgexiv2-dev libexiv2-dev nasm libavif-bin imagemagick
```

#### Fedora / RHEL
```bash
sudo dnf install -y gcc gcc-c++ cargo pkgconf-pkg-config libgexiv2-devel exiv2-devel nasm libavif-tools ImageMagick
```

#### macOS (Homebrew)
```bash
brew install rust pkg-config gexiv2 exiv2 nasm libavif imagemagick
```

---

## Installation

### Building from Source

Clone the repository and compile the release binary:

```bash
git clone https://github.com/raihanlazuardi/gasinavif.git
cd gasinavif
cargo build --release
```

The optimized binary will be created at `target/release/gasinavif`.

### Installing to PATH

Copy the compiled binary to your local user executable path:

```bash
install -Dm755 target/release/gasinavif ~/.local/bin/gasinavif
```

Ensure `~/.local/bin` is in your shell `$PATH`.

---

## Usage

### Basic Commands

```bash
# Process all photos in the current directory
gasinavif

# Process a specific target directory
gasinavif ~/Pictures/2024-Vacation

# Run with detailed real-time logs
gasinavif -v ~/Pictures

# Dry-run mode: analyze counts and sizes without modifying or deleting files
gasinavif --dry-run ~/Pictures

# Re-scan from scratch, ignoring previous checkpoints
gasinavif --fresh ~/Pictures
```

---

### Command-Line Options

```
Usage: gasinavif [OPTIONS] [DIRECTORY]

Arguments:
  [DIRECTORY]              Target directory to process [default: .]

Options:
      --settings           Open interactive global configuration menu (TUI)
  -f, --fresh              Reset all checkpoint state and start fresh
  -v, --verbose            Show detailed per-file processing logs
  -q, --quality <QUALITY>  AVIF encoding quality (1-100, default: 60 or config)
  -s, --speed <SPEED>      AVIF encoding speed (1-10, default: 8 or config)
      --yuv <YUV>          Chroma subsampling format: 420, 422, 444, 400 (default: 420)
  -j, --threads <THREADS>  Number of parallel worker threads (capped at CPU cores)
      --dry-run            Analyze and report without writing or deleting files
      --keep-raw           Keep RAW files even when a matching JPG already exists
  -h, --help               Print help information
  -V, --version            Print version
```

---

## Global Configuration (`gasinavif --settings`)

`gasinavif` includes an interactive Terminal User Interface (TUI) to adjust global defaults saved persistently to `~/.config/gasinavif/config.json`.

```bash
gasinavif --settings
```

### Navigation & Menus
- **Main Menu**: Navigate with `Up` / `Down` arrow keys (or `k` / `j`), press `Enter` to enter a submenu, or `Esc` / `q` to exit.
- **Speed Submenu (1-10)**: Adjust the horizontal slider with `Left` / `Right` arrows. Lower values yield maximum compression ratio; values 8-10 provide rapid encoding (~0.5s/image).
- **Quality Submenu (1-100)**: Adjust in steps of 1 (`Up` / `Down`) or steps of 10 (`PgUp` / `PgDn`) with visual percentage progress.
- **Chroma Subsampling (YUV)**:
  - `420`: Industry default; maximum compatibility and best file size reduction.
  - `422`: Horizontal subsampling; balanced color fidelity for sharp edges.
  - `444`: Full 1:1 color resolution; lossless color detail.
  - `400`: Monochrome / grayscale.
- **Auto-Save**: Changes take effect immediately across all subsequent standard runs without extra confirmation steps.
- **Resolution Priority**: CLI flags (`-q`, `-s`, `--yuv`) > Persistent Config (`config.json`) > Built-in defaults.

---

## Live Interactive Controls

While compression is executing, press the following single-key shortcuts at any time without pressing Enter:

| Key | Mode | Description |
| :---: | :--- | :--- |
| **`s`** | **Show Active Workers** | Toggles a live panel displaying current files being processed by each CPU worker. |
| **`v`** | **Toggle Verbose** | Toggles per-file completion logs (`[OK-AVIF]`, `[OK-RAW]`, etc.) on the fly. |
| **`t`** | **Cycle Threads** | Cycles active worker thread allocations between standard increments. |
| **`+`** / **`-`** | **Adjust Threads** | Increases or decreases parallel workers by 1 (strictly capped at host logical CPU count). |
| **`l`** | **Folder Tree Progress** | Toggles an indented directory tree hierarchy displaying completed/total files and status spinners per folder. |
| **`q`** | **Graceful Stop** | Flushes active tasks, clears temporary `.tmp` files, saves progress state, and exits cleanly. |

### Example Display: Folder Tree Mode (`l`)
```text
  ── Directory Hierarchy & Progress ──
  Vacation                         [⠋] 12/45
  ├── Beach                        [✔] 20/20
  └── Mountain                     [⠙]  5/15
  Studio                           [✔] 10/10
  JPG -> AVIF [00:15] [████████████████████] 47/90 (52%) | 4.8 photos/s | ETA: 00:09
  [ s:show (OFF) | l:tree (ON) | v:verbose (OFF) | t:threads (12) | q:quit ]
```

### Example Display: Active Workers Mode (`s`)
```text
  ── Active Worker Threads (12 Cores) ──
    [Core  0] .../Vacation/Beach/IMG_2024.jpg
    [Core  1] .../Vacation/Beach/IMG_2025.jpg
    [Core  2] .../Studio/portrait_01.jpg
    [Core  3] (idle)
    ...
  JPG -> AVIF [00:15] [████████████████████] 47/90 (52%) | 4.8 photos/s | ETA: 00:09
  [ s:show (ON) | l:tree (OFF) | v:verbose (OFF) | t:threads (12) | q:quit ]
```

---

## Supported Camera RAW Formats

| Manufacturer | Extensions |
|---|---|
| Canon | `.cr2`, `.cr3` |
| Sony | `.arw`, `.srf`, `.sr2` |
| Nikon | `.nef`, `.nrw` |
| Adobe (DNG) | `.dng` |
| Fujifilm | `.raf` |
| Panasonic | `.rw2` |
| Olympus | `.orf` |
| Pentax | `.pef` |

Standard `.jpg` and `.jpeg` images are processed directly.

---

## Processing Architecture

```
                    ┌─────────────────────────┐
                    │ Directory Traversal     │
                    │ (walkdir, O(N))         │
                    └────────────┬────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │ Group Files by Stem     │
                    │ (In-memory grouping)    │
                    └────────────┬────────────┘
                                 │
               ┌─────────────────┼─────────────────┐
               │                 │                 │
        RAW + JPG Exists      RAW Only          JPG Only
               │                 │                 │
         Delete RAW         Stage 1: Decode        │
               │            RAW -> JPG             │
               │                 │                 │
               └─────────────────┼─────────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │ Stage 2: AVIF Encoding  │
                    │ (avifenc SIMD / ravif)  │
                    └────────────┬────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │ Size Verification       │
                    │ AVIF < JPG: Replace     │
                    │ AVIF >= JPG: Keep JPG   │
                    └────────────┬────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │ Summary Statistics Box  │
                    └─────────────────────────┘
```

---

## Safety & Integrity Protections

1. **Anti-Loop Prevention**: Inspects files for `COMPRESSED_BY_SCRIPT=true` in EXIF UserComment and XMP metadata fields; previously processed images are skipped instantly.
2. **Size Regression Guard**: Compresses to an intermediate `.avif.tmp` file and compares sizes. If compression produces a larger file, the temporary file is deleted and the source is marked with `STATUS=SKIPPED_LARGER`.
3. **Atomic File Swapping**: Original files are only replaced after the temporary target file is fully written and verified on disk.
4. **Safe Interrupts**: Handles `SIGINT` / `SIGTERM` signals and the `q` key cleanly, removing pending temporary files and recording current progress to disk.

---

## Architecture & Crates

| Component | Library | Purpose |
|---|---|---|
| CLI Parsing | `clap` v4 | Command-line argument parsing with derive macros |
| Parallelism | `rayon` | Multi-threaded work-stealing job dispatcher |
| Terminal UI & Controls | `crossterm`, `console` | Non-blocking keyboard event loop and TUI rendering |
| Image Decoding & Fallback | `image`, `ravif` | Embedded pure-Rust raster decoder and fallback encoder |
| EXIF & Metadata | `rexiv2`, `kamadak-exif`, `img-parts` | In-process native EXIF extraction, injection, and cloning |
| State & Serialization | `serde`, `serde_json` | Checkpointing and resume progress tracking |
| Signal Handling | `ctrlc` | Interruption capturing and cleanup coordination |
| File Timestamps | `filetime` | Preserving source modification dates (`mtime`) |

---

## License

This project is licensed under the [GNU General Public License v3.0](LICENSE).
