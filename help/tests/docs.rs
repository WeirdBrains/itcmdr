//! The docs standard: every guide is in the index, every relative link resolves, every ITCMDR_*
//! variable in the source is in docs/ENVIRONMENT.md, and the CLIs' help has every guide.
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

const SKIP: &[&str] = &[
    ".git",
    "target",
    "third_party",
    "build",
    ".dart_tool",
    "node_modules",
    "dist",
    "generated",
    "historical",
    "target.usb-old",
    "proto",
    "ephemeral",
    ".cache",
    "Pods",
];

fn walk(dir: &Path, exts: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if !SKIP.contains(&name.as_str()) && !name.starts_with('.') {
                walk(&p, exts, out);
            }
        } else if p
            .extension()
            .is_some_and(|x| exts.contains(&x.to_string_lossy().as_ref()))
        {
            out.push(p);
        }
    }
}

fn guides() -> Vec<PathBuf> {
    let mut v: Vec<_> = fs::read_dir(root().join("docs"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    v.sort();
    v
}

#[test]
fn index_lists_every_guide() {
    let index = fs::read_to_string(root().join("docs/README.md")).unwrap();
    for g in guides() {
        let f = g.file_name().unwrap().to_string_lossy().to_string();
        if f != "README.md" {
            assert!(
                index.contains(&format!("({f})")),
                "docs/README.md does not link {f}"
            );
        }
    }
}

#[test]
fn relative_links_resolve() {
    let re_start = "](";
    for g in guides() {
        let text = fs::read_to_string(&g).unwrap();
        let mut rest = text.as_str();
        while let Some(i) = rest.find(re_start) {
            rest = &rest[i + 2..];
            let Some(j) = rest.find(')') else { break };
            let link = &rest[..j];
            if link.contains("://")
                || link.starts_with('#')
                || link.starts_with("mailto:")
                || link.contains(' ')
            {
                continue;
            }
            let path = link.split('#').next().unwrap();
            let target = g.parent().unwrap().join(path);
            assert!(target.exists(), "{}: broken link {link}", g.display());
        }
    }
}

#[test]
fn every_env_var_is_documented() {
    let env = fs::read_to_string(root().join("docs/ENVIRONMENT.md")).unwrap();
    let mut files = vec![];
    walk(
        &root(),
        &["rs", "ps1", "psm1", "dart", "sh", "iss", "py"],
        &mut files,
    );
    let mut missing = std::collections::BTreeSet::new();
    for f in files {
        let Ok(text) = fs::read_to_string(&f) else {
            continue;
        };
        let b = text.as_bytes();
        let mut i = 0;
        while let Some(k) = text[i..].find("ITCMDR_") {
            let s = i + k;
            let mut e = s + 7;
            while e < b.len()
                && (b[e].is_ascii_uppercase() || b[e].is_ascii_digit() || b[e] == b'_')
            {
                e += 1;
            }
            let name = &text[s..e];
            // a bare prefix (ITCMDR_ADMINSVR_) or a test-local name built at run time
            if !name.ends_with('_') && e - s > 7 && !env.contains(&format!("`{name}`")) {
                missing.insert(format!(
                    "{name} ({})",
                    f.strip_prefix(root()).unwrap_or(&f).display()
                ));
            }
            i = e;
        }
    }
    assert!(
        missing.is_empty(),
        "add to docs/ENVIRONMENT.md: {missing:?}"
    );
}

#[test]
fn help_has_every_guide() {
    for g in guides() {
        let id = g.file_stem().unwrap().to_string_lossy().to_lowercase();
        let id = if id == "readme" {
            "index".to_string()
        } else {
            id
        };
        assert_eq!(
            itcmdr_help::get(&id),
            Some(fs::read_to_string(&g).unwrap().as_str()),
            "help topic {id}"
        );
    }
    assert!(itcmdr_help::get("historical/1.0/plan").is_none());
    assert!(itcmdr_help::get("BACKUP.md").is_some());
    let (list, ok) = itcmdr_help::render("itcmdr-cli", None);
    assert!(ok && list.contains("certificates"));
    assert!(!itcmdr_help::render("itcmdr-cli", Some("no-such-word-xyz")).1);
}
