# Actor+Relay Example: Counter

This example shows how to transform a simple counter from traditional MoonZoon patterns to the Actor+Relay architecture.

> **📄 Related Documentation**: For global state patterns using Actor+Relay, see [`counter_example_global.md`](counter_example_global.md). This file focuses on the recommended local state approach.

## Original MoonZoon Counter

```rust
use zoon::*;

fn main() {
    start_app("app", root);
}

// Global mutable state - hard to track, test, and reason about
static COUNTER: Lazy<Mutable<i32>> = lazy::default();

fn root() -> impl Element {
    Row::new()
        .s(Align::center())
        .s(Gap::new().x(15))
        .item(counter_button("-", -1))
        .item_signal(COUNTER.signal())         // Direct global access
        .item(counter_button("+", 1))
}

fn counter_button(label: &str, step: i32) -> impl Element {
    let (hovered, hovered_signal) = Mutable::new_and_signal(false);
    Button::new()
        .s(Width::exact(45))
        .s(RoundedCorners::all_max())
        .s(Background::new()
            .color_signal(hovered_signal.map_bool(|| color!("#edc8f5"), || color!("#E1A3EE", 0.8))))
        .s(Borders::all(
            Border::new()
                .width(2)
                .color(color!("oklch(0.6 0.182 350.53 / .7")),
        ))
        .on_hovered_change(move |is_hovered| hovered.setter.send(is_hovered))
        .label(label)
        .on_press(move || *COUNTER.lock_mut() += step)  // Direct mutation
}
```

### Problems with Original Approach:
- **Global state pollution**: Hard to track mutations
- **Tight coupling**: UI directly mutates state
- **No event traceability**: Can't debug what changed the counter
- **Testing difficulty**: Global state makes unit testing hard
- **No encapsulation**: Any code can access COUNTER

## Actor+Relay Version (Local State)

```rust
use zoon::*;

/// Flattened app struct with counter state directly embedded
#[derive(Clone)]
struct CounterApp {
    // State directly in app struct - no unnecessary wrapper
    value: Actor<i32>,
    change_by: Relay<i32>,
}

impl Default for CounterApp {
    fn default() -> Self {
        let (change_by, mut change_stream) = relay();
        
        // Simple Actor that responds to change events
        let value = Actor::new(0, async move |state| {
            while let Some(amount) = change_stream.next().await {
                state.update(|value| value + amount);
            }
        });
        
        CounterApp { value, change_by }
    }
}

impl CounterApp {
    fn root(&self) -> impl Element {
        Row::new()
            .s(Align::center())
            .s(Gap::new().x(15))
            .item(self.counter_button("-", -1))
            .item_signal(self.value.signal())
            .item(self.counter_button("+", 1))
    }
    
    fn counter_button(&self, label: &str, step: i32) -> impl Element {
        let hovered = Atom::new(false);
        
        Button::new()
            .s(Width::exact(45))
            .s(RoundedCorners::all_max())
            .s(Background::new()
                .color_signal(hovered.value.signal().map_bool(|| color!("#edc8f5"), || color!("#E1A3EE", 0.8))))
            .s(Borders::all(
                Border::new()
                    .width(2)
                    .color(color!("oklch(0.6 0.182 350.53 / .7")),
            ))
            .on_hovered_change(move |is_hovered| hovered.setter.send(is_hovered))
            .label(label)
            .on_press({
                let change_by = self.change_by.clone();
                move || {
                    change_by.send(step);
                }
            })
    }
}

fn main() {
    start_app("app", || CounterApp::default().root());
}

```

## Key Benefits of Actor+Relay Counter

### 1. **🔒 Race-Condition Prevention**
- Atomic operations: `increment.send()` and `decrement.send()` 
- No get/send race conditions possible
- State mutations are controlled and sequential

### 2. **📡 Event Traceability** 
- All changes go through explicit relay events
- Easy to log, debug, and trace what happened
- Clear audit trail of state changes

### 3. **🧪 Easy Testing**
- Events can be sent programmatically for testing
- Direct state access for assertions
- Local approach especially clean for unit tests

### 4. **⚡ Simple & Atomic**
- Single event per operation - no complex enums
- Pure type usage - no conversions between i32/usize
- Atomic operations prevent concurrency bugs

### 5. **🔄 Reactive Integration**
- UI reactively updates when state changes
- Clean signal access: `counter.value.signal()`
- Easy to add multiple views of the same state

## Helper Patterns for Simple State

For simple state that doesn't need complex event types, we can create a helper pattern:

```rust
/// Generic helper for simple Actor+Relay state
struct Atom<T> {
    pub value: Actor<T>,
    pub setter: Relay<T>,
}

impl<T: Clone> Atom<T> {
    pub fn new(initial: T) -> Self {
        // Create Relay with pre-subscribed stream - eliminates clone! entirely
        let (setter, mut setter_stream) = relay();
        
        let value = Actor::new(initial, async move |state| {
            // Clean imperative style - stream moved directly into Actor
            while let Some(new_value) = setter_stream.next().await {
                // Use set_neq for efficiency - only updates if value actually changes
                state.set_neq(new_value);
            }
        });
        
        Atom { value, setter }
    }
}

// Note: We don't provide convenience methods like toggle() because they would
// require separate get() + send() calls, creating potential race conditions.
// For atomic operations, use the full Actor pattern with proper update semantics.
// Simple usage: state.setter.send(new_value)
```

This imperative `while` loop pattern is more idiomatic Rust and makes it easier to handle multiple streams without excessive cloning.

## Advanced Features Made Possible

```rust
// Easy to add features like:

// 1. Counter history/undo
struct CounterWithHistory {
    counter: Counter,
    history: ActorVec<i32>,
}

// 2. Multiple counters
struct CounterCollection {
    counters: ActorVec<Counter>,
    add_counter: Relay,
}

// 3. Persistence
impl Counter {
    pub fn save_to_storage(&self) {
        // Save current value when it changes
        // Easy to add since we have event stream
    }
}
```

## Testing

The Actor+Relay pattern makes testing straightforward since counters can be instantiated and tested in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[async_test]
    async fn test_counter_increment() {
        let counter = Counter::default();
        
        counter.change_by.send(3);
        
        let final_value = counter.value.signal().to_stream().next().await.unwrap();
        assert_eq!(final_value, 3);
    }
    
    #[async_test] 
    async fn test_counter_decrement() {
        let counter = Counter::default();
        
        counter.change_by.send(-2);
        
        let final_value = counter.value.signal().to_stream().next().await.unwrap();
        assert_eq!(final_value, -2);
    }
    
    #[async_test]
    async fn test_atom_helper() {
        let hover_state = Atom::new(false);
        
        // Test basic setter
        hover_state.setter.send(true);
        
        let final_value = hover_state.value.signal().to_stream().next().await.unwrap();
        assert_eq!(final_value, true);
        
        // Test changing value again
        hover_state.setter.send(false);
        
        let final_value = hover_state.value.signal().to_stream().next().await.unwrap();
        assert_eq!(final_value, false);
    }
}
```

This simple transformation shows how Actor+Relay patterns provide better architecture, testing, and maintainability while keeping the same UI and user experience.