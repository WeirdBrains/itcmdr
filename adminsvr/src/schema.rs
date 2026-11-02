//! Diesel schema (both backends; see migrations/postgres and migrations/sqlite).
diesel::table! { meta (k) { k -> Text, v -> Text, } }
diesel::table! {
    users (id) {
        id -> Text, email -> Text, name -> Text, role -> Text, status -> Text,
        master_pub -> Nullable<Text>, master_fp -> Nullable<Text>, key_backup_opt_out -> Bool,
        created_by -> Text, created_ms -> BigInt,
    }
}
diesel::table! { enroll_codes (secret_hash) { secret_hash -> Text, user_id -> Text, expires_ms -> BigInt, used_ms -> Nullable<BigInt>, } }
diesel::table! {
    devices (fingerprint) {
        fingerprint -> Text, user_id -> Text, public_key -> Text, certificate -> Text, platform -> Text,
        status -> Text, created_ms -> BigInt, last_used_ms -> BigInt,
    }
}
diesel::table! { key_backups (user_id) { user_id -> Text, blob -> Text, updated_ms -> BigInt, } }
diesel::table! {
    sites (id) {
        id -> Text, name -> Text, status -> Text, site_pub -> Nullable<Text>, cert_serial -> Nullable<Text>,
        version -> Text, last_seen_ms -> BigInt, share_names -> Bool, last_status -> Nullable<Binary>,
        last_seq -> BigInt, created_ms -> BigInt,
        config -> Nullable<Binary>, config_ms -> BigInt, plugins -> Nullable<Binary>, plugins_ms -> BigInt,
    }
}
diesel::table! { site_tokens (secret_hash) { secret_hash -> Text, site_id -> Text, expires_ms -> BigInt, used_ms -> Nullable<BigInt>, } }
diesel::table! { site_scopes (user_id, site_id) { user_id -> Text, site_id -> Text, } }
diesel::table! {
    alerts (site_id, alert_id) {
        site_id -> Text, alert_id -> Text, origin -> Text, state -> Text, severity -> Text,
        last_seen_ms -> BigInt, body -> Binary,
    }
}
diesel::table! {
    commands (id) {
        id -> Text, site_id -> Text, kind -> Text, requested_by -> Text, state -> Text,
        result_code -> Text, created_ms -> BigInt, body -> Binary,
        dry_run -> Bool, preview -> Nullable<Binary>, approved_by -> Text,
    }
}
diesel::table! {
    audit_chain (seq) {
        seq -> BigInt, ts_ms -> BigInt, actor -> Text, actor_key -> Text, action -> Text, target -> Text,
        payload_sha256 -> Text, request_sig -> Text, prev_hash -> Text, hash -> Text,
    }
}
diesel::table! {
    passkeys (credential_id) {
        credential_id -> Text, user_id -> Text, label -> Text, alg -> BigInt, public_key -> Binary,
        aaguid -> Text, attestation -> Text, sign_count -> BigInt, hmac_secret -> Bool,
        created_ms -> BigInt, last_used_ms -> BigInt,
    }
}
diesel::table! { site_metrics (site_id, ts_ms) { site_id -> Text, ts_ms -> BigInt, body -> Binary, } }
diesel::table! {
    push_tokens (token) {
        token -> Text, provider -> Text, user_id -> Text, device_fp -> Text, platform -> Text,
        created_ms -> BigInt,
    }
}
diesel::table! {
    catalog (sha256) {
        sha256 -> Text, name -> Text, version -> Text, size -> BigInt, description -> Text,
        uploaded_by -> Text, uploaded_ms -> BigInt, body -> Binary,
    }
}
diesel::allow_tables_to_appear_in_same_query!(
    catalog,
    site_metrics,
    push_tokens,
    passkeys,
    meta,
    users,
    enroll_codes,
    devices,
    key_backups,
    sites,
    site_tokens,
    site_scopes,
    alerts,
    commands,
    audit_chain
);
diesel::table! {
    release_approvals (sha256, user_id) {
        sha256 -> Text, user_id -> Text, role -> Text, pubkey -> Text, endorsement -> Text, sig -> Text,
        ts_ms -> BigInt,
    }
}
