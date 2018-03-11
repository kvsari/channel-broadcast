//! Acts as a sender. Multiple receivers can be requested from it and all messages are
//! cloned to all live receivers.

extern crate futures;

use std::sync::{Arc, Mutex, PoisonError};
use std::{error, fmt, convert};

use futures::sync::mpsc;

/// Unbounded broadcast sender. Multiple `futures` `UnboundedReceiver`s can be requested
/// from it. Message type must implement `Clone`. Dropping a receiver is fine as it will
/// be automatically pruned on the next send.
///
/// # Todo
/// 1. Add a cache to store messages when there are no receivers. Or, wrap it in a cache.
/// 2. Add variant impl for `T: Copy`
#[derive(Clone)]
pub struct UnboundedBroadcaster<T> {
    sender: Arc<Mutex<Vec<mpsc::UnboundedSender<T>>>>,
}

impl<T: Clone> UnboundedBroadcaster<T> {
    pub fn new() -> Self {
        UnboundedBroadcaster {
            sender: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Send the message broadcasting it to all receivers. If there are no receivers, the
    /// message is returned.    
    pub fn send(&self, msg: T) -> Result<(), BroadcastError<T>> {
        let mut sent = 0;
        let mut lock = self.sender.lock()?;
        
        lock.retain(|chan| {
            match chan.unbounded_send(msg.clone()) {
                Ok(()) => {
                    sent += 1;
                    true
                },
                Err(_) => false,
            }
        });       

        if sent > 0 {
            Ok(())
        } else {
            Err(BroadcastError::NoReceivers(msg))
        }
    }

    /// Request a receiver from the broadcaster. Messages sent prior to this will be missed
    /// but and messages sent after the call to this method will be received.
    pub fn receiver(&self) -> Result<mpsc::UnboundedReceiver<T>, BroadcastError<T>> {
        let (tx, rx) = mpsc::unbounded();
        let mut lock = self.sender.lock()?;
        lock.push(tx);
        Ok(rx)
    }
}

pub enum BroadcastError<T> {
    NoReceivers(T),
    Poisoned,
}

impl<T, G> convert::From<PoisonError<G>> for BroadcastError<T> {
    fn from(_: PoisonError<G>) -> Self {
        BroadcastError::Poisoned
    }
}

impl<T> fmt::Display for BroadcastError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &BroadcastError::NoReceivers(_) => "No receivers for send".fmt(f),
            &BroadcastError::Poisoned => "poisoned lock: another task failed".fmt(f),
        }
    }
}

impl<T> fmt::Debug for BroadcastError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &BroadcastError::NoReceivers(_) => "No receivers for send".fmt(f),
            &BroadcastError::Poisoned => "PoisonError { sender: .. }".fmt(f),
        }
    }
}

impl<T> error::Error for BroadcastError<T> {
    fn description(&self) -> &str {
        "Broadcast problem. Message can't be sent."
    }
}
