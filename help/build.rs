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
    // v1.3.1 release guard: the build stamp (version, git commit, help hash) that
    // scripts/check_stamps.py finds in every release asset. Same hash as scripts/gen-help.py.
    let mut h: u64 = 0xcbf29ce484222325;
    let mut feed = |b: &[u8]| {
        for x in b {
            h ^= *x as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    };
    let mut ids: Vec<(String, PathBuf)> = files
        .iter()
        .map(|f| {
            let rel = f.strip_prefix(&docs).unwrap().with_extension("");
            let id = rel.to_string_lossy().replace('\\', "/").to_lowercase();
            (if id == "readme" { "index".into() } else { id }, f.clone())
        })
        .collect();
    ids.sort();
    for (id, f) in &ids {
        feed(id.as_bytes());
        feed(&[0]);
        feed(&fs::read(f).unwrap());
        feed(&[0]);
    }
    println!("cargo:rerun-if-env-changed=ITCMDR_BUILD_COMMIT");
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("..");
    for g in [".git/HEAD", ".git/refs/heads", ".git/packed-refs"] {
        let p = root.join(g);
        if p.exists() {
            println!("cargo:rerun-if-changed={}", p.display());
        }
    }
    let commit = std::env::var("ITCMDR_BUILD_COMMIT")
        .ok()
        .filter(|c| !c.trim().is_empty())
        .or_else(|| {
            std::process::Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(["rev-parse", "HEAD"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        })
        .unwrap_or_else(|| "unknown".into());
    src.push_str(&format!(
        "/// Release guard stamp (scripts/check_stamps.py).\npub const BUILD_STAMP: &str = \"ITCMDR-BUILD v={} commit={} help={h:016x} END\";\n",
        std::env::var("CARGO_PKG_VERSION").unwrap(),
        commit.trim()
    ));
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("help_topics.rs");
    fs::write(out, src).unwrap();
}
