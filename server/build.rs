fn main() {
    // Ensure vendored protoc is used so CI and Windows builds work without system protoc.
    let protoc_path = protoc_bin_vendored::protoc_bin_path().expect("protoc not found");
    std::env::set_var("PROTOC", protoc_path);

    let proto_dir = std::path::Path::new("../proto");
    let out_dir = std::path::Path::new("src/generated");
    std::fs::create_dir_all(out_dir).ok();

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .out_dir(out_dir)
        .compile_protos(&[proto_dir.join("itcmdr.proto")], &[proto_dir])
        .expect("failed to compile protos");
}

