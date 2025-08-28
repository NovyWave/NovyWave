//! ActorVec<T> - Reactive Collection Management
//!
//! Reactive vector container for Actor+Relay architecture.
//! Provides controlled collection mutations through event processing.

use zoon::*;

/// Reactive vector container that processes collection events sequentially.
/// 
/// ActorVec provides the same guarantees as Actor but for collections.
/// All mutations happen through event processing, with reactive signals
/// for efficient UI updates.
/// 
/// ## Usage Pattern
/// 
/// ```rust
/// use crate::reactive_actors::{ActorVec, relay, select};
/// 
/// // Create event streams
/// let (file_dropped_relay, mut file_dropped_stream) = relay();
/// let (file_removed_relay, mut file_removed_stream) = relay();
/// 
/// // Create ActorVec with event processing
/// let tracked_files = ActorVec::new(vec![], async move |files_vec| {
///     loop {
///         select! {
///             Some(paths) = file_dropped_stream.next() => {
///                 for path in paths {
///                     let tracked_file = TrackedFile::new(path);
///                     files_vec.lock_mut().push_cloned(tracked_file);
///                 }
///             }
///             Some(file_id) = file_removed_stream.next() => {
///                 files_vec.lock_mut().retain(|f| f.id != file_id);
///             }
///         }
///     }
/// });
/// 
/// // Access collection reactively
/// tracked_files.signal_vec_cloned() // Returns SignalVec<Item = TrackedFile>
/// tracked_files.signal_vec()        // Returns SignalVec for efficient updates
/// ```
/// 
/// ## Domain Examples
/// 
/// ```rust
/// // Selected variables for timeline display
/// let selected_variables = ActorVec::new(vec![], async move |vars_vec| {
///     loop {
///         select! {
///             Some(var_id) = variable_clicked_stream.next() => {
///                 // Add if not already selected
///                 let already_selected = vars_vec.lock_ref().iter()
///                     .any(|v| v.unique_id == var_id);
///                 if !already_selected {
///                     if let Some(variable) = find_variable_by_id(&var_id) {
///                         vars_vec.lock_mut().push_cloned(variable);
///                     }
///                 }
///             }
///             Some(var_id) = variable_removed_stream.next() => {
///                 vars_vec.lock_mut().retain(|v| v.unique_id != var_id);
///             }
///             Some(()) = clear_selection_stream.next() => {
///                 vars_vec.lock_mut().clear();
///             }
///         }
///     }
/// });
/// 
/// // File loading queue with status tracking
/// let loading_files = ActorVec::new(vec![], async move |files_vec| {
///     while let Some(file_path) = file_load_stream.next().await {
///         let loading_file = LoadingFile::new(file_path.clone());
///         files_vec.lock_mut().push_cloned(loading_file);
///         
///         // Start parsing in background
///         let files_vec_clone = files_vec.clone();
///         Task::start(async move {
///             match parse_waveform_file(&file_path).await {
///                 Ok(result) => {
///                     // Update file status to completed
///                     files_vec_clone.lock_mut().iter_mut()
///                         .find(|f| f.path == file_path)
///                         .map(|f| f.status = LoadingStatus::Completed(result));
///                 }
///                 Err(error) => {
///                     // Update file status to error
///                     files_vec_clone.lock_mut().iter_mut()
///                         .find(|f| f.path == file_path)
///                         .map(|f| f.status = LoadingStatus::Error(error));
///                 }
///             }
///         });
///     }
/// });
/// ```
#[derive(Clone)]
pub struct ActorVec<T> {
    items: MutableVec<T>,
}

impl<T> ActorVec<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Creates a new ActorVec with initial items and event processor.
    /// 
    /// The event processor runs continuously, handling collection operations
    /// sequentially to prevent race conditions.
    /// 
    /// # Arguments
    /// 
    /// * `initial` - Initial items in the collection
    /// * `processor` - Async function that processes events and updates the collection
    /// 
    /// # Example
    /// 
    /// ```rust
    /// let (item_added_relay, mut item_stream) = relay();
    /// let (item_removed_relay, mut remove_stream) = relay();
    /// 
    /// let items = ActorVec::new(vec![initial_item], async move |items_vec| {
    ///     loop {
    ///         select! {
    ///             Some(new_item) = item_stream.next() => {
    ///                 items_vec.lock_mut().push_cloned(new_item);
    ///             }
    ///             Some(item_id) = remove_stream.next() => {
    ///                 items_vec.lock_mut().retain(|item| item.id != item_id);
    ///             }
    ///         }
    ///     }
    /// });
    /// ```
    pub fn new<F, Fut>(initial: Vec<T>, processor: F) -> Self
    where
        F: FnOnce(MutableVec<T>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let items = MutableVec::new_with_values(initial);
        let items_clone = items.clone();
        
        // Start event processor task
        Task::start(processor(items_clone));
        
        ActorVec { items }
    }
    
    /// Returns a reactive signal vector for this ActorVec's items.
    /// 
    /// SignalVec emits VecDiff operations for efficient collection updates.
    /// Use this for reactive UI binding where you need individual item updates.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// // Bind to UI with efficient updates
    /// Column::new()
    ///     .items_signal_vec(
    ///         tracked_files.signal_vec_cloned()
    ///             .map(|file| file_item_view(file))
    ///     )
    /// 
    /// // Filter and transform
    /// let visible_files = tracked_files.signal_vec_cloned()
    ///     .filter_map(|file| {
    ///         if file.is_visible() {
    ///             Some(file)
    ///         } else {
    ///             None
    ///         }
    ///     });
    /// ```
    pub fn signal_vec_cloned(&self) -> impl SignalVec<Item = T> {
        self.items.signal_vec_cloned()
    }
    
    /// Returns a reactive signal vector with references to items.
    /// 
    /// More efficient than signal_vec_cloned() when items are large
    /// and you don't need to own them.
    pub fn signal_vec(&self) -> impl SignalVec<Item = T> {
        self.items.signal_vec()
    }
    
    /// Returns a reactive signal for the entire collection as a Vec.
    /// 
    /// Use this when you need the entire collection as a single value,
    /// such as for computing derived data or counts.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// // Get count reactively
    /// let file_count = tracked_files.to_signal_cloned()
    ///     .map(|files| files.len());
    /// 
    /// // Compute derived state
    /// let has_errors = tracked_files.to_signal_cloned()
    ///     .map(|files| files.iter().any(|f| f.has_error()));
    /// ```
    pub fn to_signal_cloned(&self) -> impl Signal<Item = Vec<T>> {
        self.items.signal_vec_cloned().to_signal_cloned()
    }
    
    /// Returns a length signal for reactive count display.
    /// 
    /// More efficient than converting to Vec and getting length.
    pub fn len_signal(&self) -> impl Signal<Item = usize> {
        self.signal_vec_cloned()
            .to_signal_cloned()
            .map(|items| items.len())
    }
    
    /// Returns a signal indicating if the collection is empty.
    pub fn is_empty_signal(&self) -> impl Signal<Item = bool> {
        self.len_signal().map(|len| len == 0)
    }
}

// Note: ActorVec<T> intentionally does NOT provide direct access methods like .get()
// All access must be through signals to prevent race conditions and maintain
// architectural consistency

impl<T> std::fmt::Debug for ActorVec<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorVec")
            .field("items_count", &self.items.lock_ref().len())
            .finish()
    }
}

/// Collection update interface for use within ActorVec event processors.
/// 
/// This trait is automatically implemented for `MutableVec<T>` and provides
/// the methods available for updating ActorVec collections from within event processors.
pub trait ActorVecState<T> {
    /// Adds an item to the end of the collection.
    fn push_cloned(&self, item: T);
    
    /// Removes and returns the last item, or None if empty.
    fn pop_cloned(&self) -> Option<T>;
    
    /// Inserts an item at the specified index.
    fn insert_cloned(&self, index: usize, item: T);
    
    /// Removes the item at the specified index.
    fn remove(&self, index: usize) -> T;
    
    /// Retains only the items that match the predicate.
    fn retain<F>(&self, f: F)
    where
        F: FnMut(&T) -> bool;
        
    /// Removes all items from the collection.
    fn clear(&self);
    
    /// Replaces the entire collection with new items.
    fn replace_cloned(&self, items: Vec<T>);
    
    /// Updates an item at the specified index.
    fn set_cloned(&self, index: usize, item: T);
}

impl<T> ActorVecState<T> for MutableVec<T>
where
    T: Clone,
{
    fn push_cloned(&self, item: T) {
        self.push_cloned(item);
    }
    
    fn pop_cloned(&self) -> Option<T> {
        self.pop_cloned()
    }
    
    fn insert_cloned(&self, index: usize, item: T) {
        self.insert_cloned(index, item);
    }
    
    fn remove(&self, index: usize) -> T {
        self.remove(index)
    }
    
    fn retain<F>(&self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.retain(f);
    }
    
    fn clear(&self) {
        self.clear();
    }
    
    fn replace_cloned(&self, items: Vec<T>) {
        self.replace_cloned(items);
    }
    
    fn set_cloned(&self, index: usize, item: T) {
        self.set_cloned(index, item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive_actors::relay;
    use futures::{select, StreamExt};
    
    #[async_test]
    async fn test_actor_vec_basic_operations() {
        let (add_relay, mut add_stream) = relay();
        let (remove_relay, mut remove_stream) = relay();
        
        let items = ActorVec::new(vec![1, 2, 3], async move |items_vec| {
            loop {
                select! {
                    Some(item) = add_stream.next() => {
                        items_vec.push_cloned(item);
                    }
                    Some(index) = remove_stream.next() => {
                        if index < items_vec.lock_ref().len() {
                            items_vec.remove(index);
                        }
                    }
                }
            }
        });
        
        // Test initial state
        let initial_items = items.to_signal_cloned().to_stream().next().await.unwrap();
        assert_eq!(initial_items, vec![1, 2, 3]);
        
        // Add item
        add_relay.send(4);
        let updated_items = items.to_signal_cloned().to_stream().next().await.unwrap();
        assert_eq!(updated_items, vec![1, 2, 3, 4]);
        
        // Remove item
        remove_relay.send(1); // Remove index 1 (value 2)
        let final_items = items.to_signal_cloned().to_stream().next().await.unwrap();
        assert_eq!(final_items, vec![1, 3, 4]);
    }
    
    #[async_test]
    async fn test_actor_vec_length_signals() {
        let (add_relay, mut add_stream) = relay();
        
        let items = ActorVec::new(vec!["a", "b"], async move |items_vec| {
            while let Some(item) = add_stream.next().await {
                items_vec.push_cloned(item);
            }
        });
        
        // Test length signal
        let initial_len = items.len_signal().to_stream().next().await.unwrap();
        assert_eq!(initial_len, 2);
        
        // Test empty signal
        let is_empty = items.is_empty_signal().to_stream().next().await.unwrap();
        assert_eq!(is_empty, false);
        
        // Add item and check length
        add_relay.send("c");
        let new_len = items.len_signal().to_stream().next().await.unwrap();
        assert_eq!(new_len, 3);
    }
    
    #[async_test]
    async fn test_actor_vec_retain_operation() {
        let (filter_relay, mut filter_stream) = relay();
        
        let numbers = ActorVec::new(vec![1, 2, 3, 4, 5], async move |items_vec| {
            while let Some(min_value) = filter_stream.next().await {
                items_vec.retain(|&n| n >= min_value);
            }
        });
        
        // Filter to keep only >= 3
        filter_relay.send(3);
        let filtered_items = numbers.to_signal_cloned().to_stream().next().await.unwrap();
        assert_eq!(filtered_items, vec![3, 4, 5]);
    }
}