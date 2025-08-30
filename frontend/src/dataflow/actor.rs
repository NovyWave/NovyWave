//! Single-value Actor implementation for reactive state management
//!
//! Actor provides controlled state management with sequential message processing.
//! It owns a Mutable<T> and processes events from Relays to update state safely.

use zoon::{Mutable, Signal, Task, TaskHandle};
// futures imports removed - not needed in current implementation
use std::future::Future;
use std::sync::Arc;

/// Single-value reactive state container for Actor+Relay architecture.
/// 
/// Actor controls all mutations to a piece of state through sequential
/// message processing. It prevents race conditions and provides traceability
/// for all state changes.
/// 
/// # Core Principles
/// 
/// - **Single Point of Mutation**: Only the Actor can modify its state
/// - **Sequential Processing**: Events are processed one at a time in order
/// - **Reactive Signals**: UI can bind to state changes through signals
/// - **No Direct Access**: No .get() methods - use signals for all access
/// 
/// # Examples
/// 
/// ```rust
/// use crate::actors::{Actor, relay, select};
/// 
/// let (increment_relay, increment_stream) = relay();
/// let (decrement_relay, decrement_stream) = relay();
/// 
/// let counter = Actor::new(0, async move |state| {
///     loop {
///         select! {
///             Some(amount) = increment_stream.next() => {
///                 state.update_mut(|current| *current += amount);
///             }
///             Some(amount) = decrement_stream.next() => {
///                 state.update_mut(|current| *current = current.saturating_sub(amount));
///             }
///         }
///     }
/// });
/// 
/// // Emit events
/// increment_relay.send(5);
/// decrement_relay.send(2);
/// 
/// // Bind to UI
/// counter.signal() // Always returns current state reactively
/// ```
#[derive(Clone, Debug)]
pub struct Actor<T>
where
    T: Clone + Send + Sync + 'static,
{
    state: Mutable<T>,
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[allow(dead_code)]
    task_handle: Arc<TaskHandle>,
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[cfg(debug_assertions)]
    #[allow(dead_code)]
    creation_location: &'static std::panic::Location<'static>,
}

impl<T> Actor<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Create a new Actor with initial state and event processing loop.
    /// 
    /// The processor function should contain a loop that uses `select!`
    /// to handle multiple event streams sequentially.
    /// 
    /// # Arguments
    /// 
    /// - `initial_state`: The starting value for this Actor's state
    /// - `processor`: Async function that processes events and updates state
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use crate::actors::{Actor, relay, select};
    /// use futures::StreamExt;
    /// 
    /// let (event_relay, mut event_stream) = relay();
    /// 
    /// let actor = Actor::new("initial".to_string(), async move |state| {
    ///     while let Some(new_value) = event_stream.next().await {
    ///         state.set_neq(new_value);
    ///     }
    /// });
    /// ```
    #[track_caller]
    pub fn new<F, Fut>(initial_state: T, processor: F) -> Self
    where
        F: FnOnce(Mutable<T>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let state = Mutable::new(initial_state);

        // Start the async processor task with droppable handle
        let task_handle = Arc::new(Task::start_droppable(processor(state.clone())));

        Self { 
            state,
            task_handle,
            #[cfg(debug_assertions)]
            creation_location: std::panic::Location::caller(),
        }
    }

    /// Get a reactive signal for this Actor's state.
    /// 
    /// This is the ONLY way to access Actor state. No direct .get() methods
    /// are provided to maintain architectural principles.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let counter = Actor::new(0, /* processor */);
    /// 
    /// // Bind to UI reactively
    /// El::new().child_signal(
    ///     counter.signal().map(|count| Text::new(&count.to_string()))
    /// )
    /// ```
    pub fn signal(&self) -> impl Signal<Item = T> {
        self.state.signal_cloned()
    }


    /// Get a reactive signal with a reference to avoid cloning.
    /// 
    /// Use this when the state is large and you want to avoid cloning
    /// on every signal emission.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let large_data = Actor::new(vec![0; 1000], /* processor */);
    /// 
    /// // Use signal_ref to avoid cloning large vector
    /// large_data.signal_ref(|data| data.len())
    /// ```
    pub fn signal_ref<U>(&self, f: impl Fn(&T) -> U + Send + Sync + 'static) -> impl Signal<Item = U>
    where
        U: PartialEq + Send + Sync + 'static,
    {
        self.state.signal_ref(f)
    }
    

}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::relay;
    use futures::{select, StreamExt};

    #[tokio::test]
    async fn test_actor_basic_functionality() {
        let (increment_relay, mut increment_stream) = relay();
        
        let counter = Actor::new(0, async move |state| {
            while let Some(amount) = increment_stream.next().await {
                state.update_mut(|current| *current += amount);
            }
        });

        // Wait a moment for the processor to start
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        increment_relay.send(5);
        increment_relay.send(3);

        // Wait for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Check final state through signal
        let final_value = counter.signal().to_stream().next().await.unwrap();
        assert_eq!(final_value, 8);
    }

    #[tokio::test]
    async fn test_actor_multiple_streams() {
        let (increment_relay, mut increment_stream) = relay();
        let (decrement_relay, mut decrement_stream) = relay();
        
        let counter = Actor::new(10, async move |state| {
            loop {
                select! {
                    Some(amount) = increment_stream.next() => {
                        state.update_mut(|current| *current += amount);
                    }
                    Some(amount) = decrement_stream.next() => {
                        state.update_mut(|current| *current = current.saturating_sub(amount));
                    }
                }
            }
        });

        // Wait for processor to start
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        increment_relay.send(5);
        decrement_relay.send(3);

        // Wait for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let final_value = counter.signal().to_stream().next().await.unwrap();
        assert_eq!(final_value, 12); // 10 + 5 - 3
    }

    #[tokio::test]
    async fn test_actor_state_handle_operations() {
        let (event_relay, mut event_stream) = relay();
        
        let actor = Actor::new("initial".to_string(), async move |state| {
            while let Some(operation) = event_stream.next().await {
                match operation.as_str() {
                    "uppercase" => {
                        let current = state.get_cloned();
                        state.set(current.to_uppercase());
                    },
                    "clear" => state.set_neq(String::new()),
                    value => state.update_mut(|s| s.push_str(value)),
                }
            }
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        event_relay.send("_test".to_string());
        event_relay.send("uppercase".to_string());

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let final_value = actor.signal().to_stream().next().await.unwrap();
        assert_eq!(final_value, "INITIAL_TEST");
    }
}