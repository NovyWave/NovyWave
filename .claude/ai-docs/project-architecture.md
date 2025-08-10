# NovyWave Project Architecture

## Component Patterns and Conventions

### NovyUI Design System
```rust
// Icon usage - always use enum tokens, never strings
button()
    .left_icon(IconName::Folder)  // ✓ Correct
    .left_icon("folder")          // ✗ Never use strings

// Theme-aware colors using design tokens
.s(Background::new().color_signal(neutral_3().signal()))
.s(Font::new().color_signal(neutral_11().signal()))

// Layout patterns
Row::new()
    .s(Gap::new().x(8))           // Normal spacing
    .s(Align::new().center_y())   // Vertical centering
    .item(title_element)
    .item(El::new().s(Width::fill()))  // Spacer
    .item(action_button)
```

### Height Inheritance Pattern
```rust
// Critical height inheritance chain - missing Height::fill() breaks it
El::new().s(Height::screen())     // Root claims viewport
  .child(Column::new().s(Height::fill())    // All containers inherit
    .item(Row::new().s(Height::fill())      // Every container needs fill
      .item(panel_content)))
```

### TreeView Component Architecture
```rust
// TreeView with external state management
TreeView::new()
    .external_expanded_signal(EXPANDED_DIRECTORIES.signal())
    .external_selected_vec_signal(SELECTED_ITEMS.signal_vec_cloned())
    .single_scope_selection(true)  // Radio button behavior
    .item_signal(tree_data.signal_vec_cloned().map(...))
```

## Configuration System (TOML + Reactive Persistence)

### Dual-Layer Config Architecture
```rust
// shared/lib.rs - Backend schema
#[derive(Serialize, Deserialize)]
pub struct WorkspaceSection {
    pub dock_mode: String,
    pub panel_dimensions_right: PanelDimensions,
    pub panel_dimensions_bottom: PanelDimensions,
    pub selected_scope_id: Option<String>,
    pub expanded_scopes: IndexSet<String>,
}

// frontend/config.rs - Extended frontend structure
#[derive(Clone)]
pub struct PanelLayouts {
    pub docked_to_right: Mutable<PanelDimensions>,
    pub docked_to_bottom: Mutable<PanelDimensions>,
}
```

### Config Field Addition Pattern
```rust
// THREE locations required for new config fields:
// 1. shared/lib.rs types
pub struct WorkspaceSection {
    #[serde(default)]  // Always use default for new fields
    pub new_field: Option<NewType>,
}

// 2. frontend/config.rs SerializableConfig
pub struct SerializableConfig {
    pub new_field: Option<NewType>,
}

// 3. load_from_serializable() method
impl ConfigStore {
    fn load_from_serializable(&self, config: SerializableConfig) {
        if let Some(value) = config.new_field {
            self.new_field.set(value);
        }
    }
}
```

### Reactive Config Persistence
```rust
// Pattern: Signal monitoring + save triggers
fn init_config_handlers() {
    // Theme changes
    Task::start(current_theme().signal().for_each_sync(|_| {
        save_current_config();
    }));
    
    // Panel dimensions
    Task::start(FILES_PANEL_WIDTH.signal().for_each_sync(|_| {
        save_current_config();
    }));
}

// Initialization order prevents overwrites
pub fn initialize_config() -> impl Future<Output = ()> {
    async move {
        load_config().await;
        CONFIG_LOADED.set_neq(true);  // Gate flag
        init_config_handlers();       // Start reactive triggers
    }
}
```

## Theme System Implementation

### Theme-Aware Signal Patterns
```rust
// Reactive theme switching
.s(Background::new().color_signal(
    theme().signal().map(|t| match t {
        Theme::Light => neutral_1(),
        Theme::Dark => neutral_12(),
    })
))

// Scrollbar theming
.style_signal("scrollbar-color", 
    primary_6().signal().map(|thumb| 
        primary_3().signal().map(move |track| 
            format!("{} {}", thumb, track)
        )
    ).flatten()
)
```

### Color Token Usage
```rust
// Text colors
Font::new().color_signal(neutral_11().signal())  // Primary text
Font::new().color_signal(neutral_8().signal())   // Secondary/dimmed

// Background colors
Background::new().color_signal(neutral_1().signal())   // Main background
Background::new().color_signal(neutral_3().signal())   // Panel background
Background::new().color_signal(primary_6().signal())   // Accent elements
```

## File Handling and State Management

### File Loading Architecture
```rust
// Dual state system: Legacy globals + ConfigStore
static FILE_PATHS: Lazy<MutableVec<String>> = Lazy::new(MutableVec::new);
static EXPANDED_SCOPES: Lazy<Mutable<HashSet<String>>> = Lazy::new(|| Mutable::new(HashSet::new()));

// Bidirectional sync pattern
fn sync_globals_to_config() {
    let file_paths: Vec<String> = FILE_PATHS.lock_ref().to_vec();
    CONFIG_STORE.with(|store| {
        store.opened_files.set_neq(file_paths);
        // Manual save trigger needed when reactive signals fail
        save_config_to_backend();
    });
}
```

### Smart File Labeling System
```rust
// VSCode-style filename disambiguation
fn create_smart_labels(files: &[TrackedFile]) -> Vec<String> {
    let mut labels = Vec::new();
    for file in files {
        let filename = file.path.file_name().unwrap_or_default();
        
        // Check for duplicates
        let duplicates: Vec<_> = files.iter()
            .filter(|f| f.path.file_name() == Some(filename))
            .collect();
            
        if duplicates.len() > 1 {
            // Show disambiguating directory prefix
            labels.push(format!("{}/{}", parent_dir, filename));
        } else {
            labels.push(filename.to_string());
        }
    }
    labels
}
```

## Signal-Based Reactive Patterns

### Signal Composition Patterns
```rust
// Unify different signal types
let unified_signal = map_bool_signal(
    condition_signal,
    || first_signal.signal(),
    || second_signal.signal(),
);

// Dynamic element switching with type unification
Stripe::new()
    .direction_signal(dock_mode.signal().map(|mode| {
        if mode.is_docked() { Direction::Column } else { Direction::Row }
    }))
    .item_signal(content_signal.map(|content| {
        match content {
            ContentType::A => element_a().into_element(),  // Type unification
            ContentType::B => element_b().into_element(),
        }
    }))
```

### Performance-Optimized Signals
```rust
// Deduplication for expensive operations
TIMELINE_CURSOR_POSITION.signal()
    .dedupe()  // Prevent redundant triggers
    .for_each_sync(|position| {
        expensive_update(position);
    });

// Conditional signal processing with gates
if CONFIG_LOADED.get() {  // Prevent startup race conditions
    perform_config_operation();
}
```

### State Management Patterns
```rust
// MutableVec for reactive collections
static SELECTED_VARIABLES: Lazy<MutableVec<SelectedVariable>> = 
    Lazy::new(MutableVec::new);

// HashSet for expansion state
static EXPANDED_SCOPES: Lazy<Mutable<HashSet<String>>> = 
    Lazy::new(|| Mutable::new(HashSet::new()));

// Bridge pattern for compatibility
fn bridge_to_external_selected() -> impl Signal<Item = Vec<TreeId>> {
    SELECTED_ITEMS.signal_vec_cloned()
        .map_ref(|items| items.iter().map(|item| TreeId(item.id.clone())).collect())
}
```

## Dock Mode Architecture

### Dock Mode Configuration
```rust
// Per-dock-mode storage
#[derive(Clone)]
pub enum DockMode {
    Right,
    Bottom,
}

// Separate dimensions per mode
pub struct WorkspaceSection {
    pub panel_dimensions_right: PanelDimensions,
    pub panel_dimensions_bottom: PanelDimensions,
}

// Layout switching
fn main_layout() -> impl Element {
    El::new()
        .child_signal(IS_DOCKED_TO_BOTTOM.signal().map(|docked| {
            if docked {
                docked_layout().into_element()
            } else {
                undocked_layout().into_element()
            }
        }))
}
```

### Panel Dimension Preservation
```rust
// Switch modes while preserving dimensions
fn switch_dock_mode_preserving_dimensions() {
    let current_dims = get_current_panel_dimensions();
    IS_DOCKED_TO_BOTTOM.set_neq(!IS_DOCKED_TO_BOTTOM.get());
    save_panel_dimensions_for_current_mode(current_dims);
    save_current_config();
}
```