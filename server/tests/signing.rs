//! Canonical protobuf encoding for signed requests (docs/SIGNING.md).
//! The pinned bytes in proto/signing_vectors.json are shared with the app: the Dart test (v0.12.0)
//! must produce exactly the same bytes. Regenerate only on purpose: ITCMDR_REGEN_VECTORS=1.
use ::itcmdr_server as srv;
use prost::Message;
use srv::generated::itcmdr::*;

fn vectors() -> Vec<(&'static str, &'static str, Vec<u8>)> {
    let e = |m: &dyn Fn() -> Vec<u8>| m();
    vec![
        (
            "ListMachines/empty",
            "/itcmdr.Itcmdr/ListMachines",
            e(&|| ListMachinesRequest {}.encode_to_vec()),
        ),
        (
            "TriggerRuns/typical",
            "/itcmdr.Itcmdr/TriggerRuns",
            e(&|| {
                TriggerRunsRequest {
                    machines: vec!["WS-01".into(), "WS-02".into()],
                    report_only: true,
                    categories: vec!["network".into(), "services".into()],
                    ..Default::default()
                }
                .encode_to_vec()
            }),
        ),
        (
            "TriggerRuns/defaults-omitted",
            "/itcmdr.Itcmdr/TriggerRuns",
            e(&|| {
                TriggerRunsRequest {
                    report_only: false,
                    ..Default::default()
                }
                .encode_to_vec()
            }),
        ),
        (
            "SaveException/nested-unicode",
            "/itcmdr.Itcmdr/SaveException",
            e(&|| {
                SaveExceptionRequest {
                    rule: Some(ExceptionRule {
                        check_id: "NET-DNS".into(),
                        action: "downgrade:warn".into(),
                        reason: "Praxis-Drucker – known".into(),
                        expires: "2026-12-31".into(),
                        ..Default::default()
                    }),
                }
                .encode_to_vec()
            }),
        ),
        (
            "SearchLogs/ints",
            "/itcmdr.Itcmdr/SearchLogs",
            e(&|| {
                SearchLogsRequest {
                    query: "timeout".into(),
                    start_ts: 1_790_000_000_000,
                    end_ts: -1,
                    limit: 100,
                    offset: 300,
                    ..Default::default()
                }
                .encode_to_vec()
            }),
        ),
        (
            "VerifyBackup/name",
            "/itcmdr.Itcmdr/VerifyBackup",
            e(&|| {
                VerifyBackupRequest {
                    name: "itcmdr-20261030-020000000-nightly.itcb".into(),
                }
                .encode_to_vec()
            }),
        ),
        (
            "SaveSchedule/typical",
            "/itcmdr.Itcmdr/CreateSchedule",
            e(&|| {
                SaveScheduleRequest {
                    schedule: Some(Schedule {
                        name: "Evening scan".into(),
                        enabled: true,
                        machines: vec!["WS-01".into()],
                        groups: vec!["Workstation".into()],
                        categories: vec!["network".into()],
                        when: "weekdays 18:30".into(),
                        timezone: "local".into(),
                        guard: "none".into(),
                        missed: "skip".into(),
                        ..Default::default()
                    }),
                }
                .encode_to_vec()
            }),
        ),
        (
            "StageRestore/sealed-dry-run",
            "/itcmdr.Itcmdr/StageRestore",
            e(&|| {
                StageRestoreRequest {
                    backup: "itcmdr-20261030-020000000-nightly.itcb".into(),
                    sealed_words: Some(SealedWords {
                        key_id: "k1".into(),
                        eph: "ab".into(),
                        nonce: "cd".into(),
                        ct: "ef".into(),
                    }),
                    dry_run: true,
                    ..Default::default()
                }
                .encode_to_vec()
            }),
        ),
        (
            "UpdateSetting/override",
            "/itcmdr.Itcmdr/UpdateSetting",
            e(&|| {
                UpdateSettingRequest {
                    key: "agent.nightlyTime".into(),
                    value: "01:30".into(),
                    machine: "WS-01".into(),
                    unset: false,
                }
                .encode_to_vec()
            }),
        ),
    ]
}

/// Fixed-key vectors for the app (Dart): an Ed25519 request signature and a sealed-words blob.
#[test]
fn app_crypto_vectors() {
    use ed25519_dalek::Signer;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../proto/app_vectors.json");
    let seed = [7u8; 32];
    let key = ed25519_dalek::SigningKey::from_bytes(&seed);
    let body = TriggerRunsRequest {
        machines: vec!["WS-01".into()],
        ..Default::default()
    }
    .encode_to_vec();
    let msg = srv::keys::request_message(
        "/itcmdr.Itcmdr/TriggerRuns",
        1_790_000_000_000,
        "0123456789abcdef",
        &body,
    );
    let sig = key.sign(&msg);
    let server_secret = x25519_dalek::StaticSecret::from([9u8; 32]);
    let server_pub = x25519_dalek::PublicKey::from(&server_secret);
    let words =
        "abandon ability able about above absent absorb abstract absurd abuse access accident";
    let sealed = srv::restore_seal::seal_with(
        server_pub.as_bytes(),
        "key-1",
        "itcmdr-20261030-020000000-nightly.itcb",
        words,
        [5u8; 32],
        [3u8; 24],
    )
    .unwrap();
    // The server opens what the app sealed.
    assert_eq!(
        srv::restore_seal::open(
            &server_secret,
            &sealed,
            "itcmdr-20261030-020000000-nightly.itcb"
        )
        .unwrap()
        .as_str(),
        words
    );
    let now = serde_json::json!({
        "ed25519": {
            "seed_hex": hex::encode(seed),
            "public_hex": hex::encode(key.verifying_key().as_bytes()),
            "fingerprint": srv::keys::fingerprint(&key.verifying_key()),
            "method": "/itcmdr.Itcmdr/TriggerRuns",
            "ts": 1_790_000_000_000i64,
            "nonce": "0123456789abcdef",
            "body_hex": hex::encode(&body),
            "sig_hex": hex::encode(sig.to_bytes()),
        },
        "sealed_words": {
            "server_secret_hex": hex::encode([9u8; 32]),
            "server_public_hex": hex::encode(server_pub.as_bytes()),
            "eph_secret_hex": hex::encode([5u8; 32]),
            "nonce_hex": hex::encode([3u8; 24]),
            "key_id": "key-1",
            "backup": "itcmdr-20261030-020000000-nightly.itcb",
            "words": words,
            "eph": sealed.eph,
            "ct": sealed.ct,
        }
    });
    if std::env::var("ITCMDR_REGEN_VECTORS").is_ok() {
        std::fs::write(&path, serde_json::to_string_pretty(&now).unwrap() + "\n").unwrap();
    }
    let pinned: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(pinned, now, "app crypto vectors changed");
}

#[test]
fn encodings_match_the_pinned_vectors() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../proto/signing_vectors.json");
    let now: Vec<serde_json::Value> = vectors()
        .into_iter()
        .map(|(n, m, b)| {
            let msg = srv::keys::request_message(m, 1_790_000_000_000, "0123456789abcdef", &b);
            serde_json::json!({"name": n, "method": m, "body_hex": hex::encode(&b),
                "signed_message_ts": 1_790_000_000_000i64, "signed_message_nonce": "0123456789abcdef",
                "signed_message": String::from_utf8(msg).unwrap()})
        })
        .collect();
    if std::env::var("ITCMDR_REGEN_VECTORS").is_ok() {
        std::fs::write(&path, serde_json::to_string_pretty(&now).unwrap() + "\n").unwrap();
    }
    let pinned: Vec<serde_json::Value> =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        pinned, now,
        "protobuf encoding or the signed message changed: clients would fail to verify"
    );
}

#[test]
fn reencoding_is_stable() {
    // The server signs over its own re-encoding; decode -> encode must give the client's bytes.
    for (n, _, b) in vectors() {
        let back = match n.split('/').next().unwrap() {
            "TriggerRuns" => TriggerRunsRequest::decode(&b[..]).unwrap().encode_to_vec(),
            "SaveException" => SaveExceptionRequest::decode(&b[..])
                .unwrap()
                .encode_to_vec(),
            "SearchLogs" => SearchLogsRequest::decode(&b[..]).unwrap().encode_to_vec(),
            "VerifyBackup" => VerifyBackupRequest::decode(&b[..]).unwrap().encode_to_vec(),
            "SaveSchedule" => SaveScheduleRequest::decode(&b[..]).unwrap().encode_to_vec(),
            "StageRestore" => StageRestoreRequest::decode(&b[..]).unwrap().encode_to_vec(),
            "UpdateSetting" => UpdateSettingRequest::decode(&b[..])
                .unwrap()
                .encode_to_vec(),
            _ => ListMachinesRequest::decode(&b[..]).unwrap().encode_to_vec(),
        };
        assert_eq!(back, b, "{n}");
    }
    // Non-canonical input (fields out of order) is re-encoded canonically: such a client fails.
    let canonical = TriggerRunsRequest {
        machines: vec!["WS-01".into()],
        report_only: true,
        ..Default::default()
    }
    .encode_to_vec();
    let mut swapped = canonical[canonical.len() - 2..].to_vec();
    swapped.extend(&canonical[..canonical.len() - 2]);
    assert_ne!(
        TriggerRunsRequest::decode(&swapped[..])
            .unwrap()
            .encode_to_vec(),
        swapped
    );
}
