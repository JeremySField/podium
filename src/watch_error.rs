//! Error types for the Podium watch channel.
//!
//! Lifted from Zed `crates/watch/src/error.rs` (Apache 2.0) — no changes.

use std::fmt;

#[derive(Debug, Eq, PartialEq)]
pub struct NoReceiverError;

impl fmt::Display for NoReceiverError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "all receivers were dropped")
    }
}

impl std::error::Error for NoReceiverError {}

#[derive(Debug, Eq, PartialEq)]
pub struct NoSenderError;

impl fmt::Display for NoSenderError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "sender was dropped")
    }
}

impl std::error::Error for NoSenderError {}
