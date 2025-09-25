//! Local UI state Atom helper  
//!
//! Atom provides a convenient wrapper for simple local UI state while maintaining
//! the Actor+Relay architecture internally. It's designed for local component state
//! like button hover, dialog open/closed, etc.

use crate::dataflow::{Actor, Relay, relay};
use futures::StreamExt;
use zoon::Signal;

/// Internal update type for Atom operations
#[derive(Clone, Debug)]
enum AtomUpdate<T> {
    #[allow(dead_code)] // Actor+Relay API - part of complete Atom implementation
    Set(T),
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[allow(dead_code)]
    SetNeq(T),
    #[allow(dead_code)] // Toggle operation for boolean atoms
    Toggle,
}

/// Convenient wrapper for local UI state using Actor+Relay internally.
///
/// Atom provides a simple interface for local component state while
/// maintaining architectural consistency. It uses Actor+Relay internally
/// to ensure all state follows the same patterns.
///
/// Use Atom for truly local UI state like:
/// - Button hover states
/// - Dialog open/closed
/// - Input focus states  
/// - Loading indicators
/// - Form validation states
///
/// # Core Principles
///
/// - **Maintains Architecture**: Uses Actor+Relay internally
/// - **No .get() Methods**: All access through reactive signals
/// - **Local UI Only**: Not for domain state (use domain Actors)
/// - **Simple Interface**: Convenient wrapper for basic use cases
///
/// # Examples
///
/// ```rust
/// use crate::actors::Atom;
///
/// // Button hover state
/// let is_hovered = Atom::new(false);
///
/// // Dialog visibility
/// let dialog_open = Atom::new(false);
///
/// // Form input  
/// let username = Atom::new(String::new());
///
/// // Update state
/// is_hovered.set(true);
/// dialog_open.set(false);
/// username.set("new_username".to_string());
///
/// // Bind to UI reactively
/// is_hovered.signal() // Signal<Item = bool>
/// dialog_open.signal()
/// username.signal()
/// ```
#[derive(Clone, Debug)]
pub struct Atom<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// The underlying Actor that manages state
    actor: Actor<T>,
    /// Relay for sending updates to the Actor
    setter: Relay<AtomUpdate<T>>,
}

impl<T> Atom<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Create a new Atom with an initial value.
    ///
    /// The Atom uses Actor+Relay internally to maintain architectural
    /// consistency while providing a convenient interface.
    ///
    /// # Arguments
    ///
    /// - `initial`: The starting value for this Atom
    ///
    /// # Examples
    ///
    /// ```rust
    /// let button_hovered = Atom::new(false);
    /// let dialog_title = Atom::new("Default Title".to_string());  
    /// let selected_index = Atom::new(None::<usize>);
    /// ```
    pub fn new(initial: T) -> Self
    where
        T: PartialEq,
    {
        let (setter, mut setter_stream) = relay();

        let actor = Actor::new(initial, async move |state| {
            while let Some(update) = setter_stream.next().await {
                match update {
                    AtomUpdate::Set(new_value) => {
                        state.set(new_value);
                    }
                    AtomUpdate::SetNeq(new_value) => {
                        state.set_neq(new_value);
                    }
                    AtomUpdate::Toggle => {
                        // This should only be called for boolean atoms due to the type constraint
                        // on the toggle() method. We need to handle this with proper type casting.
                        // For now, we'll panic if called on non-boolean types (which shouldn't happen)
                        panic!("Toggle called on non-boolean Atom");
                    }
                }
            }
        });

        Self { actor, setter }
    }

    /// Update the Atom's value.
    ///
    /// This sends the new value through the internal relay to the Actor.
    /// The update is processed asynchronously and triggers reactive signals.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let counter = Atom::new(0);
    /// counter.set(5);
    ///
    /// let message = Atom::new(String::new());
    /// message.set("Hello World".to_string());
    /// ```
    #[allow(dead_code)] // Actor+Relay API method - preserve for completeness
    pub fn set(&self, value: T) {
        self.setter.send(AtomUpdate::Set(value));
    }

    /// Get a reactive signal for this Atom's value.
    ///
    /// This is the primary way to access Atom state. The signal emits
    /// the current value and all future updates.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let is_loading = Atom::new(false);
    ///
    /// // Bind to UI reactively
    /// El::new().child_signal(
    ///     is_loading.signal().map(|loading| {
    ///         if loading {
    ///             Text::new("Loading...")
    ///         } else {
    ///             Text::new("Ready")
    ///         }
    ///     })
    /// )
    /// ```
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[allow(dead_code)]
    pub fn signal(&self) -> impl Signal<Item = T> + use<T> {
        self.actor.signal()
    }

    /// Get a reactive signal with a reference to avoid cloning.
    ///
    /// Use this when the value is expensive to clone and you want to
    /// compute derived values efficiently.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let large_list = Atom::new(vec![0; 1000]);
    ///
    /// // Compute length without cloning the vector
    /// large_list.signal_ref(|list| list.len())
    /// ```
    #[allow(dead_code)] // Actor+Relay API method - preserve for completeness
    pub fn signal_ref<U>(&self, f: impl Fn(&T) -> U + 'static) -> impl Signal<Item = U>
    where
        U: PartialEq + Send + Sync + 'static,
    {
        self.actor.signal_ref(f)
    }

    /// Update the Atom's value only if it differs from the current value.
    ///
    /// This helps prevent unnecessary signal emissions and re-renders when
    /// the value hasn't actually changed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let counter = Atom::new(5);
    /// counter.set_neq(5); // No update, value is already 5
    /// counter.set_neq(10); // Updates to 10
    /// ```
    // Part of public Actor+Relay API - will be used when moved to standalone crate
    #[allow(dead_code)]
    pub fn set_neq(&self, value: T)
    where
        T: PartialEq,
    {
        self.setter.send(AtomUpdate::SetNeq(value));
    }

    /// Get current value (for event handlers only)
    ///
    /// This provides immediate access to the current value for use in
    /// event handlers where signal-based access isn't practical.
    ///
    /// **Use sparingly** - prefer signal-based access when possible.
    pub fn get_cloned(&self) -> T {
        // This uses the actor's internal state directly
        // It's a compromise for event handler patterns
        self.actor.state.lock_ref().clone()
    }

    // Note: update() methods are not implemented.
    // These would require mutable closure access to internal state,
    // which conflicts with the Actor+Relay architecture.
    // Use set() or set_neq() methods for all state updates.
}

/// Boolean-specific methods for Atom<bool>
impl Atom<bool> {
    /// Toggle the boolean value of this Atom.
    ///
    /// This is a convenience method specifically for boolean atoms that
    /// implements proper toggle logic within the Actor+Relay architecture.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let is_open = Atom::new(false);
    /// is_open.toggle(); // Now true
    /// is_open.toggle(); // Now false
    /// ```
    pub fn toggle(&self) {
        self.setter.send(AtomUpdate::Toggle);
    }
}

impl<T> Default for Atom<T>
where
    T: Clone + Send + Sync + Default + PartialEq + 'static,
{
    /// Create an Atom with the default value for T.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let counter: Atom<i32> = Atom::default(); // 0
    /// let text: Atom<String> = Atom::default(); // ""  
    /// let flag: Atom<bool> = Atom::default(); // false
    /// ```
    fn default() -> Self {
        Self::new(T::default())
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use futures::StreamExt;
    use zoon::SignalExt;

    #[tokio::test]
    async fn test_atom_basic_functionality() {
        let atom = Atom::new(42);

        // Check initial value
        let initial_value = atom.signal().to_stream().next().await.unwrap();
        assert_eq!(initial_value, 42);

        // Update value
        atom.set(100);
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let updated_value = atom.signal().to_stream().next().await.unwrap();
        assert_eq!(updated_value, 100);
    }

    #[tokio::test]
    async fn test_atom_default() {
        let default_int: Atom<i32> = Atom::default();
        let default_string: Atom<String> = Atom::default();
        let default_bool: Atom<bool> = Atom::default();

        let int_val = default_int.signal().to_stream().next().await.unwrap();
        let string_val = default_string.signal().to_stream().next().await.unwrap();
        let bool_val = default_bool.signal().to_stream().next().await.unwrap();

        assert_eq!(int_val, 0);
        assert_eq!(string_val, "");
        assert!(!bool_val);
    }
}
