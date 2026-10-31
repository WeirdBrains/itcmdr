use itcmdr_common::{ErrorCriticality, ServicePluginError};

#[derive(Debug, ServicePluginError)]
struct NoBacktrace {
    error: String,
    criticality: ErrorCriticality,
    service_plugin: String,
}

fn main() {}
