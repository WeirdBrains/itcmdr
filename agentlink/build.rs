fn main() {
    std::env::set_var(
        "PROTOC",
        protoc_bin_vendored::protoc_bin_path().expect("protoc"),
    );
    println!("cargo:rerun-if-changed=../proto/agent.proto");
    tonic_build::configure()
        .compile_protos(&["../proto/agent.proto"], &["../proto"])
        .expect("agent.proto");
}
