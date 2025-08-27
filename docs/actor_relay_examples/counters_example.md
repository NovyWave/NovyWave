# Actor+Relay Example: Counters (Dynamic Collection)

This example shows how to transform a complex multi-counter application with dynamic grid sizing from traditional MoonZone patterns to clean Actor+Relay architecture using simplified patterns.

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

## Actor+Relay Version (Simplified)

There are two clean approaches to organizing Actor+Relay state. Both eliminate common concurrency bugs while staying simple:

### Approach A: Paired Global Structs (Clean Globals)

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

// End of Approach A
```

### Approach B: No Globals, Struct with Methods (Most Idiomatic Rust)

```rust
use zoon::*;

// Same helper function and struct definitions from above...
// [ColumnControl, RowControl, CounterGrid with increment/decrement relays]

/// App struct containing all state - enables clean self usage in UI methods
#[derive(Clone, Default)]
struct CounterApp {
    columns: GridDimensionControl,
    rows: GridDimensionControl,
    counters: CounterGrid,
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

// Alternative main function for Approach B
fn main_with_local_state() {
    start_app("app", || CounterApp::default().root());
}
```

## Two Approaches Compared

Both approaches eliminate all the anti-patterns while showing different organization styles:

### Approach A Benefits: **Paired Global Structs**

**✅ Eliminated Anti-patterns:**
- **No INIT_ONCE**: Clean `Default` implementations wire everything automatically
- **Unified Types**: `GridDimensionControl` for both columns and rows - no duplication
- **No sync_* Methods**: Pure reactive composition using `map_ref!` patterns
- **Vec Indices**: `(index, change_amount)` tuples instead of String IDs
- **No Type Conversions**: Pure `usize` with `saturating_sub(1).max(1)` - no `as i32`/`as usize`

**🎯 Perfect for:** Applications that naturally need global state (like NovyWave's file manager, timeline controls, etc.)

### Approach B Benefits: **Struct with Methods (Most Idiomatic Rust)**

**✅ Natural Rust Patterns:**
- **Struct with Methods**: `impl CounterApp` with `self` usage - no parameter passing needed
- **Derived Default**: `#[derive(Clone, Default)]` - no manual implementation needed
- **Unified Methods**: Single `dimension_control_counter()` method works for both columns and rows
- **Idiomatic Design**: Feels like natural Rust struct usage, not forced functional patterns

**🎯 Perfect for:** Complex apps where passing context everywhere becomes unwieldy

### Code Comparison

```rust
// Approach A: Globals (most concise)
fn column_control_counter() -> impl Element {
    // Direct global access - no parameters needed
    Button::new().on_press(|| COLUMNS.increment.send(()))
}

// Approach B: Struct with Methods (most idiomatic Rust) 
impl CounterApp {
    fn dimension_control_counter(&self, control: &GridDimensionControl) -> impl Element {
        // Clean self usage - unified method, no duplication
        Button::new().on_press({
            let relay = control.increment.clone();
            move || relay.send(())
        })
    }
}
```

### Key Insights for actors_and_relays.md

**1. Eliminate Race Conditions AND Type Conversions**
- **❌ get/send antipattern**: `relay.send(actor.get() + 1)` - race condition between get() and send()
- **❌ Number conversion boilerplate**: `as i32` and `as usize` everywhere for simple operations
- **✅ Separate increment/decrement relays**: `increment.send(())` - atomic, pure usize, simple
- **✅ saturating_sub().max(1)**: Standard library math instead of type conversions

**2. Unify Identical Types and Methods**  
- **❌ Duplicate structs**: `ColumnControl` and `RowControl` with identical implementations
- **✅ Single unified type**: `GridDimensionControl` for both - eliminates duplication
- **✅ Single unified method**: `dimension_control_counter()` works for both columns and rows
- **✅ Derive Default**: `#[derive(Default)]` instead of manual implementations

**3. Choose Idiomatic Organization Patterns**
- **Approach A (Globals)**: Free functions, functional style, shared state across app
- **Approach B (Struct Methods)**: `impl CounterApp` with `self` - natural Rust, no context passing
- **Use self when you have local state** - don't fight Rust with forced functional patterns

**4. Fix Lifetime and Compilation Issues**
- **❌ Lifetime bugs**: `|| { let app = CounterApp::default(); app.root() }` won't compile  
- **✅ Simple pattern**: `|| CounterApp::default().root()` - works because actors are Arc internally
- **✅ No complex lifetime management**: Keep it simple and let Rust's ownership work

**5. Reactive Composition Patterns**
- **❌ .for_each() closures**: Need to clone captured variables, more complex
- **✅ to_stream() -> while**: Cleaner imperative style, easier to debug, consistent patterns

**6. Remove Complexity, Don't Add It**
- Simple operations should stay simple: `increment.send(())` vs complex enum patterns
- No unnecessary abstractions: direct field access, no wrapper methods  
- Pure data types: `usize` throughout instead of mixed `i32`/`usize` conversions
- Unify identical code instead of duplicating

The key insight: **Actor+Relay should prevent concurrency bugs while feeling like natural, simple Rust code.**

## Tests

Both approaches are easily testable with direct state access:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // Test Approach A (Globals)
    #[async_test]
    async fn test_grid_resize_globals() {
        // Change to 3x3 using atomic operations (avoid multiple source locations)
        for _ in 0..2 {
            Task::start(async { COLUMNS.decrement.send(()); });
        }
        for _ in 0..2 {
            Task::start(async { ROWS.decrement.send(()); });
        }
        
        let mut len_stream = COUNTERS.values.signal_vec_cloned().len().to_stream();
        assert_eq!(len_stream.next().await.unwrap(), 9);
    }
    
    #[async_test]
    async fn test_counter_increment_globals() {
        // Increment counter at index 0 (top-left)
        COUNTERS.change.send((0, 3));
        
        let mut values_stream = COUNTERS.values.signal_vec_cloned().to_signal_cloned().to_stream();
        let values = values_stream.next().await.unwrap();
        assert_eq!(values[0], 3);
    }
    
    // Test Approach B (Struct with Methods)
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
    async fn test_grid_index_calculation() {
        // 5x5 grid: index = row * columns + col
        assert_eq!(grid_index(0, 0, 5), 0);  // top-left
        assert_eq!(grid_index(0, 4, 5), 4);  // top-right
        assert_eq!(grid_index(1, 0, 5), 5);  // second row, first col
        assert_eq!(grid_index(2, 3, 5), 13); // third row, fourth col
    }
}
```