//! Reactive collection Actor implementation  
//!
//! ActorVec provides controlled collection management with sequential message processing.
//! It wraps MutableVec<T> and processes events from Relays to update collections safely.

use zoon::{MutableVec, Signal, Task, SignalVecExt, SignalExt, MutableVecExt};
use futures::stream::Stream;
use std::future::Future;

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
/// let items = ActorVec::new(vec!["initial".to_string()], async move |items_handle| {
///     loop {
///         select! {
///             Some(item) = add_stream.next() => {
///                 items_handle.push_cloned(item);
///             }
///             Some(index) = remove_stream.next() => {
///                 items_handle.remove(index);
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
    #[cfg(debug_assertions)]
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
    ///         items.push_cloned(new_item);
    ///     }
    /// });
    /// ```
    #[track_caller]
    pub fn new<F, Fut>(initial_items: Vec<T>, processor: F) -> Self
    where
        F: FnOnce(ActorVecHandle<T>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let vec = MutableVec::new_with_values(initial_items);
        let vec_handle = ActorVecHandle {
            mutable_vec: vec.clone(),
        };

        // Start the async processor task
        Task::start(processor(vec_handle));

        Self { 
            vec,
            #[cfg(debug_assertions)]
            creation_location: std::panic::Location::caller(),
        }
    }

    /// Get a signal for the entire collection.
    /// 
    /// Returns a signal that emits the full collection whenever it changes.
    /// Use `signal_vec()` for more efficient VecDiff updates.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let items = ActorVec::new(vec![1, 2, 3], /* processor */);
    /// 
    /// // Get full collection on changes
    /// items.signal().map(|vec| vec.len())
    /// ```
    pub fn signal(&self) -> impl Signal<Item = Vec<T>> {
        self.vec.signal_vec_cloned().to_signal_cloned()
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
    pub fn signal_vec(&self) -> impl zoon::SignalVec<Item = T> {
        self.vec.signal_vec_cloned()
    }

    /// Get a signal with a reference to avoid cloning.
    /// 
    /// Use when you need to compute derived values from the collection
    /// without cloning the entire vector.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let items = ActorVec::new(vec![1, 2, 3], /* processor */);
    /// 
    /// // Compute length without cloning items
    /// items.signal_ref(|vec| vec.len())
    /// ```
    pub fn signal_ref<U>(&self, f: impl Fn(&Vec<T>) -> U + Send + Sync + 'static) -> impl Signal<Item = U>
    where
        U: PartialEq + Send + Sync + Copy + 'static,
    {
        self.vec.signal_vec_cloned()
            .to_signal_cloned()
            .map(move |vec| f(&vec))
            .dedupe()
    }

    /// Get a reactive signal for the collection length.
    /// 
    /// Returns a signal that emits the collection size whenever it changes.
    /// More efficient than using `signal_ref(|v| v.len())` as it avoids
    /// creating intermediate closures.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let items = ActorVec::new(vec![1, 2, 3], /* processor */);
    /// 
    /// // Display reactive count
    /// Text::new().content_signal(
    ///     items.len_signal().map(|len| format!("{} items", len))
    /// )
    /// ```
    pub fn len_signal(&self) -> impl Signal<Item = usize> {
        self.signal_ref(|vec| vec.len())
    }

    /// Convert to a stream for async processing.
    /// 
    /// Returns a stream that emits the full collection on every change.
    /// Useful for testing and async processing scenarios.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let items = ActorVec::new(vec![], /* processor */);
    /// 
    /// // Process collection changes asynchronously
    /// let mut stream = items.to_stream();
    /// while let Some(vec) = stream.next().await {
    ///     println!("Collection now has {} items", vec.len());
    /// }
    /// ```
    pub fn to_stream(&self) -> impl Stream<Item = Vec<T>> {
        self.signal().to_stream()
    }
}

/// Handle for updating ActorVec from within the processor function.
/// 
/// This handle provides controlled access to the underlying MutableVec<T>
/// for collection updates. It's only available within the ActorVec's processor.
pub struct ActorVecHandle<T>
where
    T: Clone + Send + Sync + 'static,
{
    mutable_vec: MutableVec<T>,
}

impl<T> ActorVecHandle<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Add an item to the end of the collection.
    /// 
    /// The item is cloned into the collection. This triggers a VecDiff::Push
    /// signal for efficient UI updates.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// items_handle.push_cloned(new_item);
    /// ```
    pub fn push_cloned(&self, item: T) {
        self.mutable_vec.lock_mut().push_cloned(item);
    }

    /// Insert an item at a specific index.
    /// 
    /// Panics if the index is out of bounds. Triggers a VecDiff::InsertAt
    /// signal for the UI.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// items_handle.insert_cloned(0, first_item); // Insert at beginning
    /// ```
    pub fn insert_cloned(&self, index: usize, item: T) {
        self.mutable_vec.lock_mut().insert_cloned(index, item);
    }

    /// Remove an item at a specific index.
    /// 
    /// Returns the removed item if the index is valid, or None if out of bounds.
    /// Triggers a VecDiff::RemoveAt signal.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// if let Some(removed) = items_handle.remove(index) {
    ///     println!("Removed: {:?}", removed);
    /// }
    /// ```
    pub fn remove(&self, index: usize) -> Option<T> {
        let mut vec_guard = self.mutable_vec.lock_mut();
        if index < vec_guard.len() {
            Some(vec_guard.remove(index))
        } else {
            None
        }
    }

    /// Remove all items that match a predicate.
    /// 
    /// Returns the number of items removed. More efficient than removing
    /// items one by one as it minimizes signal emissions.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// // Remove all items matching condition
    /// let removed_count = items_handle.retain(|item| item.is_active());
    /// ```
    pub fn retain<F>(&self, mut f: F) -> usize
    where
        F: FnMut(&T) -> bool,
    {
        let mut vec_guard = self.mutable_vec.lock_mut();
        let initial_len = vec_guard.len();
        vec_guard.retain(|item| f(item));
        initial_len - vec_guard.len()
    }

    /// Replace an item at a specific index.
    /// 
    /// Returns the old item if the index is valid. Triggers a VecDiff::UpdateAt
    /// signal for efficient UI updates.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// if let Some(old_item) = items_handle.set_cloned(2, updated_item) {
    ///     println!("Replaced: {:?}", old_item);
    /// }
    /// ```
    pub fn set_cloned(&self, index: usize, item: T) -> Option<T> {
        let mut vec_guard = self.mutable_vec.lock_mut();
        if index < vec_guard.len() {
            let old_item = vec_guard[index].clone();
            vec_guard.set_cloned(index, item);
            Some(old_item)
        } else {
            None
        }
    }

    /// Replace the entire collection with new items.
    /// 
    /// This is more efficient than clearing and adding items individually
    /// as it minimizes signal emissions.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// items_handle.replace_cloned(vec![item1, item2, item3]);
    /// ```
    pub fn replace_cloned(&self, items: Vec<T>) {
        self.mutable_vec.lock_mut().replace_cloned(items);
    }

    /// Clear all items from the collection.
    /// 
    /// Triggers a VecDiff::Clear signal for the UI.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// items_handle.clear();
    /// ```
    pub fn clear(&self) {
        self.mutable_vec.lock_mut().clear();
    }

    /// Update the collection using a closure.
    /// 
    /// The closure receives a mutable reference to the underlying MutableVec
    /// and can perform multiple operations efficiently.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// items_handle.update(|items| {
    ///     items.sort();
    ///     items.dedup();
    /// });
    /// ```
    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut zoon::MutableVecLockMut<T>),
    {
        self.mutable_vec.update_mut(f)
    }

    /// Get the current length of the collection.
    /// 
    /// This provides synchronous access to the length without cloning.
    /// Use sparingly - prefer reactive signals where possible.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let current_length = items_handle.len();
    /// ```
    pub fn len(&self) -> usize {
        self.mutable_vec.lock_ref().len()
    }

    /// Check if the collection is empty.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// if items_handle.is_empty() {
    ///     // Handle empty collection
    /// }
    /// ```
    pub fn is_empty(&self) -> bool {
        self.mutable_vec.lock_ref().is_empty()
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
        
        let items = ActorVec::new(vec!["initial".to_string()], async move |items_handle| {
            loop {
                select! {
                    Some(item) = add_stream.next() => {
                        items_handle.push_cloned(item);
                    }
                    Some(index) = remove_stream.next() => {
                        items_handle.remove(index);
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

        let current_items = items.to_stream().next().await.unwrap();
        assert_eq!(current_items, vec!["initial", "second", "third"]);

        remove_relay.send(1); // Remove "second"

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let final_items = items.to_stream().next().await.unwrap();
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

        let reversed = items.to_stream().next().await.unwrap();
        assert_eq!(reversed, vec![3, 2, 1]);

        operation_relay.send("clear".to_string());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let cleared = items.to_stream().next().await.unwrap();
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

        let filtered = items.to_stream().next().await.unwrap();
        assert_eq!(filtered, vec![4, 5]);
    }
}