//! Embed every docs/**/*.md (except docs/historical/) as a help topic: one source for docs and help.
use std::fs;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == "historical") {
                continue;
            }
            walk(&p, out);
        } else if p.extension().is_some_and(|e| e == "md") {
            out.push(p);
        }
    }
}

fn main() {
    let docs = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../docs");
    println!("cargo:rerun-if-changed={}", docs.display());
    let mut files = vec![];
    walk(&docs, &mut files);
    let mut src = String::from("pub const TOPICS: &[(&str, &str)] = &[\n");
    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
        let rel = f.strip_prefix(&docs).unwrap().with_extension("");
        let id = rel.to_string_lossy().replace('\\', "/").to_lowercase();
        let id = if id == "readme" {
            "index".to_string()
        } else {
            id
        };
        src.push_str(&format!(
            "    ({id:?}, include_str!({:?})),\n",
            f.display().to_string()
        ));
    }
    src.push_str("];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("help_topics.rs");
    fs::write(out, src).unwrap();
}
