//! Simplified event streaming Relay implementation
//!
//! Relay provides type-safe event streaming for Actor+Relay architecture
//! using simple unbounded channels instead of complex custom Stream implementation.

use futures::channel::mpsc::{unbounded, UnboundedSender, UnboundedReceiver};
use std::sync::{Arc, OnceLock};

/// Type-safe event streaming relay for Actor+Relay architecture.
/// 
/// Relays provide reliable message passing from UI components to Actors
/// using simple unbounded channels.
/// 
/// # Event-Source Naming Convention
/// 
/// All relays MUST follow `{source}_{event}_relay` naming pattern:
/// - `file_dropped_relay` - User dropped files on UI
/// - `parse_completed_relay` - Parser finished processing
/// - `cursor_clicked_relay` - User clicked timeline at position
/// 
/// # Examples
/// 
/// ```rust
/// use crate::dataflow::{Relay, relay};
/// 
/// // Create relay with subscription stream
/// let (file_dropped_relay, mut stream) = relay::<Vec<PathBuf>>();
/// 
/// // Emit events from UI
/// file_dropped_relay.send(vec![PathBuf::from("test.vcd")]);
/// 
/// // Process events in Actor
/// while let Some(paths) = stream.next().await {
///     println!("Files dropped: {:?}", paths);
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Relay<T>
where
    T: Clone + Send + Sync + 'static,
{
    sender: UnboundedSender<T>,
    #[cfg(debug_assertions)]
    emit_location: Arc<OnceLock<&'static std::panic::Location<'static>>>,
}

/// Error type for Relay operations
#[derive(Debug, Clone)]
pub enum RelayError {
    /// The channel has been closed (receiver dropped)
    ChannelClosed,
    /// Relay send called from multiple locations (debug builds only)
    #[cfg(debug_assertions)]
    MultipleEmitters {
        previous: &'static std::panic::Location<'static>,
        current: &'static std::panic::Location<'static>,
    },
}

impl<T> Relay<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Create a new Relay with an associated receiver stream.
    /// 
    /// Returns a tuple of (Relay, UnboundedReceiver) following Rust's
    /// channel patterns. Use the `relay()` function for more convenient creation.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let (relay, receiver) = Relay::new();
    /// ```
    pub fn new() -> (Self, UnboundedReceiver<T>) {
        let (sender, receiver) = unbounded();
        (
            Relay {
                sender,
                #[cfg(debug_assertions)]
                emit_location: Arc::new(OnceLock::new()),
            },
            receiver,
        )
    }

    /// Check that this relay is only being sent from a single source location.
    /// 
    /// In debug builds, enforces single-source constraint for relays.
    /// Returns an error if the relay has been sent from a different location.
    #[cfg(debug_assertions)]
    #[track_caller]
    fn check_single_source(&self) -> Result<(), RelayError> {
        let caller = std::panic::Location::caller();
        match self.emit_location.set(caller) {
            Ok(()) => Ok(()), // First call, location set
            Err(previous) if previous == caller => Ok(()), // Same location, allowed
            Err(previous) => Err(RelayError::MultipleEmitters {
                previous,
                current: caller,
            }),
        }
    }

    /// Send an event through the relay.
    /// 
    /// If the receiver has been dropped, the event is silently discarded.
    /// Use `try_send()` if you need to handle send failures.
    /// 
    /// In debug builds, panics if this relay has been sent from a different
    /// location in the code (enforces single-source constraint).
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// file_dropped_relay.send(vec![PathBuf::from("test.vcd")]);
    /// ```
    #[track_caller]
    pub fn send(&self, value: T) {
        #[cfg(debug_assertions)]
        if let Err(e) = self.check_single_source() {
            panic!("{:?}", e);
        }
        
        // Ignore send errors - this matches the previous behavior
        // where events were dropped if no subscribers existed
        let _ = self.sender.unbounded_send(value);
    }

    /// Try to send an event through the relay with explicit error handling.
    /// 
    /// Returns an error if the channel has been closed (receiver dropped).
    /// In debug builds, also returns an error if this relay has been sent
    /// from a different location in the code.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// match relay.try_send(event) {
    ///     Ok(()) => println!("Event sent successfully"),
    ///     Err(RelayError::ChannelClosed) => println!("Receiver dropped"),
    ///     #[cfg(debug_assertions)]
    ///     Err(RelayError::MultipleEmitters { .. }) => println!("Multiple sources"),
    /// }
    /// ```
    #[track_caller]
    pub fn try_send(&self, value: T) -> Result<(), RelayError> {
        #[cfg(debug_assertions)]
        self.check_single_source()?;
        
        self.sender
            .unbounded_send(value)
            .map_err(|_| RelayError::ChannelClosed)
    }

}

impl<T> Default for Relay<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Create a new Relay with a receiver that is immediately dropped.
    /// 
    /// This creates a "disconnected" relay where events are silently discarded.
    /// Useful for:
    /// - Optional relays in structs that may not have event handlers
    /// - Placeholder initialization before wiring actual relays
    /// - Testing scenarios where event handling is not needed
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// struct DialogComponent {
    ///     close_relay: Relay<()>,  // Might not always have a handler
    /// }
    /// 
    /// impl Default for DialogComponent {
    ///     fn default() -> Self {
    ///         Self {
    ///             close_relay: Relay::default(), // Disconnected relay
    ///         }
    ///     }
    /// }
    /// ```
    fn default() -> Self {
        let (relay, _receiver) = Self::new();
        relay
    }
}

/// Creates a new Relay with an associated receiver stream.
/// 
/// This is the idiomatic way to create a Relay for use with Actors,
/// following Rust's channel pattern conventions.
/// 
/// # Examples
/// 
/// ```rust
/// use crate::dataflow::relay;
/// use futures::StreamExt;
/// 
/// let (increment_relay, mut increment_stream) = relay();
/// let (decrement_relay, mut decrement_stream) = relay();
/// 
/// let counter = Actor::new(0, async move |state| {
///     loop {
///         select! {
///             Some(amount) = increment_stream.next() => {
///                 state.update(|n| n + amount);
///             }
///             Some(amount) = decrement_stream.next() => {
///                 state.update(|n| n.saturating_sub(amount));
///             }
///         }
///     }
/// });
/// ```
pub fn relay<T>() -> (Relay<T>, UnboundedReceiver<T>)
where
    T: Clone + Send + Sync + 'static,
{
    Relay::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_relay_basic_functionality() {
        let (relay, mut receiver) = Relay::new();

        relay.send("test_event".to_string());
        
        let received = receiver.next().await;
        assert_eq!(received, Some("test_event".to_string()));
    }

    #[tokio::test]
    async fn test_relay_try_send() {
        let (relay, mut receiver) = Relay::new();

        // Should succeed while receiver exists
        assert!(relay.try_send("test".to_string()).is_ok());
        assert_eq!(receiver.next().await, Some("test".to_string()));

        // Drop receiver
        drop(receiver);

        // Should fail after receiver dropped
        assert!(relay.try_send("fail".to_string()).is_err());
    }

    #[tokio::test]
    async fn test_relay_function() {
        let (relay, mut stream) = relay::<String>();

        relay.send("via_function".to_string());
        
        assert_eq!(stream.next().await, Some("via_function".to_string()));
    }
}