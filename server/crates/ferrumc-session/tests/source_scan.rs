//! Root decision NET-04.2e1 F2: `ferrumc-storage` and `ferrumc-world` are
//! test-only dependencies. No production source of this crate may mention
//! them; only `#[cfg(test)]` modules (inline or file modules) and `tests/` may.

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN: [&str; 4] = [
    "ferrumc_storage",
    "ferrumc_world",
    "ferrumc-storage",
    "ferrumc-world",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Removes every `#[cfg(test)]`-gated item: inline modules by brace depth,
/// and records file modules (`#[cfg(test)] mod x;`, optionally `#[path]`).
fn production_text(path: &Path, source: &str, test_files: &mut Vec<PathBuf>) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut kept = String::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim();
        if line == "#[cfg(test)]" {
            // Skip attribute lines up to the gated item.
            let mut item = index + 1;
            let mut explicit_path = None;
            while item < lines.len() && lines[item].trim().starts_with("#[") {
                let attr = lines[item].trim();
                if let Some(rest) = attr.strip_prefix("#[path = \"") {
                    explicit_path = rest.strip_suffix("\"]").map(str::to_owned);
                }
                item += 1;
            }
            let gated = lines.get(item).map_or("", |l| l.trim());
            if let Some(name) = gated.strip_prefix("mod ").and_then(|r| r.strip_suffix(';')) {
                let dir = path.parent().unwrap();
                let stem = path.file_stem().unwrap().to_str().unwrap();
                let module_dir = if matches!(stem, "lib" | "mod") {
                    dir.to_path_buf()
                } else {
                    dir.join(stem)
                };
                let file = match explicit_path {
                    Some(explicit) => dir.join(explicit),
                    None => module_dir.join(format!("{name}.rs")),
                };
                test_files.push(file);
                index = item + 1;
                continue;
            }
            // Inline gated item: skip until its braces balance.
            let mut depth = 0_i32;
            let mut opened = false;
            let mut cursor = item;
            while cursor < lines.len() {
                for ch in lines[cursor].chars() {
                    match ch {
                        '{' => {
                            depth += 1;
                            opened = true;
                        }
                        '}' => depth -= 1,
                        _ => {}
                    }
                }
                cursor += 1;
                if opened && depth == 0 {
                    break;
                }
                if !opened && lines[cursor - 1].trim_end().ends_with(';') {
                    break;
                }
            }
            index = cursor;
            continue;
        }
        kept.push_str(lines[index]);
        kept.push('\n');
        index += 1;
    }
    kept
}

#[test]
fn production_sources_never_mention_test_only_storage_or_world_crates() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    files.sort();
    let mut test_files = Vec::new();
    let mut texts = Vec::new();
    for file in &files {
        let source = fs::read_to_string(file).unwrap();
        texts.push((
            file.clone(),
            production_text(file, &source, &mut test_files),
        ));
    }
    let mut scanned = 0;
    for (file, text) in texts {
        if test_files.contains(&file) {
            continue;
        }
        scanned += 1;
        for needle in FORBIDDEN {
            assert!(
                !text.contains(needle),
                "{} mentions {needle} outside #[cfg(test)]",
                file.display()
            );
        }
    }
    assert!(scanned >= 15, "scanned {scanned} production files");
    // The internal NET-04.2e1 test module is recognized as test-only.
    assert!(test_files
        .iter()
        .any(|f| f.ends_with("router/native_tests.rs")));
}

#[test]
fn the_scanner_detects_a_production_mention_and_skips_gated_items() {
    let mut test_files = Vec::new();
    let path = Path::new("/virtual/src/lib.rs");
    let leaked = "use ferrumc_storage::GenerationKey;\n";
    assert!(production_text(path, leaked, &mut test_files).contains("ferrumc_storage"));
    let gated = "fn a() {}\n#[cfg(test)]\nmod tests {\n    use ferrumc_world::WorldSeed;\n    fn b() { if true { } }\n}\nfn c() {}\n";
    let kept = production_text(path, gated, &mut test_files);
    assert!(!kept.contains("ferrumc_world"));
    assert!(kept.contains("fn c()"));
    let file_module = "#[cfg(test)]\n#[path = \"x_tests.rs\"]\nmod tests;\n";
    production_text(
        Path::new("/virtual/src/router/native.rs"),
        file_module,
        &mut test_files,
    );
    assert!(test_files.contains(&PathBuf::from("/virtual/src/router/x_tests.rs")));
}
