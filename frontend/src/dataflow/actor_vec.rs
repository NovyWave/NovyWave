//! Reactive collection Actor implementation  
//!
//! ActorVec provides controlled collection management with sequential message processing.
//! It wraps MutableVec<T> and processes events from Relays to update collections safely.

use zoon::{MutableVec, Signal, Task, TaskHandle, SignalVecExt, SignalExt};
use std::future::Future;
use std::sync::Arc;

/// Reactive collection container for Actor+Relay architecture.
/// 
/// ActorVec controls all mutations to a collection through sequential
/// message processing. It prevents race conditions and provides efficient
/// reactive updates through VecDiff signals.
/// 
/// # Core Principles
/// 
/// - **Sequential Processing**: Collection updates processed one at a time
/// - **VecDiff Signals**: Efficient UI updates with only changed items  
/// - **Event-Driven**: All changes come through Relay events
/// - **No Direct Access**: Use signals and streams for all access
/// 
/// # Examples
/// 
/// ```rust
/// use crate::actors::{ActorVec, relay, select};
/// 
/// let (add_item_relay, add_stream) = relay();
/// let (remove_item_relay, remove_stream) = relay();
/// 
/// let items = ActorVec::new(vec!["initial".to_string()], async move |items| {
///     loop {
///         select! {
///             Some(item) = add_stream.next() => {
///                 items.lock_mut().push_cloned(item);
///             }
///             Some(index) = remove_stream.next() => {
///                 if index < items.lock_ref().len() {
///                     items.lock_mut().remove(index);
///                 }
///             }
///         }
///     }
/// });
/// 
/// // Emit events
/// add_item_relay.send("new_item".to_string());
/// remove_item_relay.send(0);
/// 
/// // Bind to UI with efficient VecDiff updates
/// items.signal_vec()
/// ```
#[derive(Clone, Debug)]
pub struct ActorVec<T>
where
    T: Clone + Send + Sync + 'static,
{
    vec: MutableVec<T>,
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[allow(dead_code)]
    task_handle: Arc<TaskHandle>,
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[cfg(debug_assertions)]
    #[allow(dead_code)]
    creation_location: &'static std::panic::Location<'static>,
}

impl<T> ActorVec<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Create a new ActorVec with initial items and event processing loop.
    /// 
    /// The processor function should contain a loop that uses `select!`
    /// to handle multiple event streams and update the collection.
    /// 
    /// # Arguments
    /// 
    /// - `initial_items`: Starting items for this collection
    /// - `processor`: Async function that processes events and updates collection
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use crate::actors::{ActorVec, relay, select};
    /// use futures::StreamExt;
    /// 
    /// let (add_relay, mut add_stream) = relay();
    /// 
    /// let items = ActorVec::new(vec![], async move |items| {
    ///     while let Some(new_item) = add_stream.next().await {
    ///         items.lock_mut().push_cloned(new_item);
    ///     }
    /// });
    /// ```
    #[track_caller]
    pub fn new<F, Fut>(initial_items: Vec<T>, processor: F) -> Self
    where
        F: FnOnce(MutableVec<T>) -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        let vec = MutableVec::new_with_values(initial_items);

        // Start the async processor task with droppable handle
        let task_handle = Arc::new(Task::start_droppable(processor(vec.clone())));

        Self { 
            vec,
            task_handle,
            #[cfg(debug_assertions)]
            creation_location: std::panic::Location::caller(),
        }
    }


    /// Get an efficient VecDiff signal for reactive UI updates.
    /// 
    /// This is the preferred way to bind collections to UI as it only
    /// emits changes (additions, removals, updates) rather than the full
    /// collection on every change.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let items = ActorVec::new(vec![], /* processor */);
    /// 
    /// // Efficient UI binding with VecDiff
    /// El::new().children_signal_vec(
    ///     items.signal_vec().map(|item| {
    ///         El::new().child(Text::new(&item))
    ///     })
    /// )
    /// ```
    pub fn signal_vec(&self) -> impl zoon::SignalVec<Item = T> + use<T>{
        self.vec.signal_vec_cloned()
    }

    /// Get a reactive signal for the collection count.
    /// 
    /// Returns a signal that emits the number of items whenever the collection changes.
    /// Uses SignalVecExt::len() for optimal performance, avoiding full vector cloning.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let items = ActorVec::new(vec![1, 2, 3], /* processor */);
    /// 
    /// // Display reactive count
    /// Text::new().content_signal(
    ///     items.count_signal().map(|count| format!("{} items", count))
    /// )
    /// ```
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[allow(dead_code)]
    pub fn count_signal(&self) -> impl Signal<Item = usize> {
        self.vec.signal_vec_cloned().len()
    }



    /// Get a reactive signal indicating if the collection is empty.
    /// 
    /// Returns a signal that emits `true` when the collection has no items,
    /// `false` when it contains items. More semantic than checking count == 0.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let items = ActorVec::new(vec![], /* processor */);
    /// 
    /// // Show/hide UI elements based on empty state
    /// El::new().child_signal(
    ///     items.is_empty_signal().map(|is_empty| {
    ///         if is_empty {
    ///             Text::new("No items")
    ///         } else {
    ///             Text::new("Has items")
    ///         }
    ///     })
    /// )
    /// ```
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[allow(dead_code)]
    pub fn is_empty_signal(&self) -> impl Signal<Item = bool> {
        self.count_signal().map(|count| count == 0)
    }


}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::relay;
    use futures::{select, StreamExt};

    #[tokio::test]
    async fn test_actor_vec_basic_operations() {
        let (add_relay, mut add_stream) = relay();
        let (remove_relay, mut remove_stream) = relay();
        
        let items = ActorVec::new(vec!["initial".to_string()], async move |items| {
            loop {
                select! {
                    Some(item) = add_stream.next() => {
                        items.lock_mut().push_cloned(item);
                    }
                    Some(index) = remove_stream.next() => {
                        if index < items.lock_ref().len() {
                            items.lock_mut().remove(index);
                        }
                    }
                }
            }
        });

        // Wait for processor to start
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        add_relay.send("second".to_string());
        add_relay.send("third".to_string());

        // Wait for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let current_items = items.signal_vec().to_signal_cloned().to_stream().next().await.unwrap();
        assert_eq!(current_items, vec!["initial", "second", "third"]);

        remove_relay.send(1); // Remove "second"

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let final_items = items.signal_vec().to_signal_cloned().to_stream().next().await.unwrap();
        assert_eq!(final_items, vec!["initial", "third"]);
    }

    #[tokio::test]
    async fn test_actor_vec_handle_operations() {
        let (operation_relay, mut operation_stream) = relay();
        
        let items = ActorVec::new(vec![1, 2, 3], async move |items_handle| {
            while let Some(op) = operation_stream.next().await {
                match op.as_str() {
                    "clear" => items_handle.clear(),
                    "sort" => items_handle.update(|items| items.sort()),
                    "reverse" => items_handle.update(|items| items.reverse()),
                    _ => {}
                }
            }
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        operation_relay.send("reverse".to_string());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let reversed = items.signal_vec().to_signal_cloned().to_stream().next().await.unwrap();
        assert_eq!(reversed, vec![3, 2, 1]);

        operation_relay.send("clear".to_string());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let cleared = items.signal_vec().to_signal_cloned().to_stream().next().await.unwrap();
        assert!(cleared.is_empty());
    }

    #[tokio::test]
    async fn test_actor_vec_retain_functionality() {
        let (filter_relay, mut filter_stream) = relay();
        
        let items = ActorVec::new(vec![1, 2, 3, 4, 5], async move |items_handle| {
            while let Some(threshold) = filter_stream.next().await {
                items_handle.retain(|&item| item > threshold);
            }
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        filter_relay.send(3); // Keep only items > 3
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let filtered = items.signal_vec().to_signal_cloned().to_stream().next().await.unwrap();
        assert_eq!(filtered, vec![4, 5]);
    }
}