#![allow(clippy::result_large_err, clippy::field_reassign_with_default)]
pub mod config;
pub mod db;
pub mod ingest;
pub mod metrics;
pub mod events;
pub mod generated {
    pub mod itcmdr {
        include!("generated/itcmdr.rs");
    }
}

