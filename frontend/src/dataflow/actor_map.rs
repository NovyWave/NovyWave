//! Reactive map Actor implementation
//!
//! ActorMap provides controlled key-value map management with sequential message processing.
//! Built on MutableBTreeMap for ordered, deterministic iteration and efficient MapDiff updates.

use zoon::{MutableBTreeMap, Signal, SignalVec, Task, TaskHandle, SignalMapExt, SignalExt, SignalVecExt};
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;

/// Reactive key-value map container for Actor+Relay architecture.
/// 
/// ActorMap controls all mutations to a BTreeMap through sequential
/// message processing. It provides reactive access to map contents,
/// individual key-value pairs, and efficient MapDiff updates.
/// 
/// Uses MutableBTreeMap internally for deterministic ordering and efficient reactivity.
/// 
/// # Core Principles
/// 
/// - **Sequential Processing**: Map updates processed one at a time
/// - **MapDiff Efficiency**: Efficient UI updates with only changed items  
/// - **Ordered Iteration**: Deterministic key ordering via BTreeMap
/// - **Event-Driven**: All changes come through Relay events
/// - **No Direct Access**: Use signals for all access
/// 
/// # Examples
/// 
/// ```rust
/// use crate::actors::{ActorMap, relay, select};
/// 
/// let (set_relay, set_stream) = relay();
/// let (remove_relay, remove_stream) = relay();
/// 
/// let cache = ActorMap::new(BTreeMap::new(), async move |map| {
///     loop {
///         select! {
///             Some((key, value)) = set_stream.next() => {
///                 map.lock_mut().insert_cloned(key, value);
///             }
///             Some(key) = remove_stream.next() => {
///                 map.lock_mut().remove(&key);
///             }
///         }
///     }
/// });
/// 
/// // Emit events
/// set_relay.send(("key1".to_string(), "value1".to_string()));
/// remove_relay.send("key1".to_string());
/// 
/// // Bind to UI with efficient MapDiff updates
/// cache.signal_map()  // Primary reactive method
/// cache.value_signal("specific_key")  // Individual key monitoring
/// ```
#[derive(Clone, Debug)]
pub struct ActorMap<K, V>
where
    K: Clone + Ord + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    map: MutableBTreeMap<K, V>,
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[allow(dead_code)]
    task_handle: Arc<TaskHandle>,
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[cfg(debug_assertions)]
    #[allow(dead_code)]
    creation_location: &'static std::panic::Location<'static>,
}

impl<K, V> ActorMap<K, V>
where
    K: Clone + Ord + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a new ActorMap with initial entries and event processing loop.
    /// 
    /// The processor function should contain a loop that uses `select!`
    /// to handle multiple event streams and update the map.
    /// 
    /// # Arguments
    /// 
    /// - `initial_map`: Starting key-value pairs for this map
    /// - `processor`: Async function that processes events and updates map
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use crate::actors::{ActorMap, relay, select};
    /// use futures::StreamExt;
    /// use std::collections::BTreeMap;
    /// 
    /// let (update_relay, mut update_stream) = relay();
    /// 
    /// let cache = ActorMap::new(BTreeMap::new(), async move |map| {
    ///     while let Some((key, value)) = update_stream.next().await {
    ///         map.lock_mut().insert_cloned(key, value);
    ///     }
    /// });
    /// ```
    #[track_caller]
    pub fn new<F, Fut>(initial_map: BTreeMap<K, V>, processor: F) -> Self
    where
        F: FnOnce(MutableBTreeMap<K, V>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let map = MutableBTreeMap::new();
        for (k, v) in initial_map {
            map.lock_mut().insert_cloned(k, v);
        }

        // Start the async processor task with droppable handle
        let task_handle = Arc::new(Task::start_droppable(processor(map.clone())));

        Self { 
            map,
            task_handle,
            #[cfg(debug_assertions)]
            creation_location: std::panic::Location::caller(),
        }
    }

    /// Get an efficient SignalMap for reactive UI updates.
    /// 
    /// This is the preferred way to bind maps to UI as it only
    /// emits changes (insertions, removals, updates) rather than the full
    /// map on every change.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let cache = ActorMap::new(BTreeMap::new(), /* processor */);
    /// 
    /// // Efficient UI binding with MapDiff
    /// cache.signal_map()
    /// ```
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[allow(dead_code)]
    pub fn signal_map(&self) -> impl SignalMapExt<Key = K, Value = V> {
        self.map.signal_map_cloned()
    }


    /// Get a signal for a specific key's value.
    /// 
    /// Returns a signal that emits `Option<V>` whenever the value for
    /// the given key changes (including insertions and removals).
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let cache = ActorMap::new(BTreeMap::new(), /* processor */);
    /// 
    /// // Monitor specific key's value
    /// cache.value_signal("important_key").map(|opt_value| {
    ///     match opt_value {
    ///         Some(value) => format!("Key exists: {}", value),
    ///         None => "Key not found".to_string(),
    ///     }
    /// })
    /// ```
    pub fn value_signal(&self, key: K) -> impl Signal<Item = Option<V>> {
        // Use key_cloned for efficient single key tracking
        self.map.signal_map_cloned().key_cloned(key)
    }

    /// Get an efficient SignalVec for all keys in the map.
    /// 
    /// Returns a SignalVec that emits key changes efficiently.
    /// Keys are in BTreeMap order (sorted).
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let cache = ActorMap::new(BTreeMap::new(), /* processor */);
    /// 
    /// // Efficient UI binding for keys
    /// cache.keys_signal_vec()
    /// ```
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[allow(dead_code)]
    pub fn keys_signal_vec(&self) -> impl SignalVec<Item = K> {
        // Use MutableBTreeMap's native signal_vec_keys() method
        self.map.signal_vec_keys()
    }

    /// Get an efficient SignalVec for all entries (key-value pairs) in the map.
    /// 
    /// Returns a SignalVec that emits entry changes efficiently.
    /// Entries are in key order (BTreeMap order).
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let cache = ActorMap::new(BTreeMap::new(), /* processor */);
    /// 
    /// // Efficient UI binding for entries
    /// cache.entries_signal_vec()
    /// ```
    #[allow(dead_code)] // Actor+Relay API method - preserve for completeness
    pub fn entries_signal_vec(&self) -> impl SignalVec<Item = (K, V)> {
        // Use MutableBTreeMap's native entries_cloned() method
        self.map.entries_cloned()
    }

    /// Get a reactive signal for the map count.
    /// 
    /// Returns a signal that emits the number of entries whenever the map changes.
    /// Uses SignalMapExt::len() for optimal performance, avoiding full map cloning.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let cache = ActorMap::new(BTreeMap::new(), /* processor */);
    /// 
    /// // Display reactive count
    /// Text::new().content_signal(
    ///     cache.count_signal().map(|count| format!("{} items", count))
    /// )
    /// ```
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[allow(dead_code)]
    pub fn count_signal(&self) -> impl Signal<Item = usize> {
        // Use signal_vec_keys().len() for efficient counting
        self.map.signal_vec_keys().len()
    }

    /// Get a reactive signal indicating if the map is empty.
    /// 
    /// Returns a signal that emits `true` when the map has no entries,
    /// `false` when it contains entries. More semantic than checking count == 0.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let cache = ActorMap::new(BTreeMap::new(), /* processor */);
    /// 
    /// // Show/hide UI elements based on empty state
    /// El::new().child_signal(
    ///     cache.is_empty_signal().map(|is_empty| {
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

    /// Check if the map contains a specific key reactively.
    /// 
    /// Returns a signal that emits whether the key exists.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let cache = ActorMap::new(BTreeMap::new(), /* processor */);
    /// 
    /// // Check key existence reactively
    /// cache.contains_key_signal("target_key")
    /// ```
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[allow(dead_code)]
    pub fn contains_key_signal(&self, key: K) -> impl Signal<Item = bool> {
        // TODO: Implement using SignalMap when available
        // For now, derive from value_signal
        self.value_signal(key).map(|opt| opt.is_some())
    }

}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::relay;
    use futures::{select, StreamExt};
    use std::collections::BTreeMap;

    #[tokio::test]
    async fn test_actor_map_basic_operations() {
        let (set_relay, mut set_stream) = relay();
        let (remove_relay, mut remove_stream) = relay();
        
        let mut initial_map = BTreeMap::new();
        initial_map.insert("initial".to_string(), 0);
        
        let cache = ActorMap::new(initial_map, async move |map| {
            loop {
                select! {
                    Some((key, value)) = set_stream.next() => {
                        map.lock_mut().insert_cloned(key, value);
                    }
                    Some(key) = remove_stream.next() => {
                        map.lock_mut().remove(&key);
                    }
                }
            }
        });

        // Wait for processor to start
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        set_relay.send(("key1".to_string(), 1));
        set_relay.send(("key2".to_string(), 2));

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Test count signal
        let current_count = cache.count_signal().to_stream().next().await.unwrap();
        assert_eq!(current_count, 3);

        // Test value signals
        let key1_value = cache.value_signal("key1".to_string()).to_stream().next().await.unwrap();
        let key2_value = cache.value_signal("key2".to_string()).to_stream().next().await.unwrap();
        assert_eq!(key1_value, Some(1));
        assert_eq!(key2_value, Some(2));

        remove_relay.send("key1".to_string());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let final_count = cache.count_signal().to_stream().next().await.unwrap();
        let key1_removed = cache.contains_key_signal("key1".to_string()).to_stream().next().await.unwrap();
        assert_eq!(final_count, 2);
        assert_eq!(key1_removed, false);
    }

    #[tokio::test]
    async fn test_actor_map_value_signal() {
        let (update_relay, mut update_stream) = relay();
        
        let cache = ActorMap::new(BTreeMap::new(), async move |map| {
            while let Some((key, value)) = update_stream.next().await {
                map.lock_mut().insert_cloned(key, value);
            }
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Initial value should be None
        let initial = cache.value_signal("test_key".to_string()).to_stream().next().await.unwrap();
        assert_eq!(initial, None);

        update_relay.send(("test_key".to_string(), "first_value".to_string()));
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        let first = cache.value_signal("test_key".to_string()).to_stream().next().await.unwrap();
        assert_eq!(first, Some("first_value".to_string()));

        update_relay.send(("test_key".to_string(), "updated_value".to_string()));
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        let updated = cache.value_signal("test_key".to_string()).to_stream().next().await.unwrap();
        assert_eq!(updated, Some("updated_value".to_string()));
    }

    #[tokio::test]
    async fn test_actor_map_handle_operations() {
        let (operation_relay, mut operation_stream) = relay();
        
        let mut initial = BTreeMap::new();
        initial.insert("counter".to_string(), 5);
        
        let cache = ActorMap::new(initial, async move |map| {
            while let Some(operation) = operation_stream.next().await {
                match operation.as_str() {
                    "increment" => {
                        let current = map.lock_ref().get("counter").cloned().unwrap_or(0);
                        map.lock_mut().insert_cloned("counter".to_string(), current + 1);
                    }
                    "clear" => map.clear(),
                    _ => {}
                }
            }
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        operation_relay.send("increment".to_string());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let incremented = cache.value_signal("counter".to_string()).to_stream().next().await.unwrap();
        assert_eq!(incremented, Some(6));

        operation_relay.send("clear".to_string());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let is_empty = cache.is_empty_signal().to_stream().next().await.unwrap();
        assert_eq!(is_empty, true);
    }
}