# Actor+Relay Example: Counter (Global State Patterns)

> **⚠️ BRIDGE DOCUMENTATION**: This file contains global state patterns for Actor+Relay architecture. These patterns serve as a bridge between traditional MoonZoon globals and idiomatic local state. For production applications, prefer the local state patterns in `counter_example.md`.

This example shows how to transform a simple counter from traditional MoonZoon patterns to global Actor+Relay architecture. While functional, this approach is less idiomatic than local state patterns but may serve as a stepping stone during migration.

## Original MoonZoon Counter Problems Reference

For reference on why the original MoonZoon approach with global Mutables was problematic, see the "Original MoonZoon Counter" section in `counter_example.md`.

## Global Actor+Relay Version

```rust
use zoon::*;

/// Counter with proper encapsulation and event-driven updates
#[derive(Clone)]
struct Counter {
    // State managed by Actor - controlled access only
    pub value: Actor<i32>,
    
    // Single relay for all changes - matches original pattern
    pub change_by: Relay<i32>,
}

impl Default for Counter {
    fn default() -> Self {
        let (change_by, mut change_stream) = relay();
        
        // Simple Actor that responds to change events
        let value = Actor::new(0, async move |state| {
            while let Some(amount) = change_stream.next().await {
                state.update(|value| value + amount);
            }
        });
        
        Counter { value, change_by }
    }
}

// Global instance - but now properly encapsulated
static COUNTER: Lazy<Counter> = lazy::default();

fn main() {
    start_app("app", root);
}

fn root() -> impl Element {
    Row::new()
        .s(Align::center())
        .s(Gap::new().x(15))
        .item(counter_button("-", -1))
        .item_signal(COUNTER.value.signal())  // Clean reactive access
        .item(counter_button("+", 1))
}

fn counter_button(label: &str, step: i32) -> impl Element {
    let hovered = SimpleState::new(false);
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
        .on_press(move || {
            // Direct global access - atomic operation
            COUNTER.change_by.send(step);
        })
}
```

## Global Benefits

### ✅ When Global State Is Appropriate:
- Single counter shared across entire app
- Simple applications where globals are natural
- When you want minimal ceremony
- Migration from existing global Mutable patterns

### ✅ Benefits Over Raw Global Mutables:
- **Direct access**: `COUNTER.change_by.send()` - no parameter passing
- **Minimal code**: Fewest lines of code for simple use cases
- **Atomic operations**: No get/send race conditions
- **Encapsulated mutations**: Changes only through defined relays
- **Event traceability**: All state changes go through typed events

## Key Race-Condition Prevention

### ❌ Old Global Mutable Pattern (Problematic):
```rust
// Race condition possible between get() and set()
static COUNTER: Lazy<Mutable<i32>> = lazy::default();
fn increment() {
    let current = COUNTER.get();  // Read
    COUNTER.set(current + 1);     // Write - another thread might have changed value!
}
```

### ✅ Global Actor+Relay Pattern (Safe):
```rust
// Atomic operation - no race conditions possible
static COUNTER: Lazy<Counter> = lazy::default();
fn increment() {
    COUNTER.change_by.send(1);  // Single atomic operation
}
```

## Global SimpleState Helper

For simple state that doesn't need complex event types, we can create a global helper pattern:

```rust
/// Generic helper for simple Actor+Relay global state
struct SimpleState<T> {
    pub value: Actor<T>,
    pub setter: Relay<T>,
}

impl<T: Clone> SimpleState<T> {
    pub fn new(initial: T) -> Self {
        let (setter, mut setter_stream) = relay();
        
        let value = Actor::new(initial, async move |state| {
            while let Some(new_value) = setter_stream.next().await {
                state.set_neq(new_value);
            }
        });
        
        SimpleState { value, setter }
    }
}

// Usage in global context:
static HOVER_STATE: Lazy<SimpleState<bool>> = Lazy::new(|| SimpleState::new(false));
```

## Advanced Global Features

```rust
// Easy to add features like:

// 1. Counter history/undo (global)
static COUNTER_HISTORY: Lazy<ActorVec<i32>> = lazy::default();

// 2. Multiple global counters
static COUNTER_COLLECTION: Lazy<ActorVec<Counter>> = lazy::default();

// 3. Global persistence
impl Counter {
    pub fn save_to_storage(&self) {
        // Save current value when it changes - easy with global access
    }
}
```

## Testing Global Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[async_test]
    async fn test_global_counter_increment() {
        // Reset global state for test
        COUNTER.change_by.send(-COUNTER.value.get()); // Reset to 0
        
        COUNTER.change_by.send(3);
        
        let final_value = COUNTER.value.signal().to_stream().next().await.unwrap();
        assert_eq!(final_value, 3);
    }
    
    #[async_test] 
    async fn test_global_counter_decrement() {
        // Reset global state for test
        COUNTER.change_by.send(-COUNTER.value.get()); // Reset to 0
        
        COUNTER.change_by.send(-2);
        
        let final_value = COUNTER.value.signal().to_stream().next().await.unwrap();
        assert_eq!(final_value, -2);
    }
    
    #[async_test]
    async fn test_global_simple_state_helper() {
        let hover_state = SimpleState::new(false);
        
        // Test basic setter
        hover_state.setter.send(true);
        
        let final_value = hover_state.value.signal().to_stream().next().await.unwrap();
        assert_eq!(final_value, true);
    }
}
```

## Migration Notes

This global pattern serves as a bridge between traditional MoonZoon globals and idiomatic Actor+Relay local state. 

### When to Use Global Patterns:
- **Legacy migration**: Existing apps with extensive global state dependencies
- **Singleton services**: Configuration, logging, connection management
- **Cross-cutting concerns**: State that genuinely needs to be accessed from many unrelated components
- **Simple applications**: Where the overhead of parameter passing isn't justified

### When to Prefer Local Patterns:
- **New applications**: Start with local state unless you have specific global requirements
- **Reusable components**: Local state makes components more composable
- **Testing**: Local state is significantly easier to test in isolation
- **Team development**: Local state reduces hidden dependencies between components

The key insight: **Global Actor+Relay eliminates the race conditions and uncontrolled mutations of global Mutables while preserving the convenience of global access when it's genuinely needed.**