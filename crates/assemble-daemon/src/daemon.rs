mod daemon_impl;
pub mod error;
pub use daemon_impl::Daemon;

pub mod message;

/// The size of the finger prints
pub const DAEMON_FINGERPRINT_SIZE: usize = 16;
pub type DaemonFingerprint = [u8; DAEMON_FINGERPRINT_SIZE];
