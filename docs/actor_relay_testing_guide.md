# Actor+Relay Testing & Debugging Guide

Comprehensive guide for testing and debugging Actor+Relay systems in NovyWave, extracted from the complete Actor+Relay architecture documentation.

## Table of Contents

1. [Testing Philosophy](#testing-philosophy)
2. [Core Testing Utilities](#core-testing-utilities)
3. [Essential Testing Patterns](#essential-testing-patterns)
4. [Signal-Based Testing](#signal-based-testing)
5. [Performance Testing](#performance-testing)
6. [Debugging Tools](#debugging-tools)
7. [Connection Tracking](#connection-tracking)
8. [Integration Testing](#integration-testing)
9. [Mocking Strategies](#mocking-strategies)
10. [Test Organization](#test-organization)
11. [Anti-Patterns to Avoid](#anti-patterns-to-avoid)

## Testing Philosophy

The Actor+Relay architecture enables **signal-based reactive testing** that eliminates timing dependencies and race conditions common in traditional testing approaches.

### Core Principles

- **Signal-based assertions**: Use `actor.signal().to_stream().next().await` instead of direct state access
- **No arbitrary timeouts**: Wait for actual signal emissions, not Timer::sleep()
- **Reactive waiting**: Natural batching with signal stream testing
- **Deterministic tests**: Signal streams provide predictable test execution
- **Isolation**: Each Actor can be tested independently

### The Reactive Testing Model

**Testing Timeline:**
```
1. relay.send() → Returns immediately (synchronous)
2. Actor receives event → Happens on next tick (asynchronous)
3. state.update() → Happens inside Actor
4. signal().next().await → Waits for signal from step 3
5. Test continues with correct value
```

**Benefits:**
- **No arbitrary timeouts**: Wait exactly as long as needed
- **More reliable**: No race conditions from insufficient wait times
- **Consistent with architecture**: Uses reactive patterns throughout
- **Faster tests**: No unnecessary delays

## Core Testing Utilities

### MockRelay

Mock relay implementation for testing external dependencies:

```rust
/// Mock relay for testing
pub struct MockRelay<T> {
    events: Arc<Mutex<Vec<T>>>,
    auto_respond: Option<Box<dyn Fn(&T) -> Option<T>>>,
}

impl<T: Clone> MockRelay<T> {
    pub fn new() -> Self;
    
    /// Set auto-response function for testing
    pub fn with_auto_response<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> Option<T> + 'static;
    
    /// Get all emitted events
    pub fn events(&self) -> Vec<T>;
    
    /// Clear recorded events
    pub fn clear_events(&self);
    
    /// Simulate emission
    pub fn simulate_emit(&self, event: T);
}
```

**Example Usage:**

```rust
#[async_test]
async fn test_file_service_with_mock() {
    let mock_relay = MockRelay::new()
        .with_auto_response(|path: &String| {
            if path.ends_with(".txt") {
                Some(format!("Content of {}", path))
            } else {
                None
            }
        });
    
    let file_service = FileService::new(mock_relay.clone());
    
    file_service.load_file.send("test.txt".to_string());
    
    let result = file_service.content.signal().to_stream().next().await.unwrap();
    assert_eq!(result, Some("Content of test.txt".to_string()));
    
    // Verify interaction
    let events = mock_relay.events();
    assert_eq!(events, vec!["test.txt".to_string()]);
}
```

### ActorTestHarness

Test harness for comprehensive Actor lifecycle management:

```rust
/// Test harness for actors
pub struct ActorTestHarness {
    // Track all actors created during test
    actors: Vec<Box<dyn Any>>,
}

impl ActorTestHarness {
    pub fn new() -> Self;
    
    /// Run test with automatic cleanup
    pub async fn run_test<F, Fut>(&mut self, test: F)
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = ()>;
    
    /// Assert no pending messages
    pub fn assert_quiet(&self);
    
    /// Wait for actor processing
    pub async fn wait_for_processing(&self, timeout_ms: u64);
}
```

**Example Usage:**

```rust
#[async_test]
async fn test_complex_actor_interaction() {
    let mut harness = ActorTestHarness::new();
    
    harness.run_test(|| async {
        let counter = Counter::default();
        
        // Rapid events
        for i in 1..=5 {
            counter.change_by.send(i);
        }
        
        // Wait for processing
        harness.wait_for_processing(1000).await;
        
        let result = counter.value.signal().to_stream().next().await.unwrap();
        assert_eq!(result, 15);
        
        // Ensure no pending messages
        harness.assert_quiet();
    }).await;
}
```

## Essential Testing Patterns

All examples consistently use **Signal-based reactive testing**:

### Basic Counter Test

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[async_test]
    async fn test_counter_increment() {
        let counter = Counter::default();
        
        // Send event through relay
        counter.change_by.send(3);
        
        // ✅ CORRECT: Use signal.await to get value reactively
        let final_value = counter.value.signal().to_stream().next().await.unwrap();
        assert_eq!(final_value, 3);
    }
}
```

### Multiple Operations Test

```rust
#[async_test]  
async fn test_multiple_operations() {
    let counter = Counter::default();
    
    // Multiple operations
    counter.change_by.send(5);
    counter.change_by.send(-2);
    counter.change_by.send(1);
    
    // ✅ CORRECT: Await signal - no Timer::sleep needed!
    let final_value = counter.value.signal().to_stream().next().await.unwrap();
    assert_eq!(final_value, 4);  // 0 + 5 - 2 + 1 = 4
}
```

### Safe Arithmetic Test

```rust
#[async_test]
async fn test_safe_arithmetic() {
    let control = GridDimensionControl::default();  // Starts at 5
    
    // Test saturating subtraction
    for _ in 0..10 {
        control.decrement.send(());
    }
    
    // ✅ CORRECT: Use signal reactively - no .get() needed
    let final_count = control.count.signal().to_stream().next().await.unwrap();
    assert_eq!(final_count, 1);  // Should never go below 1
}
```

### Sequential Processing Test

```rust
#[async_test]
async fn test_actor_processes_events_sequentially() {
    let (relay, mut stream) = relay();
    let results = Arc::new(Mutex::new(Vec::new()));
    
    let actor = Actor::new(0, {
        let results = results.clone();
        async move |state| {
            while let Some(value) = stream.next().await {
                // Simulate processing
                Timer::sleep(10).await;
                state.update(|current| current + value);
                
                // Record result using signal-based access
                let current_value = state.signal().to_stream().next().await.unwrap();
                results.lock().unwrap().push(current_value);
            }
        }
    });
    
    // Emit events rapidly
    for i in 1..=5 {
        relay.send(i);
    }
    
    // Wait for all processing using signal-based approach
    let mut value_stream = actor.signal().to_stream();
    let mut final_value = 0;
    for _ in 1..=5 {
        final_value = value_stream.next().await.unwrap();
    }
    
    // Verify final result after all processing
    assert_eq!(final_value, 15); // 1+2+3+4+5 = 15
    let results = results.lock().unwrap().clone();
    assert_eq!(results, vec![1, 3, 6, 10, 15]);
}
```

### Relay Error Handling Test

```rust
#[async_test]
async fn test_relay_drops_events_without_subscribers() {
    let relay = Relay::<String>::new();
    
    // No subscribers
    assert!(!relay.has_subscribers());
    
    // With no subscribers, send() will auto-log the error
    relay.send("test".to_string());  // Error logged to console automatically
    
    // For explicit error checking, use try_send()
    let result = relay.try_send("test".to_string());
    assert!(matches!(result, Err(RelayError::NoSubscribers)));
}
```

### Actor Lifecycle Test

```rust
#[async_test]
async fn test_actor_lifecycle() {
    let (relay, mut stream) = relay();
    let actor = Actor::new(42, async move |_state| {
        // Actor task - wait for events
        while let Some(_) = stream.next().await {
            // Process events
        }
    });
    
    // Test initial value using signal
    let mut value_stream = actor.signal().to_stream();
    assert_eq!(value_stream.next().await.unwrap(), 42);
    
    // Test actor can receive events
    relay.send(());
    Timer::sleep(10).await;
    
    // In real implementation, would have stop() method and is_running() check
    // This shows signal-based testing approach instead of direct .get() access
}
```

## Signal-Based Testing

### Core Pattern Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // ✅ Correct reactive pattern - no .get() or Timer::sleep()
    #[async_test]
    async fn test_pattern_name() {
        // 1. Create instance using Default
        let app = CounterApp::default();
        
        // 2. Send events through relays
        app.increment.send(());
        
        // 3. Await signal reactively - no arbitrary timeouts
        let result = app.value.signal().to_stream().next().await.unwrap();
        
        // 4. Assert final state
        assert_eq!(result, expected_value);
    }
}
```

### ActorVec Testing

```rust
#[async_test]
async fn test_actor_vec_operations() {
    let todo_list = TodoList::default();
    
    // Add items
    todo_list.add_item.send("Task 1".to_string());
    todo_list.add_item.send("Task 2".to_string());
    
    // Test collection state
    let items = todo_list.items
        .signal_vec_cloned()
        .to_signal_cloned()
        .to_stream()
        .next()
        .await
        .unwrap();
    
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].title, "Task 1");
    assert_eq!(items[1].title, "Task 2");
}
```

### State Changes Over Time

```rust
#[async_test]
async fn test_state_changes_over_time() {
    let counter = Counter::default();
    
    // Create stream for multiple assertions
    let mut signal_stream = counter.value.signal().to_stream();
    
    counter.increment.send(());
    assert_eq!(signal_stream.next().await.unwrap(), 1);
    
    counter.increment.send(()); 
    assert_eq!(signal_stream.next().await.unwrap(), 2);
    
    counter.decrement.send(());
    assert_eq!(signal_stream.next().await.unwrap(), 1);
}
```

## Performance Testing

### Reactive Waiting - No Arbitrary Timeouts

```rust
// ✅ GOOD: Wait exactly as long as needed
let result = counter.value.signal().to_stream().next().await.unwrap();

// ✅ GOOD: Natural batching with signal waiting
counter.increment.send(());
counter.increment.send(());  
counter.decrement.send(());
let result = counter.value.signal().to_stream().next().await.unwrap();  // Waits for final result
```

### Batch Operations Testing

```rust
#[async_test]
async fn test_batch_operations() {
    let file_manager = FileManager::default();
    
    // Send multiple operations rapidly
    let files = vec!["file1.txt", "file2.txt", "file3.txt"];
    for file in files {
        file_manager.add_file.send(file.to_string());
    }
    
    // Single await for all operations to complete
    let final_files = file_manager.files
        .signal_vec_cloned()
        .to_signal_cloned()
        .to_stream()
        .next()
        .await
        .unwrap();
    
    assert_eq!(final_files.len(), 3);
}
```

## Debugging Tools

### ActorDebugger

Comprehensive debugging utility for development:

```rust
#[cfg(debug_assertions)]
pub struct ActorDebugger {
    /// Enable/disable debug output
    pub fn set_enabled(enabled: bool);
    
    /// Set filter for specific actors
    pub fn set_filter(pattern: &str);
    
    /// Get mutation history for all actors
    pub fn get_global_history() -> Vec<(String, Vec<MutationEvent>)>;
    
    /// Clear all debug data
    pub fn clear_debug_data();
}

// Debug macros
#[cfg(debug_assertions)]
macro_rules! actor_trace {
    ($actor:expr, $msg:expr) => {
        if ActorDebugger::is_enabled() {
            zoon::println!("[{}] {}", $actor.name(), $msg);
        }
    };
}
```

**Usage Example:**

```rust
#[cfg(debug_assertions)]
fn debug_file_operations() {
    ActorDebugger::set_enabled(true);
    ActorDebugger::set_filter("file_*");
    
    let file_manager = FileManager::default();
    file_manager.add_file.send("test.txt".to_string());
    
    // Check mutation history
    let history = ActorDebugger::get_global_history();
    for (actor_name, mutations) in history {
        zoon::println!("Actor {}: {:?}", actor_name, mutations);
    }
}
```

### Debug Tracing in Tests

```rust
#[async_test]
async fn test_with_debug_tracing() {
    #[cfg(debug_assertions)]
    ActorDebugger::set_enabled(true);
    
    let counter = Counter::default();
    
    actor_trace!(counter, "Starting test");
    
    counter.increment.send(());
    let result = counter.value.signal().to_stream().next().await.unwrap();
    
    actor_trace!(counter, format!("Final value: {}", result));
    
    assert_eq!(result, 1);
    
    #[cfg(debug_assertions)]
    ActorDebugger::clear_debug_data();
}
```

## Connection Tracking

For debugging data flow and understanding system interactions:

```rust
#[cfg(debug_assertions)]
pub struct ConnectionTracker {
    connections: Arc<Mutex<Vec<Connection>>>,
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub source: String,      // e.g., "button#add-file"
    pub relay: String,        // e.g., "add_file_relay"
    pub actor: String,        // e.g., "tracked_files_actor"
    pub timestamp: f64,
}

impl ConnectionTracker {
    /// Record a UI -> Relay connection
    pub fn track_emission(source: &str, relay: &str);
    
    /// Record a Relay -> Actor connection
    pub fn track_subscription(relay: &str, actor: &str);
    
    /// Get all connections for a relay
    pub fn connections_for_relay(relay: &str) -> Vec<Connection>;
    
    /// Visualize connection graph
    pub fn print_graph();
}

// Usage in debug builds
#[cfg(debug_assertions)]
ConnectionTracker::track_emission("file_list_item#123", "remove_file_relay");
```

### Testing Connection Tracking

```rust
#[cfg(test)]
#[async_test]
async fn test_connection_tracking() {
    #[cfg(debug_assertions)]
    {
        let file_manager = FileManager::default();
        
        ConnectionTracker::track_emission("test_source", "add_file_relay");
        ConnectionTracker::track_subscription("add_file_relay", "file_manager_actor");
        
        file_manager.add_file.send("test.txt".to_string());
        
        let connections = ConnectionTracker::connections_for_relay("add_file_relay");
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].source, "test_source");
        
        ConnectionTracker::print_graph();
    }
}
```

## Integration Testing

### Multi-Actor System Testing

```rust
#[async_test]
async fn test_file_and_variable_integration() {
    let file_manager = FileManager::default();
    let variable_manager = VariableManager::default();
    
    // Connect systems (in real app, this would be in initialization)
    let file_loaded_stream = file_manager.file_loaded.stream();
    Task::start(async move {
        while let Some(file_info) = file_loaded_stream.next().await {
            variable_manager.extract_variables.send(file_info);
        }
    });
    
    // Test integration
    file_manager.add_file.send("test.vcd".to_string());
    
    // Wait for file loading
    let loaded_files = file_manager.files
        .signal_vec_cloned()
        .to_signal_cloned()
        .to_stream()
        .next()
        .await
        .unwrap();
    
    assert_eq!(loaded_files.len(), 1);
    
    // Wait for variable extraction
    let extracted_variables = variable_manager.variables
        .signal_vec_cloned()
        .to_signal_cloned()
        .to_stream()
        .next()
        .await
        .unwrap();
    
    assert!(!extracted_variables.is_empty());
}
```

### Service Integration Testing

```rust
#[async_test]
async fn test_external_service_integration() {
    let mock_file_service = MockRelay::new()
        .with_auto_response(|path: &String| {
            if path.ends_with(".vcd") {
                Some(FileContent {
                    path: path.clone(),
                    data: vec![0, 1, 2, 3],
                    signals: vec!["clock".to_string(), "data".to_string()],
                })
            } else {
                None
            }
        });
    
    let file_manager = FileManager::with_service(mock_file_service.clone());
    
    file_manager.load_file.send("test.vcd".to_string());
    
    // Verify service interaction
    let service_calls = mock_file_service.events();
    assert_eq!(service_calls, vec!["test.vcd".to_string()]);
    
    // Verify file manager state
    let files = file_manager.loaded_files
        .signal_vec_cloned()
        .to_signal_cloned()
        .to_stream()
        .next()
        .await
        .unwrap();
    
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].signals.len(), 2);
}
```

## Mocking Strategies

### External Service Mocking

```rust
// Define trait for external dependency
pub trait FileService {
    fn load_file(&self, path: &str) -> Result<FileContent, FileError>;
    fn save_file(&self, path: &str, content: &[u8]) -> Result<(), FileError>;
}

// Production implementation
pub struct RealFileService;

impl FileService for RealFileService {
    fn load_file(&self, path: &str) -> Result<FileContent, FileError> {
        // Real file loading logic
    }
    
    fn save_file(&self, path: &str, content: &[u8]) -> Result<(), FileError> {
        // Real file saving logic
    }
}

// Mock implementation
pub struct MockFileService {
    files: HashMap<String, Vec<u8>>,
    call_log: Arc<Mutex<Vec<String>>>,
}

impl MockFileService {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            call_log: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn with_file(mut self, path: &str, content: Vec<u8>) -> Self {
        self.files.insert(path.to_string(), content);
        self
    }
    
    pub fn calls(&self) -> Vec<String> {
        self.call_log.lock().unwrap().clone()
    }
}

impl FileService for MockFileService {
    fn load_file(&self, path: &str) -> Result<FileContent, FileError> {
        self.call_log.lock().unwrap().push(format!("load_file({})", path));
        
        if let Some(content) = self.files.get(path) {
            Ok(FileContent {
                path: path.to_string(),
                data: content.clone(),
                signals: vec![], // Mock signals
            })
        } else {
            Err(FileError::NotFound)
        }
    }
    
    fn save_file(&self, path: &str, content: &[u8]) -> Result<(), FileError> {
        self.call_log.lock().unwrap().push(format!("save_file({}, {} bytes)", path, content.len()));
        Ok(())
    }
}
```

### Actor with Dependency Injection

```rust
pub struct FileManager<S: FileService> {
    pub load_file: Relay<String>,
    pub save_file: Relay<(String, Vec<u8>)>,
    pub files: ActorVec<LoadedFile>,
    service: S,
}

impl<S: FileService> FileManager<S> {
    pub fn new(service: S) -> Self {
        let (load_relay, mut load_stream) = relay();
        let (save_relay, mut save_stream) = relay();
        let files = ActorVec::new();
        
        let service_clone = service.clone();
        let files_clone = files.clone();
        Task::start(async move {
            while let Some(path) = load_stream.next().await {
                match service_clone.load_file(&path) {
                    Ok(content) => {
                        files_clone.push(LoadedFile::from(content));
                    }
                    Err(e) => {
                        // Handle error
                    }
                }
            }
        });
        
        // Handle save stream...
        
        Self {
            load_file: load_relay,
            save_file: save_relay,
            files,
            service,
        }
    }
}

// Test with mock
#[async_test]
async fn test_file_manager_with_mock() {
    let mock_service = MockFileService::new()
        .with_file("test.vcd", vec![1, 2, 3, 4]);
    
    let file_manager = FileManager::new(mock_service.clone());
    
    file_manager.load_file.send("test.vcd".to_string());
    
    let files = file_manager.files
        .signal_vec_cloned()
        .to_signal_cloned()
        .to_stream()
        .next()
        .await
        .unwrap();
    
    assert_eq!(files.len(), 1);
    assert_eq!(mock_service.calls(), vec!["load_file(test.vcd)"]);
}
```

## Test Organization

### Proper Test Structure

- **Separate tests into dedicated sections** for documentation clarity
- **Use consistent initialization** (`Default::default()`)
- **Always use signal-based testing** with `.signal().to_stream().next().await`
- **Test both positive and negative cases** (increment/decrement)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    mod counter_tests {
        use super::*;
        
        #[async_test]
        async fn test_increment() {
            let counter = Counter::default();
            counter.increment.send(());
            let result = counter.value.signal().to_stream().next().await.unwrap();
            assert_eq!(result, 1);
        }
        
        #[async_test]
        async fn test_decrement() {
            let counter = Counter::default();
            counter.decrement.send(());
            let result = counter.value.signal().to_stream().next().await.unwrap();
            assert_eq!(result, -1);  // Or 0 if using saturating arithmetic
        }
        
        #[async_test]
        async fn test_multiple_operations() {
            // Test complex scenarios
        }
    }
    
    mod edge_cases {
        use super::*;
        
        #[async_test]
        async fn test_saturating_arithmetic() {
            // Test boundary conditions
        }
        
        #[async_test]
        async fn test_error_handling() {
            // Test error scenarios
        }
    }
}
```

### Test Module Organization

```rust
// In actor_tests.rs
#[cfg(test)]
mod actor_core_tests {
    // Core Actor functionality
}

#[cfg(test)]
mod relay_core_tests {
    // Core Relay functionality
}

#[cfg(test)]
mod integration_tests {
    // Multi-component testing
}

#[cfg(test)]
mod performance_tests {
    // Performance and stress testing
}
```

## Anti-Patterns to Avoid

### ❌ Direct State Access in Tests

```rust
// ❌ WRONG: Testing code that assumes .get() exists
assert_eq!(counter.value.get(), 3); // .get() is NOT in Actor API

// ✅ CORRECT: Signal-based testing
let result = counter.value.signal().to_stream().next().await.unwrap();
assert_eq!(result, 3);
```

### ❌ Timer-Based Testing

```rust
// ❌ WRONG: Arbitrary timeouts
counter.increment.send(());
Timer::sleep(100).await; // Race condition potential
let result = counter.value.get(); // Also wrong for other reasons

// ✅ CORRECT: Reactive waiting
counter.increment.send(());
let result = counter.value.signal().to_stream().next().await.unwrap();
```

### ❌ Complex Test Setup

```rust
// ❌ WRONG: Over-complicated setup
#[async_test]
async fn test_counter() {
    let mut setup = TestSetup::new();
    let config = TestConfig::builder()
        .with_initial_value(0)
        .with_max_value(100)
        .build();
    let counter = setup.create_counter(config);
    // ... complex initialization
}

// ✅ CORRECT: Simple, direct testing
#[async_test]
async fn test_counter() {
    let counter = Counter::default();
    counter.increment.send(());
    let result = counter.value.signal().to_stream().next().await.unwrap();
    assert_eq!(result, 1);
}
```

### ❌ Multiple Assertions Without Signal Streams

```rust
// ❌ WRONG: Multiple assertions without proper signal handling
counter.increment.send(());
assert_eq!(counter.value.signal().to_stream().next().await.unwrap(), 1);
counter.increment.send(());
assert_eq!(counter.value.signal().to_stream().next().await.unwrap(), 2);
// Creates new streams each time, may miss intermediate states

// ✅ CORRECT: Single stream for multiple assertions
let mut signal_stream = counter.value.signal().to_stream();
counter.increment.send(());
assert_eq!(signal_stream.next().await.unwrap(), 1);
counter.increment.send(()); 
assert_eq!(signal_stream.next().await.unwrap(), 2);
```

### ❌ Testing Implementation Details

```rust
// ❌ WRONG: Testing internal structure
assert_eq!(actor.internal_queue.len(), 0);

// ✅ CORRECT: Testing behavior
let result = actor.process_event(event);
assert_eq!(result, expected_outcome);
```

## Best Practices Summary

### **Testing Approach**
- **Signal-based testing**: Use `actor.signal().to_stream().next().await` for assertions
- **ActorVec testing**: Use `actor_vec.signal_vec_cloned().to_signal_cloned().to_stream().next().await` for vector assertions
- **Single source location**: Relay can only be sent from ONE location - use Task::start loops or batch relays for multiple sends
- **No timing dependencies**: Wait for actual signal changes, not arbitrary timeouts
- **Reactive waiting**: Natural batching with signal stream testing

### **Test Organization**
- Use `#[cfg(test)]` modules for organization
- Group related tests into sub-modules
- Use consistent naming patterns
- Document complex test scenarios

### **Debugging Strategy**
- Use ActorDebugger for development debugging
- Implement ConnectionTracker for data flow analysis
- Add trace macros for specific investigations
- Test with mocks before integration testing

### **Performance Testing**
- Use signal-based waiting for deterministic performance tests
- Test batch operations with single awaits
- Avoid Timer::sleep() for performance measurements
- Use signal streams for precise timing

This comprehensive testing guide provides all the tools and patterns needed to effectively test Actor+Relay systems, ensuring reliable, maintainable, and performant code.