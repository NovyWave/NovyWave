//! Reactive map Actor implementation
//!
//! ActorMap provides controlled key-value map management with sequential message processing.
//! Built on std::collections::BTreeMap for ordered, deterministic iteration.

use zoon::{MutableBTreeMap, Mutable, Signal, Task, MutableBTreeMapExt, MutableExt};
use std::collections::BTreeMap;
use std::future::Future;

/// Reactive key-value map container for Actor+Relay architecture.
/// 
/// ActorMap controls all mutations to a BTreeMap through sequential
/// message processing. It provides reactive access to map contents
/// and individual key-value pairs.
/// 
/// Uses BTreeMap internally for deterministic ordering and iteration.
/// 
/// # Core Principles
/// 
/// - **Sequential Processing**: Map updates processed one at a time
/// - **Reactive Individual Keys**: Each key-value pair can be observed
/// - **Ordered Iteration**: Deterministic key ordering via BTreeMap
/// - **Event-Driven**: All changes come through Relay events
/// 
/// # Examples
/// 
/// ```rust
/// use crate::actors::{ActorMap, relay, select};
/// 
/// let (set_relay, set_stream) = relay();
/// let (remove_relay, remove_stream) = relay();
/// 
/// let cache = ActorMap::new(BTreeMap::new(), async move |map_handle| {
///     loop {
///         select! {
///             Some((key, value)) = set_stream.next() => {
///                 map_handle.insert(key, value);
///             }
///             Some(key) = remove_stream.next() => {
///                 map_handle.remove(&key);
///             }
///         }
///     }
/// });
/// 
/// // Emit events
/// set_relay.send(("key1".to_string(), "value1".to_string()));
/// remove_relay.send("key1".to_string());
/// 
/// // Bind to UI
/// cache.signal()  // Full map changes
/// cache.signal_key("specific_key")  // Individual key changes
/// ```
#[derive(Clone, Debug)]
pub struct ActorMap<K, V>
where
    K: Clone + Ord + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    map: MutableBTreeMap<K, V>,
    // Keep a Mutable copy for signal generation
    signal_map: Mutable<BTreeMap<K, V>>,
    #[cfg(debug_assertions)]
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
    ///         map.insert(key, value);
    ///     }
    /// });
    /// ```
    #[track_caller]
    pub fn new<F, Fut>(initial_map: BTreeMap<K, V>, processor: F) -> Self
    where
        F: FnOnce(ActorMapHandle<K, V>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let map = MutableBTreeMap::new();
        let signal_map = Mutable::new(initial_map.clone());
        for (k, v) in initial_map {
            map.lock_mut().insert_cloned(k, v);
        }
        let map_handle = ActorMapHandle {
            map: map.clone(),
            signal_map: signal_map.clone(),
        };

        // Start the async processor task
        Task::start(processor(map_handle));

        Self { 
            map,
            signal_map,
            #[cfg(debug_assertions)]
            creation_location: std::panic::Location::caller(),
        }
    }

    /// Get a signal for the entire map.
    /// 
    /// Returns a signal that emits the full map whenever it changes.
    /// Use `signal_key()` for more efficient individual key monitoring.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let cache = ActorMap::new(BTreeMap::new(), /* processor */);
    /// 
    /// // Get full map on changes
    /// cache.signal().map(|map| map.len())
    /// ```
    pub fn signal(&self) -> impl Signal<Item = BTreeMap<K, V>> {
        self.signal_map.signal_cloned()
    }

    /// Get a signal for a specific key.
    /// 
    /// Returns a signal that emits `Option<V>` whenever the value for
    /// the given key changes (including insertions and removals).
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let cache = ActorMap::new(BTreeMap::new(), /* processor */);
    /// 
    /// // Monitor specific key
    /// cache.signal_key("important_key").map(|opt_value| {
    ///     match opt_value {
    ///         Some(value) => format!("Key exists: {}", value),
    ///         None => "Key not found".to_string(),
    ///     }
    /// })
    /// ```
    pub fn signal_key(&self, key: K) -> impl Signal<Item = Option<V>> {
        self.signal_map.signal_ref(move |map| map.get(&key).cloned())
    }

    /// Get a signal for all keys in the map.
    /// 
    /// Returns a signal that emits a Vec of all keys whenever the
    /// map changes. Keys are in BTreeMap order (sorted).
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let cache = ActorMap::new(BTreeMap::new(), /* processor */);
    /// 
    /// // Get sorted list of keys
    /// cache.signal_keys()
    /// ```
    pub fn signal_keys(&self) -> impl Signal<Item = Vec<K>> {
        self.signal_map.signal_ref(|map| map.keys().cloned().collect())
    }

    /// Get a signal for all values in the map.
    /// 
    /// Returns a signal that emits a Vec of all values whenever the
    /// map changes. Values are in key order (BTreeMap order).
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let cache = ActorMap::new(BTreeMap::new(), /* processor */);
    /// 
    /// // Get values in key order
    /// cache.signal_values()
    /// ```
    pub fn signal_values(&self) -> impl Signal<Item = Vec<V>> {
        self.signal_map.signal_ref(|map| map.values().cloned().collect())
    }

    /// Get a signal for the number of entries.
    /// 
    /// Returns a signal that emits the map size whenever it changes.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let cache = ActorMap::new(BTreeMap::new(), /* processor */);
    /// 
    /// // Monitor map size
    /// cache.signal_len()
    /// ```
    pub fn signal_len(&self) -> impl Signal<Item = usize> {
        self.signal_map.signal_ref(|map| map.len())
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
    /// cache.signal_contains_key("target_key")
    /// ```
    pub fn signal_contains_key(&self, key: K) -> impl Signal<Item = bool> {
        self.signal_map.signal_ref(move |map| map.contains_key(&key))
    }

    /// Get a reference signal for efficient map transformations.
    /// 
    /// Allows computing derived values from the map without cloning
    /// the entire BTreeMap.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let cache = ActorMap::new(BTreeMap::new(), /* processor */);
    /// 
    /// // Compute total sum without cloning map
    /// let total = cache.signal_map_ref(|map| {
    ///     map.values().sum::<i32>()
    /// });
    /// 
    /// // Count entries matching condition
    /// let active_count = cache.signal_map_ref(|map| {
    ///     map.values().filter(|v| v.is_active()).count()
    /// });
    /// ```
    pub fn signal_map_ref<U>(&self, f: impl Fn(&BTreeMap<K, V>) -> U + Send + Sync + 'static) -> impl Signal<Item = U>
    where
        U: PartialEq + Send + Sync + 'static,
    {
        self.signal_map.signal_ref(f)
    }
}

/// Handle for updating ActorMap from within the processor function.
/// 
/// This handle provides controlled access to the underlying BTreeMap
/// for map updates. It's only available within the ActorMap's processor.
pub struct ActorMapHandle<K, V>
where
    K: Clone + Ord + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    map: MutableBTreeMap<K, V>,
    signal_map: Mutable<BTreeMap<K, V>>,
}

impl<K, V> ActorMapHandle<K, V>
where
    K: Clone + Ord + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Insert a key-value pair into the map.
    /// 
    /// Returns the previous value if the key existed, or None if it's new.
    /// Triggers reactive signals for the full map and the specific key.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let old_value = map_handle.insert("key".to_string(), "value".to_string());
    /// ```
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        self.map.lock_mut().insert_cloned(key, value)
    }

    /// Remove a key-value pair from the map.
    /// 
    /// Returns the removed value if the key existed, or None if not found.
    /// Triggers reactive signals for the full map and the specific key.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// if let Some(removed_value) = map_handle.remove(&"key".to_string()) {
    ///     println!("Removed: {:?}", removed_value);
    /// }
    /// ```
    pub fn remove(&self, key: &K) -> Option<V> {
        self.map.lock_mut().remove(key)
    }

    /// Update a value if the key exists.
    /// 
    /// The closure receives the current value and returns the new value.
    /// Returns true if the key existed and was updated, false otherwise.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// // Increment a counter value
    /// let updated = map_handle.update(&"counter".to_string(), |count| count + 1);
    /// ```
    pub fn update<F>(&self, key: &K, f: F) -> bool
    where
        F: FnOnce(V) -> V,
    {
        let mut map_guard = self.map.lock_mut();
        if let Some(current_value) = map_guard.remove(key) {
            let new_value = f(current_value);
            map_guard.insert_cloned(key.clone(), new_value.clone());
            self.signal_map.lock_mut().insert(key.clone(), new_value);
            true
        } else {
            false
        }
    }

    /// Insert a key-value pair only if the key doesn't exist.
    /// 
    /// Returns true if the value was inserted, false if key already existed.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let was_inserted = map_handle.insert_if_missing("new_key".to_string(), "value".to_string());
    /// ```
    pub fn insert_if_missing(&self, key: K, value: V) -> bool {
        let mut map_guard = self.map.lock_mut();
        if map_guard.contains_key(&key) {
            false
        } else {
            map_guard.insert_cloned(key.clone(), value.clone());
            self.signal_map.lock_mut().insert(key, value);
            true
        }
    }

    /// Clear all entries from the map.
    /// 
    /// Triggers reactive signals for the full map and all individual keys.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// map_handle.clear();
    /// ```
    pub fn clear(&self) {
        self.map.lock_mut().clear();
    }

    /// Replace the entire map with new entries.
    /// 
    /// More efficient than clearing and inserting individually as it
    /// minimizes signal emissions.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let new_map = BTreeMap::from([
    ///     ("key1".to_string(), "value1".to_string()),
    ///     ("key2".to_string(), "value2".to_string()),
    /// ]);
    /// map_handle.replace(new_map);
    /// ```
    pub fn replace(&self, new_map: BTreeMap<K, V>) {
        let mut map_guard = self.map.lock_mut();
        map_guard.clear();
        for (k, v) in new_map.clone() {
            map_guard.insert_cloned(k, v);
        }
        self.signal_map.set(new_map);
    }

    /// Update the map using a closure.
    /// 
    /// The closure receives a mutable reference to the underlying BTreeMap
    /// and can perform multiple operations efficiently.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// map_handle.update_map(|map| {
    ///     map.insert("key1".to_string(), "value1".to_string());
    ///     map.insert("key2".to_string(), "value2".to_string());
    ///     map.remove(&"old_key".to_string());
    /// });
    /// ```
    pub fn update_map<F>(&self, f: F)
    where
        F: FnOnce(&mut BTreeMap<K, V>),
    {
        let mut guard = self.map.lock_mut();
        let mut temp_map = guard.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<BTreeMap<_, _>>();
        f(&mut temp_map);
        guard.clear();
        for (k, v) in temp_map {
            guard.insert_cloned(k, v);
        }
    }

    /// Get the current number of entries.
    /// 
    /// Provides synchronous access to the map size without cloning.
    /// Use sparingly - prefer reactive signals where possible.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let current_size = map_handle.len();
    /// ```
    pub fn len(&self) -> usize {
        self.map.lock_ref().len()
    }

    /// Check if the map is empty.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// if map_handle.is_empty() {
    ///     // Handle empty map
    /// }
    /// ```
    pub fn is_empty(&self) -> bool {
        self.map.lock_ref().is_empty()
    }

    /// Check if a key exists in the map.
    /// 
    /// Provides synchronous access for conditional logic.
    /// Use sparingly - prefer reactive signals where possible.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// if map_handle.contains_key(&"target_key".to_string()) {
    ///     // Key exists
    /// }
    /// ```
    pub fn contains_key(&self, key: &K) -> bool {
        self.map.lock_ref().contains_key(key)
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
        
        let cache = ActorMap::new(initial_map, async move |map_handle| {
            loop {
                select! {
                    Some((key, value)) = set_stream.next() => {
                        map_handle.insert(key, value);
                    }
                    Some(key) = remove_stream.next() => {
                        map_handle.remove(&key);
                    }
                }
            }
        });

        // Wait for processor to start
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        set_relay.send(("key1".to_string(), 1));
        set_relay.send(("key2".to_string(), 2));

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let current_map = cache.signal().to_stream().next().await.unwrap();
        assert_eq!(current_map.len(), 3);
        assert_eq!(current_map.get("key1"), Some(&1));
        assert_eq!(current_map.get("key2"), Some(&2));

        remove_relay.send("key1".to_string());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let final_map = cache.signal().to_stream().next().await.unwrap();
        assert_eq!(final_map.len(), 2);
        assert!(!final_map.contains_key("key1"));
    }

    #[tokio::test]
    async fn test_actor_map_signal_key() {
        let (update_relay, mut update_stream) = relay();
        
        let cache = ActorMap::new(BTreeMap::new(), async move |map_handle| {
            while let Some((key, value)) = update_stream.next().await {
                map_handle.insert(key, value);
            }
        });

        let key_signal = cache.signal_key("test_key".to_string());
        let mut key_stream = key_signal.to_stream();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Initial value should be None
        let initial = key_stream.next().await.unwrap();
        assert_eq!(initial, None);

        update_relay.send(("test_key".to_string(), "first_value".to_string()));
        let first = key_stream.next().await.unwrap();
        assert_eq!(first, Some("first_value".to_string()));

        update_relay.send(("test_key".to_string(), "updated_value".to_string()));
        let updated = key_stream.next().await.unwrap();
        assert_eq!(updated, Some("updated_value".to_string()));
    }

    #[tokio::test]
    async fn test_actor_map_handle_operations() {
        let (operation_relay, mut operation_stream) = relay();
        
        let mut initial = BTreeMap::new();
        initial.insert("counter".to_string(), 5);
        
        let cache = ActorMap::new(initial, async move |map_handle| {
            while let Some(operation) = operation_stream.next().await {
                match operation.as_str() {
                    "increment" => {
                        map_handle.update(&"counter".to_string(), |count| count + 1);
                    }
                    "clear" => map_handle.clear(),
                    _ => {}
                }
            }
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        operation_relay.send("increment".to_string());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let incremented = cache.signal_key("counter".to_string()).to_stream().next().await.unwrap();
        assert_eq!(incremented, Some(6));

        operation_relay.send("clear".to_string());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let cleared = cache.signal().to_stream().next().await.unwrap();
        assert!(cleared.is_empty());
    }
}