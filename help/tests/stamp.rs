//! v1.3.1 release guard: the build stamp's format and help hash match scripts/check_stamps.py.
use std::process::Command;

#[test]
fn stamp_has_version_commit_and_help_hash() {
    let s = itcmdr_help::BUILD_STAMP;
    assert!(
        s.starts_with(&format!(
            "ITCMDR-BUILD v={} commit=",
            env!("CARGO_PKG_VERSION")
        )),
        "{s}"
    );
    assert!(s.ends_with(" END"), "{s}");
    itcmdr_help::keep_stamp();
}

#[test]
fn help_hash_matches_the_release_check() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
    let Ok(o) = Command::new("python3")
        .arg(format!("{root}/scripts/check_stamps.py"))
        .arg("--help-hash")
        .output()
    else {
        eprintln!("python3 missing: skipped");
        return;
    };
    let py = String::from_utf8_lossy(&o.stdout).trim().to_string();
    assert!(
        itcmdr_help::BUILD_STAMP.contains(&format!(" help={py} END")),
        "{} vs {py}",
        itcmdr_help::BUILD_STAMP
    );
}

#[test]
fn release_guard_script_tests_pass() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
    let Ok(o) = Command::new("python3")
        .arg(format!("{root}/scripts/test_check_stamps.py"))
        .output()
    else {
        return;
    };
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}
