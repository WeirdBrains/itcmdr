use itcmdr_common::ServicePluginError;

#[derive(Debug, ServicePluginError)]
struct WrongType {
    error: String,
    criticality: String,
    backtrace: String,
    service_plugin: String,
}

fn main() {}
