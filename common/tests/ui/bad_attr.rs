use itcmdr_common::{ErrorCriticality, ServicePluginError};

#[derive(Debug, ServicePluginError)]
#[service_plugin(plugin = "x")]
struct BadAttr {
    error: String,
    criticality: ErrorCriticality,
    backtrace: String,
    service_plugin: String,
}

fn main() {}
