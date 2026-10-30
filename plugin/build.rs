fn main() {
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("protoc not found");
    std::env::set_var("PROTOC", protoc);
    println!("cargo:rerun-if-changed=../proto/plugin.proto");
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["../proto/plugin.proto"], &["../proto"])
        .expect("failed to compile plugin.proto");
}
