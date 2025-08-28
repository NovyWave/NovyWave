# Actor+Relay Implementation Examples

This document provides practical implementation examples for the Actor+Relay architecture, showing how to build common application patterns with clean, testable code.

## Table of Contents

1. [Basic Patterns](#basic-patterns)
2. [Counter Examples](#counter-examples)  
3. [Todo App Example](#todo-app-example)
4. [Resource Manager Example](#resource-manager-example)
5. [Advanced Multi-Stream Processing](#advanced-multi-stream-processing)
6. [UI Component Examples](#ui-component-examples)
7. [Testing Patterns](#testing-patterns)
8. [Common Antipatterns](#common-antipatterns)

## Basic Patterns

### Simple Counter with Unit Relays

The most basic Actor+Relay pattern using discrete button-style events:

```rust
use std::collections::HashMap;
use futures::select;

#[derive(Clone, Default)]
struct SimpleCounter {
    pub value: Actor<i32>,
    
    // Unit relays for discrete actions
    pub increment: Relay,      // Just trigger, no data
    pub decrement: Relay,
    pub reset: Relay,
}

impl SimpleCounter {
    pub fn new() -> Self {
        // Create relays using modern relay() pattern
        let increment = Relay::new();
        let decrement = Relay::new();
        let (reset, mut reset_stream) = relay();
        
        let value = Actor::new(0, async move |state| {
            let mut increment_stream = increment.subscribe();
            let mut decrement_stream = decrement.subscribe();
            
            loop {
                select! {
                    Some(()) = increment_stream.next() => {
                        state.update(|current| current + 1);
                    }
                    Some(()) = decrement_stream.next() => {
                        state.update(|current| current - 1);
                    }
                    Some(()) = reset_stream.next() => {
                        state.set(0);
                    }
                }
            }
        });
        
        SimpleCounter { value, increment, decrement, reset }
    }
}
```

### Parametric Counter with Data Relays

More flexible pattern where events carry data:

```rust
#[derive(Clone)]
struct ParametricCounter {
    pub value: Actor<i32>,
    pub change_by: Relay<i32>,    // Event carries the amount to change
    pub set_to: Relay<i32>,       // Event carries the target value
}

impl ParametricCounter {
    pub fn new(initial: i32) -> Self {
        let (change_by, mut change_stream) = relay();
        let (set_to, mut set_stream) = relay();
        
        let value = Actor::new(initial, async move |state| {
            loop {
                select! {
                    Some(delta) = change_stream.next() => {
                        state.update(|current| current + delta);
                    }
                    Some(target) = set_stream.next() => {
                        state.set(target);
                    }
                }
            }
        });
        
        ParametricCounter { value, change_by, set_to }
    }
}
```

## Counter Examples

### Basic Counter

```rust
#[derive(Clone)]
pub struct Counter {
    pub value: Actor<i32>,
    pub increment: Relay,
    pub decrement: Relay,
}

impl Default for Counter {
    fn default() -> Self {
        let increment = Relay::new();
        let decrement = Relay::new();
        
        let value = Actor::new(0, {
            let increment = increment.clone();
            let decrement = decrement.clone();
            
            async move |state| {
                let mut increment_stream = increment.subscribe();
                let mut decrement_stream = decrement.subscribe();
                
                loop {
                    select! {
                        Some(()) = increment_stream.next() => {
                            state.update(|n| n + 1);
                        }
                        Some(()) = decrement_stream.next() => {
                            state.update(|n| n - 1);
                        }
                    }
                }
            }
        });
        
        Counter { value, increment, decrement }
    }
}
```

## Todo App Example

### Structural Pattern with Typed Messages

```rust
#[derive(Clone, Debug)]
struct TodoItem {
    id: String,
    text: String,
    completed: bool,
}

#[derive(Clone, Debug)]
enum TodoMessage {
    Add { text: String },
    Toggle { id: String },
    Remove { id: String },
    ClearCompleted,
}

#[derive(Clone)]
struct TodoApp {
    pub todos: ActorVec<TodoItem>,
    pub messages: Relay<TodoMessage>,
}

impl TodoApp {
    pub fn new() -> Self {
        let (messages, mut message_stream) = relay();
        
        let todos = ActorVec::new(vec![], async move |todos_vec| {
            while let Some(message) = message_stream.next().await {
                match message {
                    TodoMessage::Add { text } => {
                        let todo = TodoItem {
                            id: generate_id(),
                            text,
                            completed: false,
                        };
                        todos_vec.lock_mut().push_cloned(todo);
                    }
                    TodoMessage::Toggle { id } => {
                        let mut todos = todos_vec.lock_mut();
                        if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
                            todo.completed = !todo.completed;
                        }
                    }
                    TodoMessage::Remove { id } => {
                        todos_vec.lock_mut().retain(|todo| todo.id != id);
                    }
                    TodoMessage::ClearCompleted => {
                        todos_vec.lock_mut().retain(|todo| !todo.completed);
                    }
                }
            }
        });
        
        TodoApp { todos, messages }
    }
    
    // Convenience methods
    pub fn add_todo(&self, text: String) {
        self.messages.send(TodoMessage::Add { text });
    }
    
    pub fn toggle_todo(&self, id: String) {
        self.messages.send(TodoMessage::Toggle { id });
    }
    
    // Derived signals
    pub fn completed_count_signal(&self) -> impl Signal<Item = usize> {
        self.todos.signal_vec_cloned()
            .map(|todos| todos.iter().filter(|t| t.completed).count())
    }
    
    pub fn remaining_count_signal(&self) -> impl Signal<Item = usize> {
        self.todos.signal_vec_cloned()
            .map(|todos| todos.iter().filter(|t| !t.completed).count())
    }
}

fn generate_id() -> String {
    // Simple ID generation - could use uuid crate in real app
    format!("todo_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis())
}
```

## Resource Manager Example

### Clean Architecture with Resource States

This example shows a generic resource management pattern that can be adapted for files, network resources, database connections, etc.

```rust
use std::path::PathBuf;

#[derive(Clone, Debug)]
enum ResourceState {
    Loading,
    Ready { data: String },
    Error(String),
}

#[derive(Clone, Debug)]
struct Resource {
    id: String,
    identifier: String,  // URL, file path, etc.
    state: Actor<ResourceState>,
    
    // Resource operations
    reload_requested: Relay,
    remove_requested: Relay,
}

impl Resource {
    pub fn new(id: String, identifier: String) -> Self {
        let (reload_requested, mut reload_stream) = relay();
        let (remove_requested, _) = relay(); // Handled by ResourceManager
        
        let identifier_clone = identifier.clone();
        let state = Actor::new(ResourceState::Loading, async move |state_actor| {
            // Initial resource loading
            Self::load_resource(&identifier_clone, &state_actor).await;
            
            // Handle reload events
            while let Some(()) = reload_stream.next().await {
                state_actor.set_neq(ResourceState::Loading);
                Self::load_resource(&identifier_clone, &state_actor).await;
            }
        });
        
        Resource {
            id,
            identifier,
            state,
            reload_requested,
            remove_requested,
        }
    }
    
    async fn load_resource(identifier: &str, state: &Actor<ResourceState>) {
        // Simulate async resource loading
        match load_data_from_identifier(identifier).await {
            Ok(data) => state.set_neq(ResourceState::Ready { data }),
            Err(error) => state.set_neq(ResourceState::Error(error.to_string())),
        }
    }
}

async fn load_data_from_identifier(identifier: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Simulate loading - in real app would handle files, HTTP, etc.
    if identifier.starts_with("http") {
        // HTTP request
        Ok(format!("HTTP data from {}", identifier))
    } else if identifier.ends_with(".txt") {
        // File reading
        Ok(format!("File data from {}", identifier))
    } else {
        Err("Unsupported resource type".into())
    }
}

// Resource collection manager
#[derive(Clone, Debug)]
struct ResourceManager {
    resources: ActorVec<Resource>,
    
    // Collection-level events
    add_resource: Relay<String>, // identifier
    remove_resource: Relay<String>, // resource ID
    clear_all: Relay,
}

impl ResourceManager {
    pub fn new() -> Self {
        let (add_resource, mut add_stream) = relay();
        let (remove_resource, mut remove_stream) = relay();
        let (clear_all, mut clear_stream) = relay();
        
        let resources = ActorVec::new(vec![], async move |resources_vec| {
            loop {
                select! {
                    Some(identifier) = add_stream.next() => {
                        let resource_id = generate_resource_id(&identifier);
                        let resource = Resource::new(resource_id, identifier);
                        resources_vec.lock_mut().push_cloned(resource);
                    }
                    Some(resource_id) = remove_stream.next() => {
                        resources_vec.lock_mut().retain(|r| r.id != resource_id);
                    }
                    Some(()) = clear_stream.next() => {
                        resources_vec.lock_mut().clear();
                    }
                }
            }
        });
        
        ResourceManager {
            resources,
            add_resource,
            remove_resource,
            clear_all,
        }
    }
}

fn generate_resource_id(identifier: &str) -> String {
    format!("resource_{}", identifier.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>())
}
```

## Advanced Multi-Stream Processing

### Chat Application with Multiple Event Sources

```rust
#[derive(Clone, Debug)]
struct Message {
    username: String,
    text: String,
    timestamp: u64,
}

#[derive(Clone, Debug)]
struct ChatApp {
    messages: ActorVec<Message>,
    
    // Multiple input streams
    message_sent: Relay<(String, String)>, // (username, text)
    message_received: Relay<Message>,       // from network
    clear_history: Relay,
}

impl ChatApp {
    pub fn new() -> Self {
        let (message_sent, mut sent_stream) = relay();
        let (message_received, mut received_stream) = relay();
        let (clear_history, mut clear_stream) = relay();
        
        let messages = ActorVec::new(vec![], async move |messages_vec| {
            loop {
                select! {
                    Some((username, text)) = sent_stream.next() => {
                        let message = Message {
                            username,
                            text,
                            timestamp: current_timestamp(),
                        };
                        messages_vec.lock_mut().push_cloned(message);
                    }
                    Some(message) = received_stream.next() => {
                        messages_vec.lock_mut().push_cloned(message);
                    }
                    Some(()) = clear_stream.next() => {
                        messages_vec.lock_mut().clear();
                    }
                }
            }
        });
        
        ChatApp {
            messages,
            message_sent,
            message_received,
            clear_history,
        }
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
```

## UI Component Examples

### Interactive Button with Hover State

```rust
fn interactive_button(label: &str, on_click: impl Fn() + 'static) -> impl Element {
    let hover_state = Atom::new(false);
    let pressed_state = Atom::new(false);
    
    button()
        .label(label)
        .on_hover_start({
            let hover = hover_state.clone();
            move || hover.set(true)
        })
        .on_hover_end({
            let hover = hover_state.clone();
            move || hover.set(false)
        })
        .on_press_start({
            let pressed = pressed_state.clone();
            move || pressed.set(true)
        })
        .on_press_end({
            let pressed = pressed_state.clone();
            move || {
                pressed.set(false);
                on_click();
            }
        })
        .update_raw_el(|raw_el| {
            raw_el
                .style_signal("background-color", 
                    map_ref! {
                        let hovered = hover_state.value.signal(),
                        let pressed = pressed_state.value.signal() =>
                        match (pressed, hovered) {
                            (true, _) => "#0066cc",
                            (false, true) => "#0080ff", 
                            (false, false) => "#007bff",
                        }
                    }
                )
        })
}
```

### Form with Validation

```rust
#[derive(Clone, Debug)]
struct FormData {
    email: String,
    password: String,
    is_valid: bool,
}

struct LoginForm {
    form_data: Actor<FormData>,
    email_changed: Relay<String>,
    password_changed: Relay<String>,
    submit: Relay,
}

impl LoginForm {
    pub fn new() -> Self {
        let (email_changed, mut email_stream) = relay();
        let (password_changed, mut password_stream) = relay();
        let (submit, mut submit_stream) = relay();
        
        let form_data = Actor::new(FormData {
            email: String::new(),
            password: String::new(),
            is_valid: false,
        }, async move |state| {
            loop {
                select! {
                    Some(email) = email_stream.next() => {
                        state.update(|data| {
                            data.email = email;
                            data.is_valid = is_valid_email(&data.email) && data.password.len() >= 6;
                        });
                    }
                    Some(password) = password_stream.next() => {
                        state.update(|data| {
                            data.password = password;
                            data.is_valid = is_valid_email(&data.email) && data.password.len() >= 6;
                        });
                    }
                    Some(()) = submit_stream.next() => {
                        let current_data = state.get_cloned();
                        if current_data.is_valid {
                            // Handle form submission
                            handle_login(current_data.email, current_data.password);
                        }
                    }
                }
            }
        });
        
        LoginForm {
            form_data,
            email_changed,
            password_changed,
            submit,
        }
    }
}

fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}

fn handle_login(email: String, password: String) {
    // Login logic here
    println!("Logging in: {}", email);
}
```

## Testing Patterns

### Unit Testing Individual Actors

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[async_test]
    async fn test_counter_operations() {
        let counter = Counter::default();
        let mut value_stream = counter.value.signal().to_stream();
        
        // Test initial value
        assert_eq!(value_stream.next().await, Some(0));
        
        // Test increment
        counter.increment.send(());
        assert_eq!(value_stream.next().await, Some(1));
        
        // Test decrement
        counter.decrement.send(());
        assert_eq!(value_stream.next().await, Some(0));
    }
    
    #[async_test]
    async fn test_todo_app() {
        let app = TodoApp::new();
        let mut todos_stream = app.todos.signal_vec_cloned().to_signal_cloned().to_stream();
        
        // Test initial empty state
        assert_eq!(todos_stream.next().await.unwrap().len(), 0);
        
        // Test adding todo
        app.add_todo("Learn Actor+Relay".to_string());
        let todos = todos_stream.next().await.unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].text, "Learn Actor+Relay");
        assert_eq!(todos[0].completed, false);
        
        // Test toggling todo
        let todo_id = todos[0].id.clone();
        app.toggle_todo(todo_id);
        let todos = todos_stream.next().await.unwrap();
        assert_eq!(todos[0].completed, true);
    }
}
```

## Common Antipatterns

### ❌ Using Raw Mutables

```rust
// WRONG: Defeats the entire purpose of Actor+Relay
static GLOBAL_COUNT: Lazy<Mutable<i32>> = Lazy::new(|| Mutable::new(0));

fn increment_count() {
    GLOBAL_COUNT.set(GLOBAL_COUNT.get() + 1); // Race condition!
}
```

### ❌ Complex Business Logic in Actor Initialization

```rust
// WRONG: Hard to test, complex initialization
let complex_actor = Actor::new(State::default(), async move |state| {
    loop {
        select! {
            Some(event) = stream_a.next() => {
                // 50 lines of business logic here...
                // Makes testing and debugging difficult
            }
        }
    }
});
```

### ❌ Using .get() Methods on Actors

```rust
// WRONG: Creates race conditions
let current_value = actor.get(); // ❌ This method doesn't exist for good reason!
let new_value = current_value + 1;
actor.send(new_value); // Race condition - value may have changed
```

### ✅ Correct Patterns

```rust
// CORRECT: Extract business logic to testable functions
async fn handle_complex_event(state: &Actor<State>, event: ComplexEvent) -> Result<(), Error> {
    // Business logic here - can be unit tested
    let new_state = process_event(event)?;
    state.set_neq(new_state);
    Ok(())
}

// CORRECT: Use atomic operations through relays
let update_relay: Relay<fn(i32) -> i32> = Relay::new();
update_relay.send(|current| current + 1); // Atomic update function
```

These examples demonstrate the flexibility and power of Actor+Relay architecture for building maintainable, testable MoonZoon applications.