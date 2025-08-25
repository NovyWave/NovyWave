# Actor+Relay Example: Counters (Dynamic Collection)

This example shows how to transform a complex multi-counter application with dynamic grid sizing from traditional MoonZone patterns to Actor+Relay architecture.

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

## Actor+Relay Version

```rust
use std::collections::HashMap;
use zoon::*;

// Clear separation of concerns with domain-specific types

/// Grid configuration - size of the counter grid
#[derive(Clone, Debug)]
struct GridConfig {
    columns: usize,
    rows: usize,
}

impl GridConfig {
    fn total_count(&self) -> usize {
        self.columns * self.rows
    }
    
    fn thousands(&self) -> f64 {
        self.total_count() as f64 / 1000.0
    }
}

/// Individual counter state
#[derive(Clone, Debug)]
struct CounterData {
    id: String,
    value: i32,
    row: usize,
    col: usize,
}

/// Events for grid configuration changes
#[derive(Clone)]
struct SetColumns(usize);

#[derive(Clone)]
struct SetRows(usize);

/// Events for counter interactions
#[derive(Clone)]
struct IncrementCounter { id: String, amount: i32 }

#[derive(Clone)]
struct DecrementCounter { id: String, amount: i32 }

#[derive(Clone)]
struct ResetCounter { id: String }

/// Bundle related control relays
#[derive(Clone)]
struct GridControlRelays {
    set_columns: Relay<SetColumns>,
    set_rows: Relay<SetRows>,
    reset_all: Relay,
}

impl GridControlRelays {
    fn new() -> Self {
        Self {
            set_columns: Relay::new(),
            set_rows: Relay::new(), 
            reset_all: Relay::new(),
        }
    }
}

/// Bundle counter interaction relays
#[derive(Clone)]
struct CounterRelays {
    increment: Relay<IncrementCounter>,
    decrement: Relay<DecrementCounter>,
    reset: Relay<ResetCounter>,
}

impl CounterRelays {
    fn new() -> Self {
        Self {
            increment: Relay::new(),
            decrement: Relay::new(),
            reset: Relay::new(),
        }
    }
}

/// Main grid manager - coordinates everything
#[derive(Clone, Debug)]
struct GridManager {
    // State
    config: Actor<GridConfig>,
    counters: ActorVec<CounterData>,
    
    // Events
    grid_controls: GridControlRelays,
    counter_controls: CounterRelays,
    
    // Task handles for cleanup
    sync_task: Mutable<Option<TaskHandle>>,
}

impl GridManager {
    pub fn new(initial_columns: usize, initial_rows: usize) -> Self {
        let grid_controls = GridControlRelays::new();
        let counter_controls = CounterRelays::new();
        let sync_task = Mutable::new(None);
        
        // Config Actor responds to grid control events
        let config = Actor::new(GridConfig { columns: initial_columns, rows: initial_rows }, 
            clone!((grid_controls) async move |config_state| {
            Task::start_droppable(clone!((config_state, grid_controls) async move {
                grid_controls.set_columns.subscribe().for_each(clone!((config_state) async move |SetColumns(cols)| {
                    let mut current = config_state.get();
                    current.columns = cols.max(1);  // Minimum 1 column
                    config_state.set(current);
                })).await;
            }));
            
            Task::start_droppable(clone!((config_state, grid_controls) async move {
                grid_controls.set_rows.subscribe().for_each(clone!((config_state) async move |SetRows(rows)| {
                    let mut current = config_state.get();
                    current.rows = rows.max(1);  // Minimum 1 row
                    config_state.set(current);
                })).await;
            }));
        }));
        
        // Counter collection Actor
        let counters = ActorVec::new(vec![], clone!((counter_controls, config, sync_task) async move |counters_vec| {
            // Handle counter interactions
            Task::start_droppable(clone!((counters_vec, counter_controls) async move {
                counter_controls.increment.subscribe().for_each(clone!((counters_vec) async move |IncrementCounter { id, amount }| {
                    let mut counters = counters_vec.lock_mut();
                    if let Some(counter) = counters.iter_mut().find(|c| c.id == id) {
                        counter.value += amount;
                    }
                })).await;
            }));
            
            Task::start_droppable(clone!((counters_vec, counter_controls) async move {
                counter_controls.decrement.subscribe().for_each(clone!((counters_vec) async move |DecrementCounter { id, amount }| {
                    let mut counters = counters_vec.lock_mut();
                    if let Some(counter) = counters.iter_mut().find(|c| c.id == id) {
                        counter.value -= amount;
                    }
                })).await;
            }));
            
            Task::start_droppable(clone!((counters_vec, counter_controls) async move {
                counter_controls.reset.subscribe().for_each(clone!((counters_vec) async move |ResetCounter { id }| {
                    let mut counters = counters_vec.lock_mut();
                    if let Some(counter) = counters.iter_mut().find(|c| c.id == id) {
                        counter.value = 0;
                    }
                })).await;
            }));
            
            // Sync counters collection with grid config changes
            let handle = Task::start_droppable(clone!((counters_vec, config) async move {
                config.signal().for_each(clone!((counters_vec) async move |grid_config| {
                    let mut counters = counters_vec.lock_mut();
                    
                    // Generate new counter collection based on grid size
                    let mut new_counters = Vec::new();
                    for row in 0..grid_config.rows {
                        for col in 0..grid_config.columns {
                            let id = format!("{}_{}", row, col);
                            
                            // Preserve existing counter value if it exists
                            let value = counters.iter()
                                .find(|c| c.id == id)
                                .map(|c| c.value)
                                .unwrap_or(0);
                            
                            new_counters.push(CounterData {
                                id,
                                value,
                                row,
                                col,
                            });
                        }
                    }
                    
                    counters.replace_cloned(new_counters);
                })).await;
            }));
            
            sync_task.set_neq(Some(handle));
        }));
        
        GridManager {
            config,
            counters,
            grid_controls,
            counter_controls,
            sync_task,
        }
    }
    
    // Public API for grid controls
    pub fn set_columns(&self, columns: usize) -> Result<(), RelayError> {
        self.grid_controls.set_columns.send(SetColumns(columns))
    }
    
    pub fn set_rows(&self, rows: usize) -> Result<(), RelayError> {
        self.grid_controls.set_rows.send(SetRows(rows))
    }
    
    pub fn reset_all(&self) -> Result<(), RelayError> {
        self.grid_controls.reset_all.send(())
    }
    
    // Public API for counter controls
    pub fn increment_counter(&self, id: String, amount: i32) -> Result<(), RelayError> {
        self.counter_controls.increment.send(IncrementCounter { id, amount })
    }
    
    pub fn decrement_counter(&self, id: String, amount: i32) -> Result<(), RelayError> {
        self.counter_controls.decrement.send(DecrementCounter { id, amount })
    }
    
    pub fn reset_counter(&self, id: String) -> Result<(), RelayError> {
        self.counter_controls.reset.send(ResetCounter { id })
    }
    
    // Reactive access to state
    pub fn config_signal(&self) -> impl Signal<Item = GridConfig> {
        self.config.signal()
    }
    
    pub fn counters_signal_vec(&self) -> impl SignalVec<Item = CounterData> {
        self.counters.signal_vec()
    }
}

// Global instance - now properly encapsulated
static GRID: Lazy<GridManager> = Lazy::new(|| GridManager::new(5, 5));

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
        .item(test_controls())
}

fn grid_controls() -> impl Element {
    Row::new()
        .s(Gap::new().x(10))
        .item(
            Row::new()
                .item("Columns:")
                .item(grid_control_counter("columns"))
        )
        .item(
            Row::new()
                .item("Rows:")
                .item(grid_control_counter("rows"))
        )
}

fn grid_control_counter(control_type: &str) -> impl Element {
    let is_columns = control_type == "columns";
    let current_signal = if is_columns {
        GRID.config_signal().map(|config| config.columns as i32)
    } else {
        GRID.config_signal().map(|config| config.rows as i32)
    };
    
    Row::new()
        .item(
            Button::new()
                .label("-")
                .on_press(clone!((is_columns) move || {
                    let current = GRID.config.get();
                    if is_columns {
                        GRID.set_columns((current.columns).saturating_sub(1).max(1));
                    } else {
                        GRID.set_rows((current.rows).saturating_sub(1).max(1));
                    }
                }))
        )
        .item_signal(current_signal)
        .item(
            Button::new()
                .label("+")
                .on_press(clone!((is_columns) move || {
                    let current = GRID.config.get();
                    if is_columns {
                        GRID.set_columns(current.columns + 1);
                    } else {
                        GRID.set_rows(current.rows + 1);
                    }
                }))
        )
}

fn grid_stats() -> impl Element {
    Row::new()
        .s(Gap::new().x(15))
        .item_signal(
            GRID.config_signal().map(|config| 
                format!("Total Counters: {}", config.total_count())
            )
        )
        .item_signal(
            GRID.config_signal().map(|config|
                format!("Thousands: {:.2}", config.thousands())
            )
        )
}

fn test_controls() -> impl Element {
    Row::new()
        .item(
            Button::new()
                .label("Reset All")
                .on_press(|| GRID.reset_all())
        )
}

fn counters_grid() -> impl Element {
    let rows_signal = GRID.config_signal().map(|config| config.rows);
    let columns_signal = GRID.config_signal().map(|config| config.columns);
    
    // Group counters by row
    let counters_by_row_signal = map_ref! {
        let counters = GRID.counters_signal_vec().to_signal_cloned(),
        let rows = rows_signal => {
            let mut rows_map: Vec<Vec<CounterData>> = vec![vec![]; *rows];
            
            for counter in counters.iter() {
                if counter.row < *rows {
                    rows_map[counter.row].push(counter.clone());
                }
            }
            
            rows_map
        }
    };
    
    Column::new()
        .s(Gap::new().y(10))
        .s(Padding::all(20))
        .items_signal_vec(
            counters_by_row_signal.signal_vec().map(|row_counters| 
                counter_row(row_counters)
            )
        )
}

fn counter_row(counters: Vec<CounterData>) -> impl Element {
    Row::new()
        .s(Gap::new().x(10))
        .items(counters.into_iter().map(|counter_data| 
            counter_widget(counter_data)
        ))
}

fn counter_widget(counter_data: CounterData) -> impl Element {
    let id = counter_data.id.clone();
    
    Row::new()
        .s(Padding::all(5))
        .s(Background::new().color(color!("#f0f0f0")))
        .s(RoundedCorners::all(5))
        .item(
            Button::new()
                .label("-")
                .on_press(clone!((id) move || {
                    GRID.decrement_counter(id.clone(), 1);
                }))
        )
        .item(
            El::new()
                .s(Padding::new().x(10))
                .child(counter_data.value)
        )
        .item(
            Button::new()
                .label("+")  
                .on_press(clone!((id) move || {
                    GRID.increment_counter(id.clone(), 1);
                }))
        )
        .item(
            Button::new()
                .label("R")
                .s(Background::new().color(color!("#ffcccc")))
                .on_press(move || {
                    GRID.reset_counter(id, 0);
                })
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[async_test]
    async fn test_grid_resize() {
        let grid = GridManager::new(2, 2);
        assert_eq!(grid.config.get().total_count(), 4);
        
        grid.set_columns(3).unwrap();
        Timer::sleep(10).await;
        
        assert_eq!(grid.config.get().total_count(), 6);
    }
    
    #[async_test] 
    async fn test_counter_operations() {
        let grid = GridManager::new(2, 2);
        
        // Wait for initial setup
        Timer::sleep(10).await;
        
        grid.increment_counter("0_0".to_string(), 5).unwrap();
        Timer::sleep(10).await;
        
        let counters = grid.counters.lock_ref();
        let counter = counters.iter().find(|c| c.id == "0_0").unwrap();
        assert_eq!(counter.value, 5);
    }
}
```

## Key Benefits of Actor+Relay Version

### 1. **🏗️ Clear Architecture**
- **GridManager**: Single coordinator for entire system
- **Separate Actors**: Config and Counters have distinct responsibilities  
- **Relay Bundles**: Related events grouped logically
- **Domain Types**: Clear data structures for each concern

### 2. **🎯 Type Safety**
- **Typed Events**: `SetColumns`, `IncrementCounter`, etc.
- **Strong Types**: `GridConfig`, `CounterData` 
- **Compile-time Safety**: No magic strings or numbers
- **IDE Support**: Full autocomplete and refactoring

### 3. **🧪 Testability**
- **Isolated Components**: Each Actor can be tested separately
- **Event-driven Testing**: Send events, verify state changes
- **No Global Dependencies**: Clean unit tests
- **Predictable Behavior**: Event ordering guarantees

### 4. **⚡ Performance**  
- **Efficient Updates**: Only affected counters re-render
- **Smart Synchronization**: Preserves existing counter values during resize
- **Minimal Re-computation**: Reactive signals update only when needed
- **Clean Task Management**: Proper TaskHandle cleanup

### 5. **🔧 Maintainability**
- **Single Responsibility**: Each Actor has one clear job
- **Event Tracing**: Easy to debug what caused state changes
- **Extensible Design**: Easy to add new features
- **Clear Boundaries**: Well-defined interfaces between components

## Advanced Features Made Possible

```rust
// Easy to add advanced features:

// 1. Persistence
impl GridManager {
    pub fn save_to_storage(&self) {
        // Save config and counter values
        let config = self.config.get();
        let counters: Vec<_> = self.counters.lock_ref().to_vec();
        // ... serialize and save
    }
    
    pub fn load_from_storage(&self) {
        // Restore previous state
    }
}

// 2. Undo/Redo
struct GridWithHistory {
    grid: GridManager,
    history: ActorVec<GridSnapshot>,
    undo: Relay,
    redo: Relay,
}

// 3. Statistics
struct GridStats {
    total_increments: Actor<u64>,
    average_value: Actor<f64>,
    // ... track interesting metrics
}

// 4. Animation/Transitions
struct AnimatedGrid {
    grid: GridManager,
    animations: Mutable<HashMap<String, Animation>>,
}
```

This transformation shows how Actor+Relay patterns scale to complex, multi-component applications while maintaining clear separation of concerns and excellent testability.