fn main() {
    std::env::set_var(
        "PROTOC",
        protoc_bin_vendored::protoc_bin_path().expect("protoc"),
    );
    println!("cargo:rerun-if-changed=../proto/admin.proto");
    // BTreeMap: maps encode in key order, so a re-encoded message has the bytes that were signed.
    tonic_build::configure()
        .btree_map(["."])
        .compile_protos(&["../proto/admin.proto"], &["../proto"])
        .expect("admin.proto");
}
