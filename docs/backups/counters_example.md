# Actor+Relay Example: Counters (Dynamic Collection)

This example shows how to transform a complex multi-counter application with dynamic grid sizing from traditional MoonZone patterns to clean Actor+Relay architecture using simplified patterns.

> **📄 Related Documentation**: For global state patterns using Actor+Relay, see [`counters_example_global.md`](counters_example_global.md). This file focuses on the recommended local state approach.

## Original MoonZone Counters

```rust
use std::iter;
use zoon::{format, *};

mod counter;
use counter::Counter;

// Global state with multiple concerns mixed together
#[derive(Educe)]
#[educe(Default)]
struct Store {
    #[educe(Default(expression = Mutable::new(5)))]
    column_count: Mutable<usize>,

    #[educe(Default(expression = Mutable::new(5)))]
    row_count: Mutable<usize>,

    test_counter_value: Mutable<i32>,
}

static STORE: Lazy<Store> = lazy::default();

fn main() {
    start_app("app", root);
}

pub fn root() -> impl Element {
    Column::new().item(control_counters()).item(counters())
}

fn control_counters() -> impl Element {
    // Complex signal computation scattered throughout
    let counter_count = map_ref! {
        let column_count = STORE.column_count.signal(),
        let row_count = STORE.row_count.signal() =>
        column_count * row_count
    }
    .broadcast();
    
    Row::new()
        .item(column_counter())
        .item(row_counter())
        .item(
            El::new().child_signal(
                counter_count
                    .signal()
                    .map(|count| format!("Counters: {}", count)),
            ),
        )
        .item(
            El::new().child_signal(
                counter_count
                    .signal()
                    .map(|count| format!("Thousands: {:.2}", count as f64 / 1_000.)),
            ),
        )
        .item(test_counters())
        .item(click_me_button())
}

fn column_counter() -> impl Element {
    Row::new().item("Columns:").item(
        Counter::with_signal(STORE.column_count.signal())
            .on_change(|value| STORE.column_count.set(value))  // Direct store mutation
            .step(5),
    )
}

fn row_counter() -> impl Element {
    Row::new().item("Rows:").item(
        Counter::with_signal(STORE.row_count.signal())
            .on_change(|value| STORE.row_count.set(value))     // Direct store mutation
            .step(5),
    )
}

// Complex utility to sync signal to mutable vec - hard to understand
fn count_signal_to_mutable_vec(
    count: impl Signal<Item = usize> + 'static,
) -> (MutableVec<()>, TaskHandle) {
    let mutable_vec = MutableVec::new();
    let mutable_vec_updater =
        Task::start_droppable(count.for_each_sync(clone!((mutable_vec) move |count| {
            let mut mutable_vec = mutable_vec.lock_mut();
            let current_count = mutable_vec.len();
            if count > current_count {
                mutable_vec.extend(iter::repeat(()).take(count - current_count))
            } else if count < current_count {
                mutable_vec.truncate(count)
            }
        })));
    (mutable_vec, mutable_vec_updater)
}

fn counters() -> impl Element {
    // Complex dynamic collection management
    let (columns, columns_updater) = count_signal_to_mutable_vec(STORE.column_count.signal());
    Row::new()
        .items_signal_vec(columns.signal_vec().map(|()| counter_column()))
        .after_remove(move |_| drop(columns_updater))
}

fn counter_column() -> impl Element {
    let (rows, rows_updater) = count_signal_to_mutable_vec(STORE.row_count.signal());
    Column::new()
        .items_signal_vec(rows.signal_vec().map(|()| Counter::new(0)))
        .after_remove(move |_| drop(rows_updater))
}
```

### Problems with Original Approach:
- **Monolithic Store**: All concerns mixed in one global struct
- **Complex Signal Management**: Manual synchronization between signals and collections
- **Unclear Responsibilities**: Who owns what state?
- **Tight Coupling**: UI components directly mutate store
- **Hard to Test**: Complex interdependencies
- **Difficult to Extend**: Adding features requires touching many places

## Actor+Relay Version (Local State)

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

impl CounterGrid {
    fn new(columns_signal: impl Signal<Item = usize> + 'static, rows_signal: impl Signal<Item = usize> + 'static) -> Self {
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
                        let cols = columns_signal,
                        let rows = rows_signal => cols * rows
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

/// App struct containing all state - enables clean self usage in UI methods
#[derive(Clone)]
struct CounterApp {
    columns: GridDimensionControl,
    rows: GridDimensionControl,
    counters: CounterGrid,
}

impl Default for CounterApp {
    fn default() -> Self {
        let columns = GridDimensionControl::default();
        let rows = GridDimensionControl::default();
        let counters = CounterGrid::new(columns.count.signal(), rows.count.signal());
        
        CounterApp { columns, rows, counters }
    }
}

impl CounterApp {
    // UI methods using self - much cleaner than parameter passing
    fn root(&self) -> impl Element {
        Column::new()
            .item(self.control_panel())
            .item(self.counters_grid())
    }
    
    fn control_panel(&self) -> impl Element {
        Row::new()
            .s(Gap::new().x(20))
            .s(Padding::all(20))
            .item(self.grid_controls())
            .item(self.grid_stats())
    }
    
    fn grid_controls(&self) -> impl Element {
        Row::new()
            .s(Gap::new().x(10))
            .item(
                Row::new()
                    .item("Columns:")
                    .item(self.dimension_control_counter(&self.columns))
            )
            .item(
                Row::new()
                    .item("Rows:")
                    .item(self.dimension_control_counter(&self.rows))
            )
    }
    
    // Unified method - no duplication since both controls are identical
    fn dimension_control_counter(&self, control: &GridDimensionControl) -> impl Element {
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
    
    fn grid_stats(&self) -> impl Element {
        let count_signal = map_ref! {
            let cols = self.columns.count.signal(),
            let rows = self.rows.count.signal() => cols * rows
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
    
    fn counters_grid(&self) -> impl Element {
        let count_signal = map_ref! {
            let cols = self.columns.count.signal(),
            let rows = self.rows.count.signal() => (cols, rows, cols * rows)
        };
        
        Column::new()
            .s(Gap::new().y(10))
            .s(Padding::all(20))
            .child_signal(count_signal.map({
                let counters = self.counters.clone();
                move |(columns, rows, _total)| {
                    Column::new()
                        .s(Gap::new().y(10))
                        .items((0..rows).map(|row| {
                            Row::new()
                                .s(Gap::new().x(10))
                                .items((0..columns).map(move |col| {
                                    let index = grid_index(row, col, columns);
                                    Self::counter_widget(index, &counters)
                                }))
                        }))
                        .into_element()
                }
            }))
    }
    
    // Static method since it doesn't need self
    fn counter_widget(index: usize, counters: &CounterGrid) -> impl Element {
        Row::new()
            .s(Padding::all(5))
            .s(Background::new().color(color!("#f0f0f0")))
            .s(RoundedCorners::all(5))
            .item(
                Button::new()
                    .label("-")
                    .on_press({
                        let relay = counters.change.clone();
                        move || {
                            relay.send((index, -1));
                        }
                    })
            )
            .item_signal(
                counters.values.signal_vec().map_ref(move |values| {
                    values.get(index).copied().unwrap_or(0)
                }).map(|value| El::new().s(Padding::new().x(10)).child(value))
            )
            .item(
                Button::new()
                    .label("+")
                    .on_press({
                        let relay = counters.change.clone();
                        move || {
                            relay.send((index, 1));
                        }
                    })
            )
    }
}

fn main() {
    start_app("app", || CounterApp::default().root());
}
```

## Key Benefits of Actor+Relay Counters

### 1. **🔒 Race-Condition Prevention**
- **Atomic operations**: `increment.send(())` and `decrement.send()` - no get/set races
- **Sequential processing**: All state changes processed in order through Actor streams
- **No concurrent mutations**: Multiple UI components can safely trigger changes

### 2. **🎯 Clean Architecture**
- **Unified types**: Single `GridDimensionControl` for both columns and rows
- **Local state**: All state contained within `CounterApp` struct
- **Method-based API**: Natural `self.dimension_control_counter()` usage
- **Reactive composition**: Grid automatically resizes when dimensions change

### 3. **⚡ Simplified State Management**
- **No type conversions**: Pure `usize` throughout with `saturating_sub(1).max(1)`
- **Direct operations**: Simple `increment.send(())` vs complex enum patterns
- **Event traceability**: All state changes go through typed relay events
- **Testable design**: Each component can be instantiated and tested independently

### 4. **🛠️ Idiomatic Rust Patterns**
- **Struct with methods**: `impl CounterApp` with `self` - natural Rust style
- **Clean ownership**: No lifetime management complexity
- **Composable components**: Easy to create multiple counter grid instances
- **Standard patterns**: Uses `Default` trait and standard Rust conventions

## Tests

The local state approach is easily testable with isolated instances:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[async_test]
    async fn test_counter_app_creation() {
        let app = CounterApp::default();
        
        // Test atomic increment/decrement operations
        app.columns.increment.send(()); // 5 + 1 = 6
        app.rows.decrement.send(());     // 5 - 1 = 4 (min 1 enforced)
        
        // Test counter change
        app.counters.change.send((0, 5));
        
        let mut values_stream = app.counters.values.signal_vec_cloned().to_signal_cloned().to_stream();
        let values = values_stream.next().await.unwrap();
        assert_eq!(values[0], 5);
    }
    
    #[async_test]
    async fn test_grid_resize() {
        let app = CounterApp::default();
        
        // Change to 3x3 grid
        for _ in 0..2 {
            app.columns.decrement.send(()); // 5 -> 4 -> 3
        }
        for _ in 0..2 {
            app.rows.decrement.send(());    // 5 -> 4 -> 3  
        }
        
        let mut len_stream = app.counters.values.signal_vec_cloned().len().to_stream();
        assert_eq!(len_stream.next().await.unwrap(), 9);
    }
    
    #[async_test]
    async fn test_grid_index_calculation() {
        // 5x5 grid: index = row * columns + col
        assert_eq!(grid_index(0, 0, 5), 0);  // top-left
        assert_eq!(grid_index(0, 4, 5), 4);  // top-right
        assert_eq!(grid_index(1, 0, 5), 5);  // second row, first col
        assert_eq!(grid_index(2, 3, 5), 13); // third row, fourth col
    }
    
    #[async_test] 
    async fn test_multiple_counter_apps() {
        // Local state allows multiple independent instances
        let app1 = CounterApp::default();
        let app2 = CounterApp::default();
        
        // Each app manages its own state independently
        app1.columns.increment.send(());
        app2.rows.increment.send(());
        
        // Changes don't affect each other
        let mut app1_cols = app1.columns.count.signal().to_stream();
        let mut app2_rows = app2.rows.count.signal().to_stream();
        
        assert_eq!(app1_cols.next().await.unwrap(), 6);
        assert_eq!(app2_rows.next().await.unwrap(), 6);
    }
}
```