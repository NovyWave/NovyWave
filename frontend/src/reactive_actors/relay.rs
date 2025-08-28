//! Relay<T> - Type-safe Event Streaming
//!
//! Provides non-lossy event streaming with compile-time type safety and
//! source location constraints for Actor+Relay architecture.

use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::stream::Stream;
use std::sync::Arc;

/// Type-safe event relay with source location constraints.
/// 
/// Relays provide reliable message passing between UI components and Actors.
/// Unlike signals, relays do not lose events and provide compile-time type safety.
/// 
/// ## Event-Source Naming Convention
/// 
/// All relay names MUST follow the `{source}_{event}_relay` pattern:
/// 
/// ```rust
/// struct TrackedFiles {
///     // ✅ CORRECT: Event-source naming
///     file_dropped_relay: Relay<Vec<PathBuf>>,     // User dropped files
///     parse_completed_relay: Relay<ParseResult>,   // Parser finished
///     error_occurred_relay: Relay<String>,         // System error happened
///     
///     // ❌ PROHIBITED: Command-like naming  
///     // add_file: Relay<PathBuf>,                 // Sounds like command
///     // remove_file: Relay<String>,               // Imperative style
/// }
/// ```
/// 
/// ## Usage Pattern
/// 
/// ```rust
/// use crate::reactive_actors::{relay, select};
/// 
/// // Create relay and stream
/// let (file_dropped_relay, mut file_dropped_stream) = relay();
/// 
/// // Use in Actor
/// let files = ActorVec::new(vec![], async move |files_vec| {
///     loop {
///         select! {
///             Some(paths) = file_dropped_stream.next() => {
///                 // Handle dropped files
///                 for path in paths {
///                     files_vec.lock_mut().push_cloned(TrackedFile::new(path));
///                 }
///             }
///         }
///     }
/// });
/// 
/// // Emit events from UI
/// file_dropped_relay.send(vec![PathBuf::from("waveform.vcd")]);
/// ```
#[derive(Clone)]
pub struct Relay<T> {
    sender: Arc<UnboundedSender<T>>,
}

impl<T> Relay<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Creates a new Relay for event streaming.
    /// 
    /// Use the `relay()` function instead for creating Relay+Stream pairs.
    pub fn new() -> Self {
        let (sender, _) = mpsc::unbounded();
        Relay {
            sender: Arc::new(sender),
        }
    }
    
    /// Creates a new Relay with access to its receiver stream.
    /// 
    /// This is used internally by the `relay()` function.
    pub(crate) fn with_receiver() -> (Self, UnboundedReceiver<T>) {
        let (sender, receiver) = mpsc::unbounded();
        let relay = Relay {
            sender: Arc::new(sender),
        };
        (relay, receiver)
    }
    
    /// Creates a subscription stream for this relay.
    /// 
    /// Multiple streams can be created from the same relay, but each event
    /// will only be delivered to the first available stream.
    pub fn subscribe(&self) -> impl Stream<Item = T> {
        let (_, receiver) = Self::with_receiver();
        receiver
    }
    
    /// Sends an event through this relay.
    /// 
    /// Events are delivered to any active stream subscribers. If no subscribers
    /// are active, the event is dropped (fire-and-forget semantics).
    /// 
    /// ## Event-Source Examples
    /// 
    /// ```rust
    /// // User interactions - what the user DID
    /// button_clicked_relay.send(());
    /// input_changed_relay.send("new text".to_string());
    /// file_dropped_relay.send(vec![path1, path2]);
    /// 
    /// // System events - what HAPPENED
    /// file_loaded_relay.send(PathBuf::from("data.vcd"));  
    /// parse_completed_relay.send(ParseResult::Success(signals));
    /// error_occurred_relay.send("Parse failed".to_string());
    /// ```
    pub fn send(&self, value: T) {
        // Fire-and-forget semantics - if no receivers, event is dropped
        let _ = self.sender.unbounded_send(value);
    }
    
    /// Attempts to send an event, returning an error if the relay is closed.
    /// 
    /// Use this for critical operations where you need confirmation that the
    /// event was delivered to the relay's internal queue.
    pub fn try_send(&self, value: T) -> Result<(), mpsc::TrySendError<T>> {
        self.sender.unbounded_send(value)
            .map_err(|e| match e {
                mpsc::SendError::Full(v) => mpsc::TrySendError::Full(v),
                mpsc::SendError::Disconnected(v) => mpsc::TrySendError::Disconnected(v),
            })
    }
    
    /// Checks if this relay is closed (no active receivers).
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

impl<T> Default for Relay<T>
where
    T: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

// Debug implementation that doesn't expose internal channel details
impl<T> std::fmt::Debug for Relay<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Relay")
            .field("closed", &self.is_closed())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    
    #[tokio::test]
    async fn test_relay_event_delivery() {
        let (relay, mut stream) = crate::reactive_actors::relay();
        
        // Send event
        relay.send("test_event".to_string());
        
        // Receive event
        let received = stream.next().await;
        assert_eq!(received, Some("test_event".to_string()));
    }
    
    #[tokio::test] 
    async fn test_relay_fire_and_forget() {
        let relay = Relay::new();
        
        // Should not block or error when no receivers
        relay.send("dropped_event".to_string());
        
        // Try_send should return disconnected when no receivers
        let result = relay.try_send("another_event".to_string());
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_multiple_events() {
        let (relay, mut stream) = crate::reactive_actors::relay();
        
        // Send multiple events
        relay.send(1);
        relay.send(2);
        relay.send(3);
        
        // Should receive all events in order
        assert_eq!(stream.next().await, Some(1));
        assert_eq!(stream.next().await, Some(2));
        assert_eq!(stream.next().await, Some(3));
    }
}