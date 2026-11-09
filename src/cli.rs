use clap::Parser;
use std::path::PathBuf;

/// gasinavif — High-performance camera photo compressor (RAW → JPG → AVIF)
#[derive(Parser, Debug)]
#[command(name = "gasinavif", version, about, long_about = None)]
pub struct Args {
    /// Target directory to process (default: current directory)
    #[arg(default_value = ".")]
    pub directory: PathBuf,

    /// Buka menu pengaturan interaktif (TUI Settings)
    #[arg(long)]
    pub settings: bool,

    /// Reset all checkpoint state and start fresh
    #[arg(short, long)]
    pub fresh: bool,

    /// Show detailed per-file processing logs
    #[arg(short, long)]
    pub verbose: bool,

    /// AVIF encoding quality (1-100, lower = smaller file)
    #[arg(short, long)]
    pub quality: Option<u8>,

    /// AVIF encoding speed (1-10, higher = faster but larger)
    #[arg(short, long)]
    pub speed: Option<u8>,

    /// Format subsampling chroma AVIF (420, 422, 444, 400)
    #[arg(long)]
    pub yuv: Option<String>,

    /// Number of parallel worker threads (default: number of CPU cores)
    #[arg(short = 'j', long)]
    pub threads: Option<usize>,

    /// Dry-run mode: analyze and report without writing or deleting files
    #[arg(long)]
    pub dry_run: bool,

    /// Keep RAW files even when a matching JPG already exists
    #[arg(long)]
    pub keep_raw: bool,
}
