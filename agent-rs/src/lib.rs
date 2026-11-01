//! itcmdr-agent (v0.17.0): enrolls with the site server and keeps a heartbeat link over pinned
//! TLS with a client certificate (mTLS) and a signed Hello. See docs/AGENT.md.
pub mod link;
pub mod state;
