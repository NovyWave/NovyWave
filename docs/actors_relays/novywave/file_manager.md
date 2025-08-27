# NovyWave File Manager with Actor+Relay

## Waveform File Management Pattern

This example shows how NovyWave manages waveform files (.vcd, .fst) using Actor+Relay architecture, replacing the problematic global `TRACKED_FILES` pattern.

## Domain-Specific Types

```rust
use std::path::PathBuf;

#[derive(Clone, Debug)]
enum FileState {
    Loading,
    Parsed { 
        signals: Vec<WaveformSignal>,
        time_range: (f64, f64),
    },
    Error(String),
}

#[derive(Clone, Debug)]
struct WaveformSignal {
    name: String,
    scope: String,
    bit_width: u32,
    transitions: Vec<SignalTransition>,
}

#[derive(Clone, Debug)]
struct SignalTransition {
    time: f64,
    value: String,
}
```

## TrackedFile Actor Pattern

```rust
#[derive(Clone, Debug)]
pub struct TrackedFile {
    pub id: String,
    pub path: PathBuf,
    pub state: Actor<FileState>,
    
    // File operations
    pub reload_requested: Relay,
    pub remove_requested: Relay,
    pub parse_completed: Relay<Vec<WaveformSignal>>,
}

impl TrackedFile {
    pub fn new(id: String, path: PathBuf) -> Self {
        let (reload_requested, mut reload_stream) = relay();
        let (remove_requested, _) = relay(); // Handled by FileManager
        let (parse_completed, mut parse_stream) = relay();
        
        let path_clone = path.clone();
        let state = Actor::new(FileState::Loading, async move |state_actor| {
            // Initial file parsing
            Self::parse_file(&path_clone, &state_actor).await;
            
            loop {
                select! {
                    Some(()) = reload_stream.next() => {
                        state_actor.set_neq(FileState::Loading);
                        Self::parse_file(&path_clone, &state_actor).await;
                    }
                    Some(signals) = parse_stream.next() => {
                        if let Some(time_range) = Self::calculate_time_range(&signals) {
                            state_actor.set_neq(FileState::Parsed { signals, time_range });
                        }
                    }
                }
            }
        });
        
        TrackedFile {
            id,
            path,
            state,
            reload_requested,
            remove_requested,
            parse_completed,
        }
    }
    
    async fn parse_file(path: &PathBuf, state: &Actor<FileState>) {
        match path.extension().and_then(|s| s.to_str()) {
            Some("vcd") => {
                match parse_vcd_file(path).await {
                    Ok(signals) => {
                        let time_range = Self::calculate_time_range(&signals);
                        state.set_neq(FileState::Parsed { 
                            signals, 
                            time_range: time_range.unwrap_or((0.0, 0.0))
                        });
                    }
                    Err(error) => state.set_neq(FileState::Error(error.to_string())),
                }
            }
            Some("fst") => {
                match parse_fst_file(path).await {
                    Ok(signals) => {
                        let time_range = Self::calculate_time_range(&signals);
                        state.set_neq(FileState::Parsed { 
                            signals, 
                            time_range: time_range.unwrap_or((0.0, 0.0))
                        });
                    }
                    Err(error) => state.set_neq(FileState::Error(error.to_string())),
                }
            }
            _ => {
                state.set_neq(FileState::Error("Unsupported file format".to_string()));
            }
        }
    }
    
    fn calculate_time_range(signals: &[WaveformSignal]) -> Option<(f64, f64)> {
        let mut min_time = f64::MAX;
        let mut max_time = f64::MIN;
        
        for signal in signals {
            for transition in &signal.transitions {
                min_time = min_time.min(transition.time);
                max_time = max_time.max(transition.time);
            }
        }
        
        if min_time <= max_time {
            Some((min_time, max_time))
        } else {
            None
        }
    }
}
```

## FileManager Collection

```rust
#[derive(Clone, Debug)]
pub struct FileManager {
    pub files: ActorVec<TrackedFile>,
    
    // Collection operations
    pub add_file_requested: Relay<PathBuf>,
    pub remove_file_requested: Relay<String>, // file ID
    pub clear_all_requested: Relay,
    pub batch_add_requested: Relay<Vec<PathBuf>>,
}

impl FileManager {
    pub fn new() -> Self {
        let (add_file_requested, mut add_stream) = relay();
        let (remove_file_requested, mut remove_stream) = relay();
        let (clear_all_requested, mut clear_stream) = relay();
        let (batch_add_requested, mut batch_stream) = relay();
        
        let files = ActorVec::new(vec![], async move |files_vec| {
            loop {
                select! {
                    Some(path) = add_stream.next() => {
                        let file_id = generate_file_id(&path);
                        let tracked_file = TrackedFile::new(file_id, path);
                        files_vec.lock_mut().push_cloned(tracked_file);
                    }
                    Some(file_id) = remove_stream.next() => {
                        files_vec.lock_mut().retain(|f| f.id != file_id);
                    }
                    Some(()) = clear_stream.next() => {
                        files_vec.lock_mut().clear();
                    }
                    Some(paths) = batch_stream.next() => {
                        for path in paths {
                            let file_id = generate_file_id(&path);
                            let tracked_file = TrackedFile::new(file_id, path);
                            files_vec.lock_mut().push_cloned(tracked_file);
                        }
                    }
                }
            }
        });
        
        FileManager {
            files,
            add_file_requested,
            remove_file_requested,
            clear_all_requested,
            batch_add_requested,
        }
    }
}

fn generate_file_id(path: &PathBuf) -> String {
    // Simple ID generation - could be more sophisticated
    format!("file_{}", path.file_name().unwrap_or_default().to_string_lossy())
}
```

## Integration with Timeline

```rust
// File manager coordinates with timeline for time range calculation
impl FileManager {
    pub fn get_global_time_range_signal(&self) -> impl Signal<Item = Option<(f64, f64)>> {
        self.files.signal_vec_cloned()
            .map(|files| {
                let mut global_min = f64::MAX;
                let mut global_max = f64::MIN;
                let mut has_valid_range = false;
                
                for file in files.iter() {
                    if let FileState::Parsed { time_range, .. } = file.state.get() {
                        global_min = global_min.min(time_range.0);
                        global_max = global_max.max(time_range.1);
                        has_valid_range = true;
                    }
                }
                
                if has_valid_range {
                    Some((global_min, global_max))
                } else {
                    None
                }
            })
    }
}
```

## Usage in UI Components

```rust
// Replace global TRACKED_FILES with local FileManager
pub fn waveform_viewer() -> impl Element {
    let file_manager = FileManager::new();
    
    Column::new()
        .item(file_header(&file_manager))
        .item(file_list(&file_manager))
        .item(timeline_with_files(&file_manager))
}

fn file_header(file_manager: &FileManager) -> impl Element {
    Row::new()
        .item(
            button()
                .label("Add File")
                .on_press({
                    let add_relay = file_manager.add_file_requested.clone();
                    move || {
                        // Open file dialog and send path
                        if let Some(path) = open_file_dialog() {
                            add_relay.send(path);
                        }
                    }
                })
        )
        .item(
            button()
                .label("Clear All")
                .on_press({
                    let clear_relay = file_manager.clear_all_requested.clone();
                    move || clear_relay.send(())
                })
        )
}

fn file_list(file_manager: &FileManager) -> impl Element {
    Column::new()
        .items_signal_vec(
            file_manager.files.signal_vec_cloned()
                .map(|file| file_item(file))
        )
}

fn file_item(file: TrackedFile) -> impl Element {
    Row::new()
        .item(
            El::new()
                .child_signal(
                    file.state.signal().map(|state| match state {
                        FileState::Loading => Text::new("Loading..."),
                        FileState::Parsed { signals, time_range } => 
                            Text::new(&format!("Parsed: {} signals, {}s-{}s", 
                                signals.len(), time_range.0, time_range.1)),
                        FileState::Error(error) => Text::new(&format!("Error: {}", error)),
                    })
                )
        )
        .item(
            button()
                .label("Reload")
                .on_press({
                    let reload_relay = file.reload_requested.clone();
                    move || reload_relay.send(())
                })
        )
}
```

## Testing Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[async_test]
    async fn test_file_manager_operations() {
        let file_manager = FileManager::new();
        let mut files_stream = file_manager.files.signal_vec_cloned()
            .to_signal_cloned()
            .to_stream();
        
        // Test initial empty state
        assert_eq!(files_stream.next().await.unwrap().len(), 0);
        
        // Test adding file
        file_manager.add_file_requested.send(PathBuf::from("test.vcd"));
        let files = files_stream.next().await.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, PathBuf::from("test.vcd"));
        
        // Test removing file
        let file_id = files[0].id.clone();
        file_manager.remove_file_requested.send(file_id);
        assert_eq!(files_stream.next().await.unwrap().len(), 0);
    }
    
    #[async_test]
    async fn test_tracked_file_state_transitions() {
        let file = TrackedFile::new("test".to_string(), PathBuf::from("test.vcd"));
        let mut state_stream = file.state.signal().to_stream();
        
        // File starts in Loading state
        assert!(matches!(state_stream.next().await.unwrap(), FileState::Loading));
        
        // Trigger reload
        file.reload_requested.send(());
        
        // Should transition back to Loading, then to Parsed/Error based on file content
        // (In real implementation, would mock file parsing)
    }
}
```

This pattern replaces NovyWave's global `TRACKED_FILES` with a clean, testable, and traceable file management system.