//! Parity with the pinned vectors in proto/ (shared with the server and the app). Never edit them.
use itcmdr_keys as k;

fn json(name: &str) -> serde_json::Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../proto")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn h(v: &serde_json::Value) -> Vec<u8> {
    hex::decode(v.as_str().unwrap()).unwrap()
}

#[test]
fn request_messages_match_signing_vectors() {
    let vs = json("signing_vectors.json");
    let vs = vs.as_array().unwrap();
    assert!(!vs.is_empty());
    for v in vs {
        let msg = k::request_message(
            v["method"].as_str().unwrap(),
            v["signed_message_ts"].as_i64().unwrap(),
            v["signed_message_nonce"].as_str().unwrap(),
            &h(&v["body_hex"]),
        );
        assert_eq!(
            String::from_utf8(msg).unwrap(),
            v["signed_message"].as_str().unwrap(),
            "{}",
            v["name"]
        );
    }
}

#[test]
fn ed25519_vector() {
    let e = &json("app_vectors.json")["ed25519"];
    let key = k::key_from_seed(&h(&e["seed_hex"])).unwrap();
    assert_eq!(hex::encode(key.verifying_key().as_bytes()), e["public_hex"]);
    assert_eq!(k::fingerprint(&key.verifying_key()), e["fingerprint"]);
    let md = k::sign_request_at(
        &key,
        e["method"].as_str().unwrap(),
        &h(&e["body_hex"]),
        e["ts"].as_i64().unwrap(),
        e["nonce"].as_str().unwrap(),
    );
    let get = |n: &str| md.iter().find(|(k, _)| *k == n).unwrap().1.clone();
    assert_eq!(get("x-itcmdr-sig"), e["sig_hex"]);
    assert_eq!(get("x-itcmdr-key"), e["fingerprint"]);
    k::verify_request(
        &key.verifying_key(),
        e["method"].as_str().unwrap(),
        e["ts"].as_i64().unwrap(),
        e["nonce"].as_str().unwrap(),
        &h(&e["body_hex"]),
        e["sig_hex"].as_str().unwrap(),
    )
    .unwrap();
    assert!(k::verify_request(
        &key.verifying_key(),
        e["method"].as_str().unwrap(),
        e["ts"].as_i64().unwrap() + 1,
        e["nonce"].as_str().unwrap(),
        &h(&e["body_hex"]),
        e["sig_hex"].as_str().unwrap(),
    )
    .is_err());
}

#[test]
fn sealed_words_vector() {
    let s = &json("app_vectors.json")["sealed_words"];
    let server: [u8; 32] = h(&s["server_public_hex"]).try_into().unwrap();
    let out = k::restore::seal_with(
        &server,
        s["key_id"].as_str().unwrap(),
        s["backup"].as_str().unwrap(),
        s["words"].as_str().unwrap(),
        h(&s["eph_secret_hex"]).try_into().unwrap(),
        h(&s["nonce_hex"]).try_into().unwrap(),
    )
    .unwrap();
    assert_eq!(out.eph, s["eph"]);
    assert_eq!(out.ct, s["ct"]);
    let secret: [u8; 32] = h(&s["server_secret_hex"]).try_into().unwrap();
    let opened = k::restore::open(
        &x25519_dalek_secret(secret),
        &out,
        s["backup"].as_str().unwrap(),
    )
    .unwrap();
    assert_eq!(opened.as_str(), s["words"]);
}

fn x25519_dalek_secret(b: [u8; 32]) -> k::restore::StaticSecret {
    k::restore::StaticSecret::from(b)
}

#[test]
fn master_derivation_is_pinned() {
    let w = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon \
             abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let m = k::Master::from_words(w).unwrap();
    assert_eq!(
        k::fingerprint(&m.public().ed25519),
        "cf6f7e6b7c4206b44e4590982041fae3c6c02f700450d1a000ec4fa25bb3ccc8"
    );
}
