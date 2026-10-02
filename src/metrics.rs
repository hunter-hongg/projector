use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::detect::{scan_dir_for_extensions, walk_dirs};

pub fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let bytes_f = bytes as f64;
    let unit_idx = (bytes_f.log10() / 3.0).floor() as usize;
    let unit_idx = unit_idx.min(UNITS.len() - 1);
    let value = bytes_f / (1024u64.pow(unit_idx as u32) as f64);
    if unit_idx == 0 {
        format!("{} {}", value as u64, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", value, UNITS[unit_idx])
    }
}

pub fn calc_dir_size(dir: &Path, skip_hidden: bool) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if skip_hidden && name.starts_with('.') {
                    continue;
                }
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file()
                    && let Ok(meta) = fs::metadata(&path)
                {
                    total += meta.len();
                }
            }
        }
    }
    total
}

const COUNTABLE_EXTENSIONS: &[&str] = &[
    "rs", "js", "ts", "jsx", "tsx", "go", "py", "java", "kt", "kts", "c", "h", "cpp", "hpp", "cc",
    "cxx", "ml", "mli", "dart", "toml", "json", "yaml", "yml", "md", "css", "html",
];

pub fn estimate_loc(dir: &Path) -> u32 {
    let mut total = 0u32;
    walk_dirs(dir, |path| {
        if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && COUNTABLE_EXTENSIONS.contains(&ext)
            && let Ok(content) = fs::read_to_string(path)
        {
            total += content.lines().count() as u32;
        }
    });
    total
}

pub struct FileTypeDistribution {
    pub groups: Vec<(String, u32, u32)>,
}

pub fn file_type_distribution(dir: &Path) -> FileTypeDistribution {
    let mut ext_counts: HashMap<String, u32> = HashMap::new();
    scan_dir_for_extensions(dir, &mut ext_counts);

    let total_files: u32 = ext_counts.values().sum();

    let type_groups: Vec<(&str, Vec<&str>)> = vec![
        ("Rust", vec!["rs"]),
        ("JavaScript/TypeScript", vec!["js", "ts", "jsx", "tsx"]),
        ("Go", vec!["go"]),
        ("Python", vec!["py"]),
        ("Java/Kotlin", vec!["java", "kt", "kts"]),
        ("C/C++", vec!["c", "h", "cpp", "hpp", "cc", "cxx"]),
        ("OCaml", vec!["ml", "mli"]),
        ("Dart", vec!["dart"]),
        ("Data/Config", vec!["json", "yaml", "yml", "toml"]),
        ("Markdown", vec!["md"]),
        ("Web", vec!["css", "html"]),
        ("Other", vec![]),
    ];

    let mut groups: Vec<(String, u32, u32)> = Vec::new();
    let mut accounted: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (group_name, exts) in &type_groups {
        if exts.is_empty() {
            continue;
        }
        let mut count = 0u32;
        for ext in exts {
            if let Some(&c) = ext_counts.get(*ext) {
                count += c;
                accounted.insert(ext.to_string());
            }
        }
        if count > 0 {
            let pct = if total_files > 0 {
                (count as f64 / total_files as f64 * 100.0).round() as u32
            } else {
                0
            };
            groups.push((group_name.to_string(), count, pct));
        }
    }

    let other_count: u32 = ext_counts
        .iter()
        .filter(|(k, _)| !accounted.contains(*k))
        .map(|(_, v)| v)
        .sum();
    if other_count > 0 {
        let pct = if total_files > 0 {
            (other_count as f64 / total_files as f64 * 100.0).round() as u32
        } else {
            0
        };
        groups.push(("Other".to_string(), other_count, pct));
    }

    groups.sort_by_key(|g| std::cmp::Reverse(g.1));

    FileTypeDistribution { groups }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_loc_iterative() {
        let dir = std::env::temp_dir().join("projector_test_loc");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("a.rs"), "line1\nline2\n").unwrap();
        std::fs::write(dir.join("b.py"), "x\n").unwrap();
        assert_eq!(estimate_loc(&dir), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_file_type_distribution_empty_dir() {
        let dir = std::env::temp_dir().join("projector_test_ftd_empty");
        let _ = std::fs::create_dir_all(&dir);
        let ftd = file_type_distribution(&dir);
        assert!(ftd.groups.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_file_type_distribution_with_files() {
        let dir = std::env::temp_dir().join("projector_test_ftd");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("main.rs"), "").unwrap();
        std::fs::write(dir.join("lib.rs"), "").unwrap();
        std::fs::write(dir.join("style.css"), "").unwrap();
        let ftd = file_type_distribution(&dir);
        assert!(!ftd.groups.is_empty());
        let rust_count = ftd
            .groups
            .iter()
            .find(|(name, _, _)| name == "Rust")
            .map(|(_, count, _)| *count)
            .unwrap_or(0);
        assert_eq!(rust_count, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_human_size_bytes() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(500), "500 B");
    }

    #[test]
    fn test_human_size_kb() {
        assert_eq!(human_size(1_024), "1.0 KB");
        assert_eq!(human_size(12_345), "12.1 KB");
    }

    #[test]
    fn test_human_size_mb() {
        assert_eq!(human_size(1_048_576), "1.0 MB");
        assert_eq!(human_size(3_500_000), "3.3 MB");
    }

    #[test]
    fn test_human_size_gb() {
        assert_eq!(human_size(1_073_741_824), "1.0 GB");
    }

    #[test]
    fn test_calc_dir_size_empty() {
        let dir = std::env::temp_dir().join("projector_test_size_empty");
        let _ = std::fs::create_dir_all(&dir);
        let size = calc_dir_size(&dir, false);
        assert_eq!(size, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_calc_dir_size_with_files() {
        let dir = std::env::temp_dir().join("projector_test_size_files");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("a.txt"), "hello").unwrap();
        std::fs::write(dir.join("b.txt"), "world!").unwrap();
        let size = calc_dir_size(&dir, false);
        assert!(size > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_calc_dir_size_deep_skips_hidden() {
        let dir = std::env::temp_dir().join("projector_test_size_deep");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::create_dir_all(dir.join(".hidden"));
        std::fs::write(dir.join(".hidden").join("big.bin"), vec![0u8; 10_000]).unwrap();
        let size_skip_hidden = calc_dir_size(&dir, true);
        let size_include_hidden = calc_dir_size(&dir, false);
        assert!(size_include_hidden > size_skip_hidden);
        assert!(size_skip_hidden == 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
