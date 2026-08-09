//! Reactive watch channel for Podium.
//!
//! Provides a `Sender<T>` / `Receiver<T>` pair where receivers can `.await`
//! the next change to the shared value. Multiple receivers are supported —
//! each is independently tracked by version number and woken when the sender
//! produces a new value.
//!
//! Used in Phase 2 for reactive project state: when `projects.toml` changes
//! on disk, watchers fire and the UI updates without polling.
//!
//! # Usage
//!
//! ```rust
//! let (mut tx, mut rx) = watch::channel(0u32);
//! tx.send(1).ok();
//! // In an async context:
//! // let value = rx.recv().await.unwrap(); // → 1
//! ```
//!
//! Lifted from Zed `crates/watch/src/watch.rs` (Apache 2.0).
//! Changes from original:
//! - `mod error` replaced with `use crate::watch_error::*` (flat Podium layout:
//!   watch_error.rs is a crate-root sibling, not a submodule of watch)
//! - `use std::future::Future` added (not needed in Zed due to glob imports)
//! - `changed()` return type gains `+ use<'_, T>` (required by Rust 2024 edition)
//! - `#[cfg(test)]` block removed (used Zed-specific test executor and zlog)

use crate::watch_error::*;

use parking_lot::{RwLock, RwLockReadGuard, RwLockUpgradableReadGuard};
use std::{
    collections::BTreeMap,
    future::Future,
    mem,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

/// Create a new watch channel with an initial value.
///
/// Returns a `(Sender, Receiver)` pair. The receiver starts at version 0 and
/// will not yield until the sender produces its first new value.
pub fn channel<T>(value: T) -> (Sender<T>, Receiver<T>) {
    let state = Arc::new(RwLock::new(State {
        value,
        wakers: BTreeMap::new(),
        next_waker_id: WakerId::default(),
        version: 0,
        closed: false,
    }));

    (
        Sender {
            state: state.clone(),
        },
        Receiver { state, version: 0 },
    )
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct WakerId(usize);

impl WakerId {
    fn post_inc(&mut self) -> Self {
        let id = *self;
        self.0 = id.0.wrapping_add(1);
        *self
    }
}

struct State<T> {
    value: T,
    wakers: BTreeMap<WakerId, Waker>,
    next_waker_id: WakerId,
    version: usize,
    closed: bool,
}

/// The sending half of a watch channel.
///
/// Dropping the sender marks the channel as closed; any pending `recv()` on
/// a `Receiver` will resolve to `Err(NoSenderError)`.
pub struct Sender<T> {
    state: Arc<RwLock<State<T>>>,
}

impl<T> Sender<T> {
    /// Create a new `Receiver` from this sender, starting at the current version.
    pub fn receiver(&self) -> Receiver<T> {
        let version = self.state.read().version;
        Receiver {
            state: self.state.clone(),
            version,
        }
    }

    /// Send a new value.
    ///
    /// Returns `Err(NoReceiverError)` if there are no receivers (i.e. this
    /// sender holds the only `Arc` reference to the shared state). The value
    /// is still stored but no wakers are notified.
    pub fn send(&mut self, value: T) -> Result<(), NoReceiverError> {
        if let Some(state) = Arc::get_mut(&mut self.state) {
            let state = state.get_mut();
            state.value = value;
            debug_assert_eq!(state.wakers.len(), 0);
            Err(NoReceiverError)
        } else {
            let mut state = self.state.write();
            state.value = value;
            state.version = state.version.wrapping_add(1);
            let wakers = mem::take(&mut state.wakers);
            drop(state);

            for (_, waker) in wakers {
                waker.wake();
            }

            Ok(())
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut state = self.state.write();
        state.closed = true;
        for (_, waker) in mem::take(&mut state.wakers) {
            waker.wake();
        }
    }
}

/// The receiving half of a watch channel.
///
/// Clone to create additional independent receivers — each tracks its own
/// version and is woken independently.
#[derive(Clone)]
pub struct Receiver<T> {
    state: Arc<RwLock<State<T>>>,
    version: usize,
}

struct Changed<'a, T> {
    receiver: &'a mut Receiver<T>,
    pending_waker_id: Option<WakerId>,
}

impl<T> Future for Changed<'_, T> {
    type Output = Result<(), NoSenderError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = &mut *self;

        let state = this.receiver.state.upgradable_read();
        if state.version != this.receiver.version {
            // The sender produced a new value. Avoid unregistering the pending
            // waker, because the sender has already done so.
            this.pending_waker_id = None;
            this.receiver.version = state.version;
            Poll::Ready(Ok(()))
        } else if state.closed {
            Poll::Ready(Err(NoSenderError))
        } else {
            let mut state = RwLockUpgradableReadGuard::upgrade(state);

            // Unregister the pending waker. This should happen automatically
            // when the waker gets awoken by the sender, but if this future was
            // polled again without an explicit call to `wake` (e.g., a spurious
            // wake by the executor), we need to remove it manually.
            if let Some(pending_waker_id) = this.pending_waker_id.take() {
                state.wakers.remove(&pending_waker_id);
            }

            // Register the waker for this future.
            let waker_id = state.next_waker_id.post_inc();
            state.wakers.insert(waker_id, cx.waker().clone());
            this.pending_waker_id = Some(waker_id);

            Poll::Pending
        }
    }
}

impl<T> Drop for Changed<'_, T> {
    fn drop(&mut self) {
        // If this future gets dropped before the waker has a chance of being
        // awoken, we need to clear it to avoid a memory leak.
        if let Some(waker_id) = self.pending_waker_id {
            let mut state = self.receiver.state.write();
            state.wakers.remove(&waker_id);
        }
    }
}

impl<T> Receiver<T> {
    /// Borrow the current value, marking this version as seen.
    ///
    /// After calling `borrow()`, `changed()` will not resolve until the sender
    /// produces a *new* value (version increments again).
    pub fn borrow(&mut self) -> parking_lot::MappedRwLockReadGuard<'_, T> {
        let state = self.state.read();
        self.version = state.version;
        RwLockReadGuard::map(state, |state| &state.value)
    }

    /// Returns a future that resolves when the sender produces a new value.
    ///
    /// Resolves to `Err(NoSenderError)` if the sender was dropped.
    ///
    /// The `+ use<'_, T>` bound explicitly captures the anonymous lifetime of
    /// `&mut self` in the returned opaque type, required by Rust 2024 edition.
    pub fn changed(&mut self) -> impl Future<Output = Result<(), NoSenderError>> + use<'_, T> {
        Changed {
            receiver: self,
            pending_waker_id: None,
        }
    }

    /// Create a `Receiver` holding a constant value that will never change.
    ///
    /// Useful in tests or for contexts where a `Receiver<T>` is required but
    /// no live sender exists.
    pub fn constant(value: T) -> Self {
        let state = Arc::new(RwLock::new(State {
            value,
            wakers: BTreeMap::new(),
            next_waker_id: WakerId::default(),
            version: 0,
            closed: false,
        }));

        Self { state, version: 0 }
    }
}

impl<T: Clone> Receiver<T> {
    /// Await the next value from the sender and return a clone of it.
    ///
    /// Resolves to `Err(NoSenderError)` if the sender was dropped before
    /// producing a new value.
    pub async fn recv(&mut self) -> Result<T, NoSenderError> {
        self.changed().await?;
        Ok(self.borrow().clone())
    }
}
