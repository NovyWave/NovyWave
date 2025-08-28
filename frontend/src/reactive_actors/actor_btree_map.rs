//! ActorBTreeMap<K,V> - Reactive Ordered Map Management
//!
//! Reactive ordered map container for Actor+Relay architecture.
//! Provides controlled map mutations through event processing with ordering guarantees.

use zoon::*;
use std::collections::BTreeMap;

/// Reactive ordered map container that processes key-value events sequentially.
/// 
/// ActorBTreeMap provides the same guarantees as Actor but for ordered key-value
/// collections. All mutations happen through event processing, with reactive 
/// signals for efficient UI updates.
/// 
/// ## Usage Pattern
/// 
/// ```rust
/// use crate::reactive_actors::{ActorBTreeMap, relay, select};
/// 
/// // Create event streams
/// let (value_updated_relay, mut update_stream) = relay();
/// let (key_removed_relay, mut remove_stream) = relay();
/// 
/// // Create ActorBTreeMap with event processing
/// let signal_values = ActorBTreeMap::new(BTreeMap::new(), async move |map| {
///     loop {
///         select! {
///             Some((key, value)) = update_stream.next() => {
///                 map.lock_mut().insert_cloned(key, value);
///             }
///             Some(key) = remove_stream.next() => {
///                 map.lock_mut().remove(&key);
///             }
///         }
///     }
/// });
/// 
/// // Access map reactively
/// signal_values.signal_cloned()     // Returns Signal<Item = BTreeMap<K,V>>
/// signal_values.entries_signal()    // Returns SignalVec of key-value pairs
/// ```
/// 
/// ## Domain Examples
/// 
/// ```rust
/// // Signal cursor values by variable ID
/// let cursor_values = ActorBTreeMap::new(BTreeMap::new(), async move |values_map| {
///     loop {
///         select! {
///             Some((var_id, cursor_pos)) = cursor_moved_stream.next() => {
///                 // Look up signal value at cursor position
///                 if let Some(value) = get_signal_value_at_time(&var_id, cursor_pos) {
///                     values_map.lock_mut().insert_cloned(var_id, value);
///                 }
///             }
///             Some(var_id) = variable_removed_stream.next() => {
///                 values_map.lock_mut().remove(&var_id);
///             }
///             Some(()) = values_invalidated_stream.next() => {
///                 values_map.lock_mut().clear();
///             }
///         }
///     }
/// });
/// 
/// // Scope expansion state by scope ID
/// let expanded_scopes = ActorBTreeMap::new(BTreeMap::new(), async move |scopes_map| {
///     while let Some((scope_id, is_expanded)) = scope_toggle_stream.next().await {
///         if is_expanded {
///             scopes_map.lock_mut().insert_cloned(scope_id, true);
///         } else {
///             scopes_map.lock_mut().remove(&scope_id);
///         }
///     }
/// });
/// ```
#[derive(Clone)]
pub struct ActorBTreeMap<K, V> {
    map: Mutable<BTreeMap<K, V>>,
}

impl<K, V> ActorBTreeMap<K, V>
where
    K: Clone + Ord + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Creates a new ActorBTreeMap with initial entries and event processor.
    /// 
    /// The event processor runs continuously, handling map operations
    /// sequentially to prevent race conditions.
    /// 
    /// # Arguments
    /// 
    /// * `initial` - Initial key-value pairs in the map
    /// * `processor` - Async function that processes events and updates the map
    /// 
    /// # Example
    /// 
    /// ```rust
    /// let (entry_updated_relay, mut update_stream) = relay();
    /// let (entry_removed_relay, mut remove_stream) = relay();
    /// 
    /// let config_values = ActorBTreeMap::new(
    ///     BTreeMap::from([("theme".to_string(), "dark".to_string())]),
    ///     async move |config_map| {
    ///         loop {
    ///             select! {
    ///                 Some((key, value)) = update_stream.next() => {
    ///                     config_map.lock_mut().insert_cloned(key, value);
    ///                     save_config_to_disk(&*config_map.lock_ref());
    ///                 }
    ///                 Some(key) = remove_stream.next() => {
    ///                     config_map.lock_mut().remove(&key);
    ///                     save_config_to_disk(&*config_map.lock_ref());
    ///                 }
    ///             }
    ///         }
    ///     }
    /// );
    /// ```
    pub fn new<F, Fut>(initial: BTreeMap<K, V>, processor: F) -> Self
    where
        F: FnOnce(Mutable<BTreeMap<K, V>>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let map = Mutable::new(initial);
        let map_clone = map.clone();
        
        // Start event processor task
        Task::start(processor(map_clone));
        
        ActorBTreeMap { map }
    }
    
    /// Returns a reactive signal for the entire map.
    /// 
    /// The signal updates automatically when the map changes through event processing.
    /// Use this for reactive UI binding when you need the complete map.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// // Bind to UI for configuration display
    /// Column::new()
    ///     .items_signal(
    ///         config_values.signal_cloned()
    ///             .map(|config_map| {
    ///                 config_map.iter()
    ///                     .map(|(key, value)| config_entry_view(key, value))
    ///                     .collect::<Vec<_>>()
    ///             })
    ///     )
    /// 
    /// // Check if map contains specific key
    /// let has_dark_theme = config_values.signal_cloned()
    ///     .map(|config| config.contains_key("theme"));
    /// ```
    pub fn signal_cloned(&self) -> impl Signal<Item = BTreeMap<K, V>> {
        self.map.signal_cloned()
    }
    
    /// Returns a signal reference for efficient access without cloning.
    /// 
    /// Use this when you need to access map values without cloning the entire map.
    pub fn signal_ref<U, F>(&self, f: F) -> impl Signal<Item = U>
    where
        F: Fn(&BTreeMap<K, V>) -> U + Send + Sync + 'static,
        U: Send + Sync + 'static,
    {
        self.map.signal_ref(f)
    }
    
    /// Returns a reactive signal for map length.
    pub fn len_signal(&self) -> impl Signal<Item = usize> {
        self.signal_ref(|map| map.len())
    }
    
    /// Returns a reactive signal indicating if the map is empty.
    pub fn is_empty_signal(&self) -> impl Signal<Item = bool> {
        self.len_signal().map(|len| len == 0)
    }
    
    /// Returns a reactive signal for a specific key's value.
    /// 
    /// The signal emits None when the key doesn't exist, Some(value) when it does.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// // Track specific signal value reactively
    /// let variable_value = signal_values.key_signal("scope.variable_name");
    /// 
    /// // Bind to UI
    /// Text::new_signal(
    ///     variable_value.map(|opt_value| {
    ///         opt_value.unwrap_or_else(|| "N/A".to_string())
    ///     })
    /// )
    /// ```
    pub fn key_signal(&self, key: K) -> impl Signal<Item = Option<V>>
    where
        K: Clone,
    {
        self.signal_ref(move |map| map.get(&key).cloned())
    }
    
    /// Returns a reactive signal for all keys in the map.
    /// 
    /// Keys are returned in sorted order (BTreeMap guarantee).
    pub fn keys_signal(&self) -> impl Signal<Item = Vec<K>> {
        self.signal_ref(|map| map.keys().cloned().collect())
    }
    
    /// Returns a reactive signal for all values in the map.
    /// 
    /// Values are returned in key-sorted order.
    pub fn values_signal(&self) -> impl Signal<Item = Vec<V>> {
        self.signal_ref(|map| map.values().cloned().collect())
    }
}

// Note: ActorBTreeMap<K,V> intentionally does NOT provide direct access methods
// All access must be through signals to prevent race conditions and maintain
// architectural consistency

impl<K, V> std::fmt::Debug for ActorBTreeMap<K, V>
where
    K: std::fmt::Debug,
    V: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorBTreeMap")
            .field("entries_count", &self.map.lock_ref().len())
            .field("current_map", &*self.map.lock_ref())
            .finish()
    }
}

/// Map update interface for use within ActorBTreeMap event processors.
/// 
/// This trait is automatically implemented for `Mutable<BTreeMap<K,V>>` and provides
/// the methods available for updating ActorBTreeMap from within event processors.
pub trait ActorBTreeMapState<K, V> {
    /// Inserts a key-value pair into the map.
    fn insert_cloned(&self, key: K, value: V) -> Option<V>;
    
    /// Removes a key from the map, returning the value if it existed.
    fn remove(&self, key: &K) -> Option<V>;
    
    /// Removes all entries from the map.
    fn clear(&self);
    
    /// Retains only the entries that match the predicate.
    fn retain<F>(&self, f: F)
    where
        F: FnMut(&K, &mut V) -> bool;
        
    /// Replaces the entire map with new entries.
    fn replace_cloned(&self, map: BTreeMap<K, V>);
    
    /// Updates the map using a function.
    fn update<F>(&self, f: F)
    where
        F: FnOnce(BTreeMap<K, V>) -> BTreeMap<K, V>;
}

impl<K, V> ActorBTreeMapState<K, V> for Mutable<BTreeMap<K, V>>
where
    K: Clone + Ord,
    V: Clone,
{
    fn insert_cloned(&self, key: K, value: V) -> Option<V> {
        self.lock_mut().insert(key, value)
    }
    
    fn remove(&self, key: &K) -> Option<V> {
        self.lock_mut().remove(key)
    }
    
    fn clear(&self) {
        self.lock_mut().clear();
    }
    
    fn retain<F>(&self, mut f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.lock_mut().retain(|k, v| f(k, v));
    }
    
    fn replace_cloned(&self, map: BTreeMap<K, V>) {
        *self.lock_mut() = map;
    }
    
    fn update<F>(&self, f: F)
    where
        F: FnOnce(BTreeMap<K, V>) -> BTreeMap<K, V>,
    {
        self.update(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive_actors::relay;
    use futures::{select, StreamExt};
    use std::collections::BTreeMap;
    
    #[async_test]
    async fn test_actor_btree_map_basic_operations() {
        let (update_relay, mut update_stream) = relay();
        let (remove_relay, mut remove_stream) = relay();
        
        let initial_map = BTreeMap::from([
            ("a".to_string(), 1),
            ("b".to_string(), 2),
        ]);
        
        let map = ActorBTreeMap::new(initial_map, async move |map_mutable| {
            loop {
                select! {
                    Some((key, value)) = update_stream.next() => {
                        map_mutable.insert_cloned(key, value);
                    }
                    Some(key) = remove_stream.next() => {
                        map_mutable.remove(&key);
                    }
                }
            }
        });
        
        // Test initial state
        let initial_state = map.signal_cloned().to_stream().next().await.unwrap();
        assert_eq!(initial_state.len(), 2);
        assert_eq!(initial_state.get("a"), Some(&1));
        assert_eq!(initial_state.get("b"), Some(&2));
        
        // Add new entry
        update_relay.send(("c".to_string(), 3));
        let updated_state = map.signal_cloned().to_stream().next().await.unwrap();
        assert_eq!(updated_state.len(), 3);
        assert_eq!(updated_state.get("c"), Some(&3));
        
        // Update existing entry
        update_relay.send(("a".to_string(), 10));
        let modified_state = map.signal_cloned().to_stream().next().await.unwrap();
        assert_eq!(modified_state.get("a"), Some(&10));
        
        // Remove entry
        remove_relay.send("b".to_string());
        let final_state = map.signal_cloned().to_stream().next().await.unwrap();
        assert_eq!(final_state.len(), 2);
        assert!(!final_state.contains_key("b"));
    }
    
    #[async_test]
    async fn test_actor_btree_map_key_signal() {
        let (update_relay, mut update_stream) = relay();
        
        let map = ActorBTreeMap::new(
            BTreeMap::from([("key1".to_string(), "value1".to_string())]),
            async move |map_mutable| {
                while let Some((key, value)) = update_stream.next().await {
                    map_mutable.insert_cloned(key, value);
                }
            }
        );
        
        // Test key signal for existing key
        let key1_signal = map.key_signal("key1".to_string());
        let initial_value = key1_signal.to_stream().next().await.unwrap();
        assert_eq!(initial_value, Some("value1".to_string()));
        
        // Test key signal for non-existing key
        let key2_signal = map.key_signal("key2".to_string());
        let missing_value = key2_signal.to_stream().next().await.unwrap();
        assert_eq!(missing_value, None);
        
        // Update key and verify signal updates
        update_relay.send(("key1".to_string(), "updated_value".to_string()));
        let updated_value = key1_signal.to_stream().next().await.unwrap();
        assert_eq!(updated_value, Some("updated_value".to_string()));
        
        // Add new key and verify its signal
        update_relay.send(("key2".to_string(), "new_value".to_string()));
        let new_value = key2_signal.to_stream().next().await.unwrap();
        assert_eq!(new_value, Some("new_value".to_string()));
    }
    
    #[async_test]
    async fn test_actor_btree_map_length_and_empty_signals() {
        let (update_relay, mut update_stream) = relay();
        let (clear_relay, mut clear_stream) = relay();
        
        let map = ActorBTreeMap::new(BTreeMap::new(), async move |map_mutable| {
            loop {
                select! {
                    Some((key, value)) = update_stream.next() => {
                        map_mutable.insert_cloned(key, value);
                    }
                    Some(()) = clear_stream.next() => {
                        map_mutable.clear();
                    }
                }
            }
        });
        
        // Test initial empty state
        let initial_len = map.len_signal().to_stream().next().await.unwrap();
        let initial_empty = map.is_empty_signal().to_stream().next().await.unwrap();
        assert_eq!(initial_len, 0);
        assert_eq!(initial_empty, true);
        
        // Add entries and test length
        update_relay.send((1, "one".to_string()));
        update_relay.send((2, "two".to_string()));
        
        let new_len = map.len_signal().to_stream().next().await.unwrap();
        let new_empty = map.is_empty_signal().to_stream().next().await.unwrap();
        assert_eq!(new_len, 2);
        assert_eq!(new_empty, false);
        
        // Clear and test empty state
        clear_relay.send(());
        let final_len = map.len_signal().to_stream().next().await.unwrap();
        let final_empty = map.is_empty_signal().to_stream().next().await.unwrap();
        assert_eq!(final_len, 0);
        assert_eq!(final_empty, true);
    }
}