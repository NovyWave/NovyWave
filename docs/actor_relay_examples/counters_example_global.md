# Actor+Relay Example: Counters Dynamic Collection (Global State Patterns)

> **⚠️ BRIDGE DOCUMENTATION**: This file contains global state patterns for Actor+Relay architecture. These patterns serve as a bridge between traditional MoonZoon globals and idiomatic local state. For production applications, prefer the local state patterns in `counters_example.md`.

This example shows how to transform a complex multi-counter application with dynamic grid sizing from traditional MoonZoon patterns to global Actor+Relay architecture using clean patterns. While functional, this approach is less idiomatic than local state patterns but may serve as a stepping stone during migration.

## Original MoonZone Counters Problems Reference

For reference on why the original MoonZone approach with global Store was problematic, see the "Original MoonZone Counters" section in `counters_example.md`.

## Global Actor+Relay Version (Clean Globals)

```rust
use zoon::*;

// Helper function to convert row/col to index
fn grid_index(row: usize, col: usize, columns: usize) -> usize {
    row * columns + col
}

// Unified control - no duplication needed
#[derive(Clone)]
struct GridDimensionControl {
    pub count: Actor<usize>,
    pub increment: Relay, // Simple atomic increment
    pub decrement: Relay, // Simple atomic decrement
}

impl Default for GridDimensionControl {
    fn default() -> Self {
        let (increment, mut increment_stream) = relay();
        let (decrement, mut decrement_stream) = relay();
        let count = Actor::new(5, async move |state| {
            loop {
                select! {
                    Some(()) = increment_stream.next() => {
                        state.update(|current| current + 1);
                    }
                    Some(()) = decrement_stream.next() => {
                        state.update(|current| current.saturating_sub(1).max(1));
                    }
                }
            }
        });
        GridDimensionControl { count, increment, decrement }
    }
}

#[derive(Clone)]  
struct CounterGrid {
    pub values: ActorVec<i32>,
    pub change: Relay<(usize, i32)>,
}

impl Default for CounterGrid {
    fn default() -> Self {
        let (change, mut change_stream) = relay();
        let values = ActorVec::new(vec![0; 25], async move |counters_vec| {
            join!(
                // Handle counter changes
                async {
                    while let Some((index, change_amount)) = change_stream.next().await {
                        let mut counters = counters_vec.lock_mut();
                        if let Some(value) = counters.get_mut(index) {
                            *value += change_amount;
                        }
                    }
                },
                // Reactive grid resizing - cleaner stream pattern
                async {
                    let mut resize_stream = map_ref! {
                        let cols = COLUMNS.count.signal(),
                        let rows = ROWS.count.signal() => cols * rows
                    }.to_stream();
                    
                    while let Some(total_count) = resize_stream.next().await {
                        let mut counters = counters_vec.lock_mut();
                        counters.resize_with(total_count, || 0);
                    }
                }
            );
        });
        CounterGrid { values, change }
    }
}

// Clean global instances - unified type, no duplication
static COLUMNS: Lazy<GridDimensionControl> = lazy::default();
static ROWS: Lazy<GridDimensionControl> = lazy::default();
static COUNTERS: Lazy<CounterGrid> = lazy::default();

fn main() {
    start_app("app", root);
}

pub fn root() -> impl Element {
    Column::new()
        .item(control_panel())
        .item(counters_grid())
}

fn control_panel() -> impl Element {
    Row::new()
        .s(Gap::new().x(20))
        .s(Padding::all(20))
        .item(grid_controls())
        .item(grid_stats())
}

fn grid_controls() -> impl Element {
    Row::new()
        .s(Gap::new().x(10))
        .item(
            Row::new()
                .item("Columns:")
                .item(dimension_control_counter(&COLUMNS))
        )
        .item(
            Row::new()
                .item("Rows:")
                .item(dimension_control_counter(&ROWS))
        )
}

// Unified function - no duplication since controls are identical
fn dimension_control_counter(control: &GridDimensionControl) -> impl Element {
    Row::new()
        .item(
            Button::new()
                .label("-")
                .on_press({
                    let relay = control.decrement.clone();
                    move || { relay.send(()); }
                })
        )
        .item_signal(control.count.signal())
        .item(
            Button::new()
                .label("+")
                .on_press({
                    let relay = control.increment.clone();
                    move || { relay.send(()); }
                })
        )
}

fn grid_stats() -> impl Element {
    let count_signal = map_ref! {
        let columns = COLUMNS.count.signal(),
        let rows = ROWS.count.signal() => columns * rows
    };
    
    Row::new()
        .s(Gap::new().x(15))
        .item_signal(
            count_signal.map(|count| 
                format!("Total Counters: {}", count)
            )
        )
        .item_signal(
            count_signal.map(|count|
                format!("Thousands: {:.2}", count as f64 / 1000.0)
            )
        )
}

fn counters_grid() -> impl Element {
    // Use the computed count signal to create a flat counter list
    let count_signal = map_ref! {
        let columns = COLUMNS.count.signal(),
        let rows = ROWS.count.signal() => (columns, rows, columns * rows)
    };
    
    Column::new()
        .s(Gap::new().y(10))
        .s(Padding::all(20))
        .child_signal(count_signal.map(|(columns, rows, _total)| {
            // Create grid layout
            Column::new()
                .s(Gap::new().y(10))
                .items((0..rows).map(|row| {
                    Row::new()
                        .s(Gap::new().x(10))
                        .items((0..columns).map(move |col| {
                            let index = grid_index(row, col, columns);
                            counter_widget(index)
                        }))
                }))
                .into_element()
        }))
}

fn counter_widget(index: usize) -> impl Element {
    Row::new()
        .s(Padding::all(5))
        .s(Background::new().color(color!("#f0f0f0")))
        .s(RoundedCorners::all(5))
        .item(
            Button::new()
                .label("-")
                .on_press(move || {
                    COUNTERS.change.send((index, -1));
                })
        )
        .item_signal(
            COUNTERS.values.signal_vec().map_ref(move |counters| {
                counters.get(index).copied().unwrap_or(0)
            }).map(|value| El::new().s(Padding::new().x(10)).child(value))
        )
        .item(
            Button::new()
                .label("+")
                .on_press(move || {
                    COUNTERS.change.send((index, 1));
                })
        )
}
```

## Global Benefits: Paired Global Structs

### ✅ Eliminated Anti-patterns:
- **No INIT_ONCE**: Clean `Default` implementations wire everything automatically
- **Unified Types**: `GridDimensionControl` for both columns and rows - no duplication
- **No sync_* Methods**: Pure reactive composition using `map_ref!` patterns
- **Vec Indices**: `(index, change_amount)` tuples instead of String IDs
- **No Type Conversions**: Pure `usize` with `saturating_sub(1).max(1)` - no `as i32`/`as usize`

### 🎯 Perfect for:
Applications that naturally need global state (like NovyWave's file manager, timeline controls, etc.)

### ✅ Benefits Over Original MoonZone:
- **Clean Global Access**: `COLUMNS.increment.send(())` - no parameter passing needed
- **Unified Types**: Single `GridDimensionControl` eliminates code duplication
- **Race-Free Operations**: Atomic relay operations instead of get/set races
- **Reactive Composition**: Grid automatically resizes when dimensions change
- **Event Traceability**: All state changes go through typed events

## Key Race-Condition Prevention

### ❌ Original MoonZone Pattern (Problematic):
```rust
// Race condition possible between read and write
static STORE: Lazy<Store> = lazy::default();
fn increment_column() {
    let current = STORE.column_count.get();  // Read
    STORE.column_count.set(current + 1);     // Write - race possible!
}
```

### ✅ Global Actor+Relay Pattern (Safe):
```rust
// Atomic operation - no race conditions possible
static COLUMNS: Lazy<GridDimensionControl> = lazy::default();
fn increment_column() {
    COLUMNS.increment.send(());  // Single atomic operation
}
```

## Testing Global Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // Helper to reset global state between tests
    async fn reset_global_state() {
        // Reset to 5x5 grid (default)
        while COLUMNS.count.get() > 5 {
            COLUMNS.decrement.send(());
        }
        while COLUMNS.count.get() < 5 {
            COLUMNS.increment.send(());
        }
        // Similar for ROWS...
        
        // Reset all counter values to 0
        let current_values = COUNTERS.values.signal_vec_cloned().to_signal_cloned().get_cloned();
        for (index, value) in current_values.iter().enumerate() {
            if *value != 0 {
                COUNTERS.change.send((index, -value));
            }
        }
    }
    
    #[async_test]
    async fn test_global_grid_resize() {
        reset_global_state().await;
        
        // Change to 3x3 using atomic operations
        COLUMNS.decrement.send(()); // 5 -> 4
        COLUMNS.decrement.send(()); // 4 -> 3
        ROWS.decrement.send(());    // 5 -> 4  
        ROWS.decrement.send(());    // 4 -> 3
        
        let mut len_stream = COUNTERS.values.signal_vec_cloned().len().to_stream();
        assert_eq!(len_stream.next().await.unwrap(), 9);
    }
    
    #[async_test]
    async fn test_global_counter_increment() {
        reset_global_state().await;
        
        // Increment counter at index 0 (top-left)
        COUNTERS.change.send((0, 3));
        
        let mut values_stream = COUNTERS.values.signal_vec_cloned().to_signal_cloned().to_stream();
        let values = values_stream.next().await.unwrap();
        assert_eq!(values[0], 3);
    }
    
    #[async_test]
    async fn test_grid_index_calculation() {
        // 5x5 grid: index = row * columns + col
        assert_eq!(grid_index(0, 0, 5), 0);  // top-left
        assert_eq!(grid_index(0, 4, 5), 4);  // top-right
        assert_eq!(grid_index(1, 0, 5), 5);  // second row, first col
        assert_eq!(grid_index(2, 3, 5), 13); // third row, fourth col
    }
}
```

## Key Insights for Global Patterns

### 1. **Eliminate Race Conditions AND Type Conversions**
- **❌ get/send antipattern**: `relay.send(actor.get() + 1)` - race condition between get() and send()
- **❌ Number conversion boilerplate**: `as i32` and `as usize` everywhere for simple operations
- **✅ Separate increment/decrement relays**: `increment.send(())` - atomic, pure usize, simple
- **✅ saturating_sub().max(1)**: Standard library math instead of type conversions

### 2. **Unify Identical Types and Methods**  
- **❌ Duplicate structs**: `ColumnControl` and `RowControl` with identical implementations
- **✅ Single unified type**: `GridDimensionControl` for both - eliminates duplication
- **✅ Single unified method**: `dimension_control_counter()` works for both columns and rows
- **✅ Derive Default**: `#[derive(Default)]` instead of manual implementations

### 3. **Reactive Composition Patterns**
- **❌ .for_each() closures**: Need to clone captured variables, more complex
- **✅ to_stream() -> while**: Cleaner imperative style, easier to debug, consistent patterns
- **✅ map_ref! for derived signals**: Automatic updates when dependencies change
- **✅ Reactive grid resizing**: Grid automatically resizes when dimensions change

## Migration Notes

This global pattern serves as a bridge between traditional MoonZone globals and idiomatic Actor+Relay local state.

### When Global Patterns Are Appropriate:
- **Complex shared state**: Grid dimensions that affect multiple UI components
- **Cross-cutting concerns**: State that genuinely needs global access
- **Legacy migration**: Transitioning from existing global Mutable patterns
- **Singleton services**: When you need exactly one instance across the application

### Trade-offs vs Local State:
- **Less composable**: Harder to create multiple independent counter grids
- **Testing complexity**: Need to manage global state between test runs
- **Hidden dependencies**: Components implicitly depend on global state
- **Reduced reusability**: Components tied to specific global instances

The key insight: **Global Actor+Relay eliminates the concurrency bugs and complex signal management of the original MoonZone approach while preserving convenient global access when it's genuinely needed.**