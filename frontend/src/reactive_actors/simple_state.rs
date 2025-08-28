//! SimpleState<T> - Local UI State Management
//!
//! Unified helper for local UI state using Actor+Relay architecture internally.
//! Provides convenient API for simple use cases while maintaining architectural consistency.

use crate::reactive_actors::{Actor, Relay, relay};
use zoon::*;

/// Unified helper for local UI state - uses Actor+Relay architecture internally.
/// 
/// SimpleState provides a convenient API for truly local UI state (button hover,
/// dropdown open/closed, input focus, etc.) while maintaining architectural 
/// principles internally through Actor+Relay.
/// 
/// ## Usage Pattern
/// 
/// ```rust
/// use crate::reactive_actors::SimpleState;
/// 
/// // Dialog component state
/// struct FileDialogState {
///     is_open: SimpleState<bool>,
///     filter_text: SimpleState<String>,
///     selected_files: SimpleState<Vec<PathBuf>>,
///     current_directory: SimpleState<PathBuf>,
///     error_message: SimpleState<Option<String>>,
/// }
/// 
/// impl Default for FileDialogState {
///     fn default() -> Self {
///         Self {
///             is_open: SimpleState::new(false),
///             filter_text: SimpleState::new(String::new()),
///             selected_files: SimpleState::new(vec![]),
///             current_directory: SimpleState::new(std::env::current_dir().unwrap()),
///             error_message: SimpleState::new(None),
///         }
///     }
/// }
/// 
/// // Usage in UI
/// fn file_dialog(state: &FileDialogState) -> impl Element {
///     El::new()
///         .child_signal(
///             state.is_open.signal().map(|is_open| {
///                 if is_open {
///                     dialog_content(state).into_element()
///                 } else {
///                     El::new().into_element()
///                 }
///             })
///         )
/// }
/// 
/// // Event handlers
/// button()
///     .on_press({
///         let is_open = state.is_open.clone();
///         move || is_open.set(true)
///     })
/// ```
/// 
/// ## Domain Examples
/// 
/// ```rust
/// // Panel component state
/// struct PanelState {
///     width: SimpleState<f32>,
///     height: SimpleState<f32>,
///     is_collapsed: SimpleState<bool>,
///     is_hovered: SimpleState<bool>,
///     resize_dragging: SimpleState<bool>,
/// }
/// 
/// // Search component state
/// struct SearchState {
///     filter_text: SimpleState<String>,
///     is_focused: SimpleState<bool>,
///     match_count: SimpleState<usize>,
///     selected_index: SimpleState<Option<usize>>,
/// }
/// 
/// // TreeView component state
/// struct TreeViewState {
///     expanded_nodes: SimpleState<HashSet<String>>,
///     selected_node: SimpleState<Option<String>>,
///     hover_node: SimpleState<Option<String>>,
///     is_dragging: SimpleState<bool>,
/// }
/// ```
/// 
/// ## Why SimpleState is Acceptable
/// 
/// - **Uses Actor+Relay internally** - maintains architectural principles
/// - **Provides convenient API** for simple use cases  
/// - **Still prevents race conditions** (no `.get()` method)
/// - **Maintains traceability** through Actor infrastructure
/// - **Can be tested** like any other Actor
/// - **Local scope only** - for truly local UI state, not shared domain state
#[derive(Clone, Debug)]
pub struct SimpleState<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Internal Actor that manages the state value
    pub value: Actor<T>,
    /// Internal Relay for setting new values  
    pub value_changed_relay: Relay<T>,
}

impl<T> SimpleState<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Creates a new SimpleState with an initial value.
    /// 
    /// Internally creates an Actor+Relay pair for state management.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// // UI component local state
    /// let dialog_open = SimpleState::new(false);
    /// let filter_text = SimpleState::new(String::new());
    /// let selected_index = SimpleState::new(None::<usize>);
    /// ```
    pub fn new(initial: T) -> Self {
        let (value_changed_relay, mut value_stream) = relay();
        
        let value = Actor::new(initial, async move |state| {
            while let Some(new_value) = value_stream.next().await {
                state.set_neq(new_value);
            }
        });
        
        SimpleState { value, value_changed_relay }
    }
    
    /// Sets a new value for this SimpleState.
    /// 
    /// The value is updated through the internal Actor+Relay system,
    /// ensuring consistency and preventing race conditions.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// // Update state from event handlers
    /// dialog_state.is_open.set(true);
    /// search_state.filter_text.set("waveform".to_string());
    /// panel_state.width.set(300.0);
    /// ```
    pub fn set(&self, value: T) {
        self.value_changed_relay.send(value);
    }
    
    /// Returns a reactive signal for this SimpleState's current value.
    /// 
    /// Use this for reactive UI binding and computed values.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// // Bind to UI elements
    /// Text::new_signal(
    ///     search_state.filter_text.signal()
    ///         .map(|text| format!("Searching: {}", text))
    /// )
    /// 
    /// // Conditional rendering
    /// El::new()
    ///     .child_signal(
    ///         dialog_state.is_open.signal().map(|is_open| {
    ///             if is_open {
    ///                 dialog_content().into_element()
    ///             } else {
    ///                 El::new().into_element()
    ///             }
    ///         })
    ///     )
    /// 
    /// // Combine with other signals
    /// map_ref! {
    ///     let filter = search_state.filter_text.signal(),
    ///     let items = items_list.signal() => {
    ///         items.into_iter()
    ///             .filter(|item| item.name.contains(&*filter))
    ///             .collect::<Vec<_>>()
    ///     }
    /// }
    /// ```
    pub fn signal(&self) -> impl Signal<Item = T> {
        self.value.signal()
    }
    
    /// Returns a signal reference for efficient access without cloning.
    /// 
    /// Use this when you need to access the value without cloning it,
    /// particularly useful for large data structures.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// // Get string length without cloning the string
    /// let text_length = search_state.filter_text.signal_ref(|text| text.len());
    /// 
    /// // Check collection properties without cloning
    /// let has_items = selected_files.signal_ref(|files| !files.is_empty());
    /// ```
    pub fn signal_ref<U, F>(&self, f: F) -> impl Signal<Item = U>
    where
        F: Fn(&T) -> U + Send + Sync + 'static,
        U: Send + Sync + 'static,
    {
        self.value.signal_ref(f)
    }
    
    /// Updates the value using a function.
    /// 
    /// Convenient for making modifications based on the current value.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// // Toggle boolean state
    /// dialog_state.is_open.update(|current| !current);
    /// 
    /// // Modify collection
    /// selected_files.update(|mut files| {
    ///     files.push(new_file);
    ///     files
    /// });
    /// 
    /// // Update numeric value
    /// counter.update(|current| current + 1);
    /// ```
    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(T) -> T + Send + 'static,
    {
        // Get current value and compute new value
        // This is safe because we use the Actor+Relay pattern internally
        let current_signal = self.value.signal();
        let value_changed_relay = self.value_changed_relay.clone();
        
        Task::start(async move {
            if let Some(current) = current_signal.to_stream().next().await {
                let new_value = f(current);
                value_changed_relay.send(new_value);
            }
        });
    }
}

impl<T> Default for SimpleState<T>
where
    T: Default + Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

// Note: SimpleState<T> intentionally does NOT provide a .get() method
// All state access must be through signals to prevent race conditions
// and maintain architectural consistency

/// Convenience functions for common SimpleState use cases
impl SimpleState<bool> {
    /// Toggles a boolean SimpleState value.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// // Toggle dialog visibility
    /// dialog_state.is_open.toggle();
    /// 
    /// // Toggle panel collapse state
    /// panel_state.is_collapsed.toggle();
    /// ```
    pub fn toggle(&self) {
        self.update(|current| !current);
    }
}

impl<T> SimpleState<Vec<T>>
where
    T: Clone + Send + Sync + 'static,
{
    /// Pushes an item to a Vec SimpleState.
    pub fn push(&self, item: T) {
        self.update(|mut vec| {
            vec.push(item);
            vec
        });
    }
    
    /// Removes the last item from a Vec SimpleState.
    pub fn pop(&self) {
        self.update(|mut vec| {
            vec.pop();
            vec
        });
    }
    
    /// Clears all items from a Vec SimpleState.
    pub fn clear(&self) {
        self.update(|_| Vec::new());
    }
}

impl SimpleState<String> {
    /// Appends text to a String SimpleState.
    pub fn append(&self, text: &str) {
        let text = text.to_string();
        self.update(|mut current| {
            current.push_str(&text);
            current
        });
    }
    
    /// Clears the String SimpleState.
    pub fn clear(&self) {
        self.set(String::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    
    #[async_test]
    async fn test_simple_state_basic_functionality() {
        let state = SimpleState::new(42);
        
        // Test initial value
        let initial = state.signal().to_stream().next().await.unwrap();
        assert_eq!(initial, 42);
        
        // Test setting new value
        state.set(100);
        let updated = state.signal().to_stream().next().await.unwrap();
        assert_eq!(updated, 100);
    }
    
    #[async_test] 
    async fn test_simple_state_update_function() {
        let counter = SimpleState::new(0);
        
        // Test update function
        counter.update(|current| current + 5);
        let result = counter.signal().to_stream().next().await.unwrap();
        assert_eq!(result, 5);
        
        // Test multiple updates
        counter.update(|current| current * 2);
        let final_result = counter.signal().to_stream().next().await.unwrap();
        assert_eq!(final_result, 10);
    }
    
    #[async_test]
    async fn test_simple_state_boolean_toggle() {
        let flag = SimpleState::new(false);
        
        // Test initial state
        let initial = flag.signal().to_stream().next().await.unwrap();
        assert_eq!(initial, false);
        
        // Test toggle
        flag.toggle();
        let toggled = flag.signal().to_stream().next().await.unwrap();
        assert_eq!(toggled, true);
        
        // Test toggle again
        flag.toggle();
        let toggled_again = flag.signal().to_stream().next().await.unwrap();
        assert_eq!(toggled_again, false);
    }
    
    #[async_test]
    async fn test_simple_state_vec_operations() {
        let items = SimpleState::new(vec![1, 2, 3]);
        
        // Test push
        items.push(4);
        let after_push = items.signal().to_stream().next().await.unwrap();
        assert_eq!(after_push, vec![1, 2, 3, 4]);
        
        // Test pop
        items.pop();
        let after_pop = items.signal().to_stream().next().await.unwrap();
        assert_eq!(after_pop, vec![1, 2, 3]);
        
        // Test clear
        items.clear();
        let after_clear = items.signal().to_stream().next().await.unwrap();
        assert_eq!(after_clear, Vec::<i32>::new());
    }
    
    #[async_test]
    async fn test_simple_state_string_operations() {
        let text = SimpleState::new("Hello".to_string());
        
        // Test append
        text.append(" World");
        let after_append = text.signal().to_stream().next().await.unwrap();
        assert_eq!(after_append, "Hello World");
        
        // Test clear
        text.clear();
        let after_clear = text.signal().to_stream().next().await.unwrap();
        assert_eq!(after_clear, String::new());
    }
    
    #[async_test]
    async fn test_simple_state_signal_ref() {
        let text = SimpleState::new("Hello World".to_string());
        
        // Test signal_ref for length
        let length_signal = text.signal_ref(|s| s.len());
        let length = length_signal.to_stream().next().await.unwrap();
        assert_eq!(length, 11);
        
        // Update text and verify length changes
        text.set("Hi".to_string());
        let new_length = length_signal.to_stream().next().await.unwrap();
        assert_eq!(new_length, 2);
    }
}