//! Simplified event streaming Relay implementation
//!
//! Relay provides type-safe event streaming for Actor+Relay architecture
//! using simple unbounded channels instead of complex custom Stream implementation.

use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::stream::Stream;
use std::sync::{Arc, OnceLock, Mutex};

/// Type-safe event streaming relay for Actor+Relay architecture.
/// 
/// Relays provide reliable message passing from UI components to Actors
/// using broadcast channels for multiple subscriber support.
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
/// // Or create relay and subscribe separately
/// let file_dropped_relay = Relay::new();
/// let mut stream = file_dropped_relay.subscribe();
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
pub struct Relay<T = ()>
where
    T: Clone + Send + Sync + 'static,
{
    subscribers: Arc<Mutex<Vec<UnboundedSender<T>>>>,
    #[cfg(debug_assertions)]
    emit_location: Arc<OnceLock<&'static std::panic::Location<'static>>>,
}

/// Error type for Relay operations
#[derive(Debug, Clone)]
pub enum RelayError {
    /// The channel has been closed (receiver dropped)
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[allow(dead_code)]
    ChannelClosed,
    /// Relay send called from multiple locations (debug builds only)
    #[cfg(debug_assertions)]
    MultipleEmitters {
        // Part of public Actor+Relay API - will be used when moved to standalone crate
        #[allow(dead_code)]
        previous: &'static std::panic::Location<'static>,
        // Part of public Actor+Relay API - will be used when moved to standalone crate
        #[allow(dead_code)]
        current: &'static std::panic::Location<'static>,
    },
}

impl<T> Relay<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Create a new Relay.
    /// 
    /// Use `subscribe()` to get streams for receiving events.
    /// Use the `relay()` function for convenient creation with immediate stream access.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let relay = Relay::new();
    /// let stream = relay.subscribe();
    /// ```
    pub fn new() -> Self {
        Relay {
            subscribers: Arc::new(Mutex::new(Vec::new())),
            #[cfg(debug_assertions)]
            emit_location: Arc::new(OnceLock::new()),
        }
    }

    /// Subscribe to receive events from this relay.
    /// 
    /// Returns a stream that will receive all future events sent through this relay.
    /// Multiple subscribers can be created from the same relay.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let relay = Relay::new();
    /// let mut stream1 = relay.subscribe();
    /// let mut stream2 = relay.subscribe(); // Multiple subscribers
    /// ```
    pub fn subscribe(&self) -> impl Stream<Item = T> + use<T> {
        let (sender, receiver) = unbounded();
        
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.push(sender);
        }
        
        receiver
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
    /// Broadcasts the event to all current subscribers.
    /// If a subscriber's receiver has been dropped, it's automatically removed.
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
        
        if let Ok(mut subscribers) = self.subscribers.lock() {
            // Send to all subscribers, removing closed ones
            subscribers.retain(|sender| {
                sender.unbounded_send(value.clone()).is_ok()
            });
        }
    }

    /// Try to send an event through the relay with explicit error handling.
    /// 
    /// Broadcasts the event to all current subscribers.
    /// Returns an error if no subscribers are available.
    /// In debug builds, also returns an error if this relay has been sent
    /// from a different location in the code.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// match relay.try_send(event) {
    ///     Ok(()) => println!("Event sent successfully"),
    ///     Err(RelayError::ChannelClosed) => println!("No subscribers"),
    ///     #[cfg(debug_assertions)]
    ///     Err(RelayError::MultipleEmitters { .. }) => println!("Multiple sources"),
    /// }
    /// ```
    #[track_caller]
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[allow(dead_code)]
    pub fn try_send(&self, value: T) -> Result<(), RelayError> {
        #[cfg(debug_assertions)]
        self.check_single_source()?;
        
        if let Ok(mut subscribers) = self.subscribers.lock() {
            if subscribers.is_empty() {
                return Err(RelayError::ChannelClosed);
            }
            
            // Send to all subscribers, removing closed ones
            let mut any_success = false;
            subscribers.retain(|sender| {
                let success = sender.unbounded_send(value.clone()).is_ok();
                if success { any_success = true; }
                success
            });
            
            if any_success {
                Ok(())
            } else {
                Err(RelayError::ChannelClosed)
            }
        } else {
            Err(RelayError::ChannelClosed)
        }
    }

}

impl<T> Default for Relay<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Create a new Relay with no subscribers.
    /// 
    /// Events sent to this relay will be discarded until a subscriber is added.
    /// Use `subscribe()` to add subscribers.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// struct DialogComponent {
    ///     close_relay: Relay<()>,  // Start with no subscribers
    /// }
    /// 
    /// impl Default for DialogComponent {
    ///     fn default() -> Self {
    ///         Self {
    ///             close_relay: Relay::default(),
    ///         }
    ///     }
    /// }
    /// ```
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a new Relay with an associated stream, following Rust's channel pattern.
/// 
/// This is the idiomatic way to create a Relay for use with Actors,
/// eliminating clone! macro boilerplate and providing direct stream access.
/// 
/// # Examples
/// 
/// ```rust
/// use crate::dataflow::relay;
/// use futures::StreamExt;
/// 
/// // Just like Rust channels:
/// // let (tx, rx) = channel();
/// 
/// // Actor+Relay pattern:
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
pub fn relay<T>() -> (Relay<T>, impl Stream<Item = T>)
where
    T: Clone + Send + Sync + 'static,
{
    let relay = Relay::new();
    let stream = relay.subscribe();
    (relay, stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_relay_basic_functionality() {
        let relay = Relay::new();
        let mut receiver = relay.subscribe();

        relay.send("test_event".to_string());
        
        let received = receiver.next().await;
        assert_eq!(received, Some("test_event".to_string()));
    }

    #[tokio::test]
    async fn test_relay_try_send() {
        let relay = Relay::new();
        let mut receiver = relay.subscribe();

        // Should succeed while receiver exists
        assert!(relay.try_send("test".to_string()).is_ok());
        assert_eq!(receiver.next().await, Some("test".to_string()));

        // Drop receiver
        drop(receiver);

        // Should fail after receiver dropped (no subscribers)
        assert!(relay.try_send("fail".to_string()).is_err());
    }

    #[tokio::test]
    async fn test_relay_function() {
        let (relay, mut stream) = relay::<String>();

        relay.send("via_function".to_string());
        
        assert_eq!(stream.next().await, Some("via_function".to_string()));
    }

    #[tokio::test]
    async fn test_relay_multiple_subscribers() {
        let relay = Relay::new();
        let mut stream1 = relay.subscribe();
        let mut stream2 = relay.subscribe();

        relay.send("broadcast".to_string());
        
        // Both subscribers should receive the event
        assert_eq!(stream1.next().await, Some("broadcast".to_string()));
        assert_eq!(stream2.next().await, Some("broadcast".to_string()));
    }

    #[tokio::test]
    async fn test_relay_subscriber_cleanup() {
        let relay = Relay::new();
        let mut stream1 = relay.subscribe();
        let stream2 = relay.subscribe(); // Will be dropped
        
        // Drop one subscriber
        drop(stream2);
        
        relay.send("cleanup_test".to_string());
        
        // Remaining subscriber should still work
        assert_eq!(stream1.next().await, Some("cleanup_test".to_string()));
        
        // Should still work after cleanup
        assert!(relay.try_send("still_works".to_string()).is_ok());
    }
}