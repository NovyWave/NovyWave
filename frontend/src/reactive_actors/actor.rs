//! Actor<T> - Reactive State Management
//!
//! Single-value reactive state container for Actor+Relay architecture.
//! Provides controlled state mutations through event processing.

use zoon::*;
use std::sync::Arc;

/// Reactive state container that processes events sequentially.
/// 
/// Actors own a `Mutable<T>` and provide controlled access through signals.
/// All mutations happen through event processing, ensuring no race conditions.
/// 
/// ## Usage Pattern
/// 
/// ```rust
/// use crate::reactive_actors::{Actor, relay, select};
/// 
/// // Create event streams
/// let (increment_relay, mut increment_stream) = relay();
/// let (reset_relay, mut reset_stream) = relay();
/// 
/// // Create Actor with event processing
/// let counter = Actor::new(0, async move |state| {
///     loop {
///         select! {
///             Some(amount) = increment_stream.next() => {
///                 state.update(|current| current + amount);
///             }
///             Some(()) = reset_stream.next() => {
///                 state.set_neq(0);
///             }
///         }
///     }
/// });
/// 
/// // Access state reactively
/// counter.signal() // Returns Signal<Item = i32>
/// 
/// // Emit events
/// increment_relay.send(5);
/// reset_relay.send(());
/// ```
/// 
/// ## Domain Examples
/// 
/// ```rust
/// // Timeline cursor position
/// let cursor_position = Actor::new(0.0, async move |state| {
///     while let Some(new_pos) = cursor_moved_stream.next().await {
///         state.set_neq(new_pos);
///         // Trigger related updates
///         update_cursor_values(new_pos);
///     }
/// });
/// 
/// // File loading state
/// let file_state = Actor::new(FileState::Loading, async move |state| {
///     loop {
///         select! {
///             Some(()) = reload_stream.next() => {
///                 state.set_neq(FileState::Loading);
///                 // Start parsing process
///             }
///             Some(result) = parse_completed_stream.next() => {
///                 match result {
///                     Ok(data) => state.set_neq(FileState::Loaded(data)),
///                     Err(error) => state.set_neq(FileState::Error(error)),
///                 }
///             }
///         }
///     }
/// });
/// ```
#[derive(Clone)]
pub struct Actor<T> {
    state: Mutable<T>,
}

impl<T> Actor<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Creates a new Actor with initial state and event processor.
    /// 
    /// The event processor runs continuously, handling events sequentially
    /// to prevent race conditions and ensure predictable state mutations.
    /// 
    /// # Arguments
    /// 
    /// * `initial` - Initial state value
    /// * `processor` - Async function that processes events and updates state
    /// 
    /// # Example
    /// 
    /// ```rust
    /// let (theme_changed_relay, mut theme_stream) = relay();
    /// 
    /// let theme = Actor::new(Theme::Light, async move |state| {
    ///     while let Some(new_theme) = theme_stream.next().await {
    ///         state.set_neq(new_theme);
    ///         // Save to config, update UI, etc.
    ///         save_theme_to_config(&new_theme);
    ///     }
    /// });
    /// ```
    pub fn new<F, Fut>(initial: T, processor: F) -> Self
    where
        F: FnOnce(Mutable<T>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let state = Mutable::new(initial);
        let state_clone = state.clone();
        
        // Start event processor task
        Task::start(processor(state_clone));
        
        Actor { state }
    }
    
    /// Returns a reactive signal for this Actor's current state.
    /// 
    /// The signal updates automatically when the Actor's state changes
    /// through event processing. Use this for reactive UI binding.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// // Bind to UI
    /// Text::new_signal(counter.signal().map(|count| format!("Count: {}", count)))
    /// 
    /// // Combine with other signals
    /// map_ref! {
    ///     let files = tracked_files.signal(),
    ///     let filter = search_filter.signal() => {
    ///         files.iter()
    ///             .filter(|f| f.name.contains(&*filter))
    ///             .collect::<Vec<_>>()
    ///     }
    /// }
    /// ```
    pub fn signal(&self) -> impl Signal<Item = T> {
        self.state.signal()
    }
    
    /// Returns a signal reference for efficient access without cloning.
    /// 
    /// Use this when you need to access the state value without cloning it,
    /// particularly useful for large data structures.
    pub fn signal_ref<U, F>(&self, f: F) -> impl Signal<Item = U>
    where
        F: Fn(&T) -> U + Send + Sync + 'static,
        U: Send + Sync + 'static,
    {
        self.state.signal_ref(f)
    }
}

// Note: Actor<T> intentionally does NOT provide a .get() method
// All state access must be through signals to prevent race conditions
// and maintain architectural consistency

impl<T> std::fmt::Debug for Actor<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Actor")
            .field("current_state", &self.state.get())
            .finish()
    }
}

/// State update interface for use within Actor event processors.
/// 
/// This trait is automatically implemented for `Mutable<T>` and provides
/// the methods available for updating Actor state from within event processors.
pub trait ActorState<T> {
    /// Updates the state value, triggering signals if the value changed.
    fn set_neq(&self, value: T);
    
    /// Updates the state using a function, triggering signals if the value changed.
    fn update<F>(&self, f: F) 
    where 
        F: FnOnce(T) -> T;
        
    /// Updates the state using a function with mutable access.
    fn update_mut<F>(&self, f: F)
    where
        F: FnOnce(&mut T);
}

impl<T> ActorState<T> for Mutable<T>
where
    T: Clone + PartialEq,
{
    fn set_neq(&self, value: T) {
        self.set_neq(value);
    }
    
    fn update<F>(&self, f: F)
    where
        F: FnOnce(T) -> T,
    {
        self.update(f);
    }
    
    fn update_mut<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        self.update_mut(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive_actors::relay;
    use futures::{select, StreamExt};
    
    #[async_test]
    async fn test_actor_basic_functionality() {
        let (increment_relay, mut increment_stream) = relay();
        
        let counter = Actor::new(0, async move |state| {
            while let Some(amount) = increment_stream.next().await {
                state.update(|current| current + amount);
            }
        });
        
        // Send increment event
        increment_relay.send(5);
        
        // Wait for state to update
        let current_value = counter.signal().to_stream().next().await.unwrap();
        assert_eq!(current_value, 5);
    }
    
    #[async_test]
    async fn test_actor_multiple_events() {
        let (increment_relay, mut increment_stream) = relay();
        let (reset_relay, mut reset_stream) = relay();
        
        let counter = Actor::new(0, async move |state| {
            loop {
                select! {
                    Some(amount) = increment_stream.next() => {
                        state.update(|current| current + amount);
                    }
                    Some(()) = reset_stream.next() => {
                        state.set_neq(0);
                    }
                }
            }
        });
        
        // Increment multiple times
        increment_relay.send(3);
        increment_relay.send(2);
        
        // Wait for updates
        let mut signal_stream = counter.signal().to_stream();
        assert_eq!(signal_stream.next().await.unwrap(), 3);
        assert_eq!(signal_stream.next().await.unwrap(), 5);
        
        // Reset
        reset_relay.send(());
        assert_eq!(signal_stream.next().await.unwrap(), 0);
    }
    
    #[async_test]
    async fn test_actor_signal_ref() {
        let (update_relay, mut update_stream) = relay();
        
        let data = Actor::new(vec![1, 2, 3], async move |state| {
            while let Some(new_item) = update_stream.next().await {
                state.update_mut(|vec| vec.push(new_item));
            }
        });
        
        // Test signal_ref for length without cloning vec
        let length_signal = data.signal_ref(|vec| vec.len());
        let initial_length = length_signal.to_stream().next().await.unwrap();
        assert_eq!(initial_length, 3);
        
        // Add item and check length
        update_relay.send(4);
        let new_length = length_signal.to_stream().next().await.unwrap();
        assert_eq!(new_length, 4);
    }
}