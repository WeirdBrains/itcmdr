//! v0.15.0: a Rust plugin returns a derived ServicePluginError; it round-trips through the proto
//! and the server-side log line validates.
use itcmdr_plugin::{proto, ErrorCriticality, ServicePluginError};

#[derive(Debug, ServicePluginError)]
#[service_plugin(name = "disk-free")]
struct DiskFull {
    error: String,
    criticality: ErrorCriticality,
    backtrace: String,
    service_plugin: String,
    drive: String,
}

#[test]
fn plugin_error_round_trip_and_log_line() {
    let mut e = DiskFull::new("C: is 97% full", ErrorCriticality::Critical);
    e.drive = "C:".into();
    let resp = itcmdr_plugin::plugin_error(&e);
    assert!(!resp.ok);
    assert_eq!(resp.message, "C: is 97% full");
    let pe = resp.error.clone().unwrap();
    assert_eq!(pe.criticality, "critical");
    assert_eq!(pe.kind, "DiskFull");
    assert_eq!(pe.context.get("drive").map(String::as_str), Some("C:"));
    let bytes = prost::Message::encode_to_vec(&resp);
    let back: proto::StageResponse = prost::Message::decode(bytes.as_slice()).unwrap();
    let rec = back.error.unwrap().to_record(ErrorCriticality::Cautious);
    assert_eq!(rec, e.to_record());
    let odd = proto::PluginError {
        criticality: "fatal".into(),
        ..Default::default()
    };
    assert_eq!(
        odd.to_record(ErrorCriticality::Informational).criticality,
        ErrorCriticality::Informational
    );
    let dir = tempfile::tempdir().unwrap();
    let log = itcmdr_plugin::log::Logger {
        path: dir.path().join("x.jsonl"),
        plugin: "disk-free".into(),
        plugin_version: "1.0.0".into(),
        run_id: "r".into(),
        machine: "SERVER01".into(),
        stage: "execute".into(),
    };
    log.plugin_error(&rec).unwrap();
    let line = std::fs::read_to_string(dir.path().join("x.jsonl")).unwrap();
    let m = itcmdr_plugin::log::validate(line.trim()).unwrap();
    assert_eq!(m["event"], "plugin-error");
    assert_eq!(m["level"], "error");
    assert_eq!(m["criticality"], "critical");
    assert_eq!(m["context"]["drive"], "C:");
    let bad = line.replace("\"criticality\":\"critical\",", "");
    assert_ne!(bad, line);
    assert!(itcmdr_plugin::log::validate(bad.trim()).is_err());
}
