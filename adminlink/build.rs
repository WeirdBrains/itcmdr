fn main() {
    std::env::set_var(
        "PROTOC",
        protoc_bin_vendored::protoc_bin_path().expect("protoc"),
    );
    println!("cargo:rerun-if-changed=../proto/admin.proto");
    tonic_build::configure()
        .compile_protos(&["../proto/admin.proto"], &["../proto"])
        .expect("admin.proto");
}
