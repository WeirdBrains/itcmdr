use itcmdr_common::{ErrorCriticality, ServicePluginError};

#[derive(Debug, ServicePluginError)]
#[service_plugin(name = "disk-free", kind = "disk-full")]
pub struct DiskError {
    error: String,
    criticality: ErrorCriticality,
    backtrace: String,
    service_plugin: String,
    drive: String,
    percent: u8,
    #[service_plugin(skip)]
    internal: String,
}

#[derive(Debug, ServicePluginError)]
pub struct CustomServicePluginError {
    error: String,
    criticality: itcmdr_common::ErrorCriticality,
    backtrace: String,
    service_plugin: String,
}

#[test]
fn derive_with_name_and_context() {
    let mut e = DiskError::new("C: is 97% full", ErrorCriticality::Critical);
    e.drive = "C:".into();
    e.percent = 97;
    e.internal = "x".into();
    assert_eq!(e.to_string(), "C: is 97% full");
    let r = e.to_record();
    assert_eq!(r.service_plugin, "disk-free");
    assert_eq!(r.kind, "disk-full");
    assert_eq!(r.criticality, ErrorCriticality::Critical);
    assert_eq!(r.context.get("drive").unwrap(), "C:");
    assert_eq!(r.context.get("percent").unwrap(), "97");
    assert!(!r.context.contains_key("internal"));
    assert!(r.backtrace.contains("derive_with_name_and_context") || !r.backtrace.is_empty());
    let boxed: Box<dyn std::error::Error> = Box::new(e);
    assert_eq!(boxed.to_string(), "C: is 97% full");
}

#[test]
fn derive_without_name() {
    let e = CustomServicePluginError::new(
        "backup-check",
        "share unreachable",
        ErrorCriticality::Cautious,
    );
    let r = e.to_record();
    assert_eq!(r.service_plugin, "backup-check");
    assert_eq!(r.kind, "CustomServicePluginError");
    assert!(r.context.is_empty());
    let d: &dyn ServicePluginError = &e;
    assert_eq!(d.criticality().log_level(), "warn");
}

#[test]
fn compile_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
