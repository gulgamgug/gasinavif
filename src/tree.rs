//! tree.rs — Hierarki struktur folder ala `tree` dengan live progress counter dan status spinner.
//!
//! Menampilkan hanya folder (tanpa nama file) dengan indentasi box-drawing (`├── `, `└── `),
//! counter progres per folder (file langsung tanpa subfolder), dan ikon spinner / centang hijau.

use console::style;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Representasi satu node folder dalam struktur hierarki pohon.
#[derive(Debug)]
pub struct FolderNode {
    pub name: String,
    pub path: PathBuf,
    /// Jumlah file yang berada LANGSUNG di folder ini (tidak termasuk subfolder).
    pub direct_total: usize,
    /// Jumlah file di folder ini yang sudah selesai diproses.
    pub direct_completed: Arc<AtomicUsize>,
    pub children: Vec<FolderNode>,
}

impl FolderNode {
    /// Mencatat bahwa 1 file di path `file_path` telah selesai diproses.
    pub fn record_file_completed(&self, file_dir: &Path) -> bool {
        if self.path == file_dir {
            self.direct_completed.fetch_add(1, Ordering::Relaxed);
            return true;
        }

        for child in &self.children {
            if child.record_file_completed(file_dir) {
                return true;
            }
        }

        false
    }

    /// Merender seluruh hierarki pohon menjadi baris-baris teks terminal.
    pub fn render(&self, spinner_char: char) -> Vec<String> {
        let mut lines = Vec::new();
        self.render_internal("", true, true, spinner_char, &mut lines);
        lines
    }

    fn render_internal(
        &self,
        prefix: &str,
        is_last: bool,
        is_root: bool,
        spinner_char: char,
        lines: &mut Vec<String>,
    ) {
        let done = self.direct_completed.load(Ordering::Relaxed);
        let total = self.direct_total;

        // Render status icon: centang hijau jika selesai, spinner jika masih berjalan
        let status_str = if total == 0 {
            style("✔".to_string()).green().bold().to_string()
        } else if done >= total {
            style("✔".to_string()).green().bold().to_string()
        } else {
            style(spinner_char.to_string()).cyan().bold().to_string()
        };

        let progress_str = format!("{done}/{total}");

        let line = if is_root {
            format!("{:<32} {} {}", style(&self.name).bold(), status_str, progress_str)
        } else {
            let connector = if is_last { "└── " } else { "├── " };
            let full_prefix = format!("{prefix}{connector}");
            let label = format!("{full_prefix}{}", self.name);
            format!("{:<32} {} {}", label, status_str, progress_str)
        };

        lines.push(line);

        let child_count = self.children.len();
        for (i, child) in self.children.iter().enumerate() {
            let child_is_last = i == child_count - 1;
            let next_prefix = if is_root {
                ""
            } else if is_last {
                &format!("{prefix}    ")
            } else {
                &format!("{prefix}│   ")
            };

            child.render_internal(next_prefix, child_is_last, false, spinner_char, lines);
        }
    }
}

/// Membangun hierarki FolderNode dari daftar file yang akan diproses.
pub fn build_folder_tree(root_dir: &Path, files: &[PathBuf]) -> FolderNode {
    // 1. Hitung direct_total per direktori
    let mut dir_counts: HashMap<PathBuf, usize> = HashMap::new();
    for file in files {
        let parent = file.parent().unwrap_or(root_dir).to_path_buf();
        *dir_counts.entry(parent).or_insert(0) += 1;
    }

    // Pastikan root_dir ada dalam map
    dir_counts.entry(root_dir.to_path_buf()).or_insert(0);

    // 2. Kumpulkan semua direktori (termasuk intermediate dirs)
    let mut all_dirs: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for dir in dir_counts.keys() {
        let mut curr = dir.as_path();
        while curr != root_dir && curr.starts_with(root_dir) {
            if let Some(parent) = curr.parent() {
                let entry = all_dirs.entry(parent.to_path_buf()).or_default();
                let curr_buf = curr.to_path_buf();
                if !entry.contains(&curr_buf) {
                    entry.push(curr_buf);
                }
                curr = parent;
            } else {
                break;
            }
        }
    }

    // 3. Bangun pohon secara rekursif
    fn build_node(
        dir: &Path,
        root_dir: &Path,
        dir_counts: &HashMap<PathBuf, usize>,
        all_dirs: &HashMap<PathBuf, Vec<PathBuf>>,
    ) -> FolderNode {
        let name = if dir == root_dir {
            dir.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Root")
                .to_string()
        } else {
            dir.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("folder")
                .to_string()
        };

        let direct_total = dir_counts.get(dir).copied().unwrap_or(0);

        let mut children = Vec::new();
        if let Some(subdirs) = all_dirs.get(dir) {
            let mut sorted_subdirs = subdirs.clone();
            sorted_subdirs.sort();
            for sub in sorted_subdirs {
                children.push(build_node(&sub, root_dir, dir_counts, all_dirs));
            }
        }

        FolderNode {
            name,
            path: dir.to_path_buf(),
            direct_total,
            direct_completed: Arc::new(AtomicUsize::new(0)),
            children,
        }
    }

    build_node(root_dir, root_dir, &dir_counts, &all_dirs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_construction_and_render() {
        let root = Path::new("/photos");
        let files = vec![
            PathBuf::from("/photos/pic1.jpg"),
            PathBuf::from("/photos/pic2.jpg"),
            PathBuf::from("/photos/vacation/sea.jpg"),
            PathBuf::from("/photos/vacation/mountain.jpg"),
            PathBuf::from("/photos/vacation/hotel/room.jpg"),
            PathBuf::from("/photos/studio/model.jpg"),
        ];

        let tree = build_folder_tree(root, &files);

        assert_eq!(tree.name, "photos");
        assert_eq!(tree.direct_total, 2); // pic1, pic2
        assert_eq!(tree.children.len(), 2); // studio, vacation

        // Uji rekam penyelesaian file
        assert!(tree.record_file_completed(Path::new("/photos")));
        assert_eq!(tree.direct_completed.load(Ordering::Relaxed), 1);

        assert!(tree.record_file_completed(Path::new("/photos/vacation")));
        assert!(tree.record_file_completed(Path::new("/photos/vacation/hotel")));

        let lines = tree.render('⠋');
        assert!(lines.len() >= 4);

        // Pastikan format pohon dan karakter indentasi valid
        let joined = lines.join("\n");
        assert!(joined.contains("photos"));
        assert!(joined.contains("studio"));
        assert!(joined.contains("vacation"));
        assert!(joined.contains("hotel"));
    }
}
