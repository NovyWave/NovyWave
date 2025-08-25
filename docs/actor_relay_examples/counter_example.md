# Actor+Relay Example: Counter

This example shows how to transform a simple counter from traditional MoonZoon patterns to the Actor+Relay architecture.

## Original MoonZone Counter

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
        .on_hovered_change(move |is_hovered| hovered.set(is_hovered))
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

## Actor+Relay Version

```rust
use zoon::*;

// Event types - clear, typed interactions
#[derive(Clone)]
struct IncrementBy(i32);

#[derive(Clone)] 
struct DecrementBy(i32);

#[derive(Clone)]
struct Reset;

/// Counter with proper encapsulation and event-driven updates
#[derive(Clone, Debug)]
struct CounterState {
    // State managed by Actor - controlled access only
    value: Actor<i32>,
    
    // Event Relays - clear interaction points
    increment: Relay<IncrementBy>,
    decrement: Relay<DecrementBy>, 
    reset: Relay<Reset>,
}

impl CounterState {
    pub fn new(initial_value: i32) -> Self {
        // Create Relays FIRST
        let increment = Relay::new();
        let decrement = Relay::new(); 
        let reset = Relay::new();
        
        // Create Actor that responds to Relay events
        let value = Actor::new(initial_value, clone!((increment, decrement, reset) async move |state| {
            // Handle all counter events in one place
            Task::start_droppable(clone!((state, increment, decrement, reset) async move {
                // Increment events
                increment.subscribe().for_each(clone!((state) async move |IncrementBy(amount)| {
                    let current = state.get();
                    state.set(current + amount);
                })).await;
            }));
            
            Task::start_droppable(clone!((state, decrement) async move {
                // Decrement events  
                decrement.subscribe().for_each(clone!((state) async move |DecrementBy(amount)| {
                    let current = state.get();
                    state.set(current - amount);
                })).await;
            }));
            
            Task::start_droppable(clone!((state, reset) async move {
                // Reset events
                reset.subscribe().for_each(clone!((state) async move |Reset| {
                    state.set(0);
                })).await;
            }));
        }));
        
        CounterState {
            value,
            increment,
            decrement,
            reset,
        }
    }
    
    // Public API - only through events
    pub fn increment_by(&self, amount: i32) -> Result<(), RelayError> {
        self.increment.send(IncrementBy(amount))
    }
    
    pub fn decrement_by(&self, amount: i32) -> Result<(), RelayError> {
        self.decrement.send(DecrementBy(amount))
    }
    
    pub fn reset(&self) -> Result<(), RelayError> {
        self.reset.send(Reset)
    }
    
    // Access to reactive state
    pub fn value_signal(&self) -> impl Signal<Item = i32> {
        self.value.signal()
    }
    
    pub fn get_value(&self) -> i32 {
        self.value.get()
    }
}

// Global instance - but now properly encapsulated
static COUNTER: Lazy<CounterState> = Lazy::new(|| CounterState::new(0));

fn main() {
    start_app("app", root);
}

fn root() -> impl Element {
    Row::new()
        .s(Align::center())
        .s(Gap::new().x(15))
        .item(counter_button("-", -1))
        .item_signal(COUNTER.value_signal())  // Clean reactive access
        .item(counter_button("+", 1))
        .item(reset_button())  // New functionality - easy to add!
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
        .on_hovered_change(move |is_hovered| hovered.set(is_hovered))
        .label(label)
        .on_press(move || {
            // Event-driven interaction - clear and traceable
            if step > 0 {
                COUNTER.increment_by(step);
            } else {
                COUNTER.decrement_by(-step);
            }
        })
}

fn reset_button() -> impl Element {
    Button::new()
        .s(Width::exact(60))
        .s(RoundedCorners::all_max())
        .s(Background::new().color(color!("#ffcccc")))
        .label("Reset")
        .on_press(|| COUNTER.reset())  // New feature - easy to add!
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[async_test]
    async fn test_counter_increment() {
        let counter = CounterState::new(5);
        
        counter.increment_by(3).unwrap();
        // Wait for actor to process event
        Timer::sleep(10).await;
        
        assert_eq!(counter.get_value(), 8);
    }
    
    #[async_test] 
    async fn test_counter_reset() {
        let counter = CounterState::new(42);
        
        counter.reset().unwrap();
        Timer::sleep(10).await;
        
        assert_eq!(counter.get_value(), 0);
    }
}
```

## Key Benefits of Actor+Relay Version

### 1. **🔒 Encapsulation**
- State is owned by the Actor - only controlled mutations allowed
- Public API clearly defines what operations are possible
- No accidental direct state mutations

### 2. **📡 Event Traceability** 
- All changes go through typed events: `IncrementBy`, `DecrementBy`, `Reset`
- Easy to log, debug, and trace what happened
- Clear audit trail of state changes

### 3. **🧪 Testability**
- Counter can be instantiated and tested in isolation
- Events can be sent programmatically for testing
- No global state pollution between tests

### 4. **⚡ Extensibility**
- Adding new features (like Reset button) is trivial
- Just add new Relay and handler in Actor
- No risk of breaking existing functionality

### 5. **🎯 Type Safety**
- Events are strongly typed - compile-time guarantees
- No magic numbers or strings
- IDE autocomplete and refactoring support

### 6. **🔄 Reactive Patterns**
- UI reactively updates when state changes
- Clear separation between events and state
- Easy to add multiple views of the same state

## Advanced Features Made Possible

```rust
// Easy to add features like:

// 1. Counter history/undo
struct CounterWithHistory {
    counter: CounterState,
    history: ActorVec<i32>,
}

// 2. Multiple counters
struct CounterCollection {
    counters: ActorVec<CounterState>,
    add_counter: Relay,
}

// 3. Persistence
impl CounterState {
    pub fn save_to_storage(&self) {
        // Save current value when it changes
        // Easy to add since we have event stream
    }
}
```

This simple transformation shows how Actor+Relay patterns provide better architecture, testing, and maintainability while keeping the same UI and user experience.