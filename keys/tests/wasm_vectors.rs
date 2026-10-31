//! The same pinned vectors under WASM (`wasm-pack test --node keys`). Embedded: no filesystem.
#![cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use itcmdr_keys as k;
use wasm_bindgen_test::wasm_bindgen_test;

const APP: &str = include_str!("../../proto/app_vectors.json");
const REQ: &str = include_str!("../../proto/signing_vectors.json");

fn h(v: &serde_json::Value) -> Vec<u8> {
    hex::decode(v.as_str().unwrap()).unwrap()
}

#[wasm_bindgen_test]
fn vectors_match_in_wasm() {
    let req: serde_json::Value = serde_json::from_str(REQ).unwrap();
    for v in req.as_array().unwrap() {
        let m = k::request_message(
            v["method"].as_str().unwrap(),
            v["signed_message_ts"].as_i64().unwrap(),
            v["signed_message_nonce"].as_str().unwrap(),
            &h(&v["body_hex"]),
        );
        assert_eq!(String::from_utf8(m).unwrap(), v["signed_message"]);
    }
    let app: serde_json::Value = serde_json::from_str(APP).unwrap();
    let e = &app["ed25519"];
    let key = k::key_from_seed(&h(&e["seed_hex"])).unwrap();
    let md = k::sign_request_at(
        &key,
        e["method"].as_str().unwrap(),
        &h(&e["body_hex"]),
        e["ts"].as_i64().unwrap(),
        e["nonce"].as_str().unwrap(),
    );
    assert_eq!(md[3].1, e["sig_hex"].as_str().unwrap());
    let s = &app["sealed_words"];
    let out = k::restore::seal_with(
        &h(&s["server_public_hex"]).try_into().unwrap(),
        s["key_id"].as_str().unwrap(),
        s["backup"].as_str().unwrap(),
        s["words"].as_str().unwrap(),
        h(&s["eph_secret_hex"]).try_into().unwrap(),
        h(&s["nonce_hex"]).try_into().unwrap(),
    )
    .unwrap();
    assert_eq!(out.ct, s["ct"]);
    // Randomness works in the browser/node (getrandom js).
    assert_ne!(k::random_seed().to_vec(), k::random_seed().to_vec());
    assert!(k::now_ms() > 1_700_000_000_000);
}
