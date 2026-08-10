//! Error types for the Podium watch channel.
//!
//! Lifted from Zed `crates/watch/src/error.rs`.
//! Copyright 2022 - 2025 Zed Industries, Inc.
//! Licensed under the Apache License, Version 2.0.
//! No changes from original.

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
