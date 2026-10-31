use itcmdr_common::ServicePluginError;

#[derive(Debug, ServicePluginError)]
enum NotAStruct {
    A,
}

fn main() {}
