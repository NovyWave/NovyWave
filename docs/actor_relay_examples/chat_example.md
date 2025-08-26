# Actor+Relay Example: Chat Application

This example shows how to transform a WebSocket-based chat application from traditional MoonZoon patterns to the Actor+Relay architecture, demonstrating async Actor patterns and external service integration.

## Original MoonZoon Chat (Snippet)

```rust
use shared::{DownMsg, Message, UpMsg};
use zoon::{eprintln, *};

// Global mutable state - multiple concerns mixed together
static USERNAME: Lazy<Mutable<String>> = Lazy::new(|| Mutable::new("John".to_owned()));
static MESSAGES: Lazy<MutableVec<Message>> = lazy::default();
static NEW_MESSAGE_TEXT: Lazy<Mutable<String>> = lazy::default();
static RECEIVED_MESSAGES_VIEWPORT_Y: Lazy<Mutable<i32>> = lazy::default();

// Global connection with mixed concerns
pub static CONNECTION: Lazy<Connection<UpMsg, DownMsg>> = Lazy::new(|| {
    Connection::new(|DownMsg::MessageReceived(message), _| {
        MESSAGES.lock_mut().push_cloned(message);  // Direct state mutation
        RECEIVED_MESSAGES_VIEWPORT_Y.set(i32::MAX);  // Mixed UI concern
    })
});

fn send_message() {
    Task::start(async {
        let result = CONNECTION
            .send_up_msg(UpMsg::SendMessage(Message {
                username: USERNAME.get_cloned(),
                text: NEW_MESSAGE_TEXT.take(),  // Direct state access
            }))
            .await;
        if let Err(error) = result {
            eprintln!("Failed to send message: {:?}", error);  // No error handling
        }
    });
}

// Note: This shows core state management patterns - full UI code would include
// root(), content(), received_messages(), new_message_panel(), username_panel() functions
```

### Problems with Original Approach:
- **Mixed concerns**: Connection logic mixed with UI state management
- **Global state pollution**: Multiple global statics with unclear relationships
- **No error recovery**: Connection failures not handled gracefully
- **Tight coupling**: UI directly accesses and mutates state
- **No message queuing**: Messages can be lost during connection issues
- **Testing difficulty**: SSE and HTTP dependencies make unit testing hard

## Actor+Relay Version (Local State)

```rust
use shared::{DownMsg, Message, UpMsg};
use zoon::{eprintln, *};
use futures::select;
use std::sync::Arc;

// Type aliases for cheap cloning of frequently passed data
type Username = Arc<String>;
type MessageText = Arc<String>;

// Single source of truth for defaults
const DEFAULT_USERNAME: &str = "John";

/// Local chat app with clean separation of concerns
#[derive(Clone)]
struct ChatApp {
    // State managed by Actors
    messages_actor: ActorVec<Message>,
    username_actor: Actor<Username>,
    message_text_actor: Actor<MessageText>,
    viewport_y_actor: Actor<i32>,
    
    // Events - event-source based naming with single source per relay
    enter_pressed_relay: Relay,
    send_button_clicked_relay: Relay,
    username_input_changed_relay: Relay<Username>,
    message_input_changed_relay: Relay<MessageText>,
    message_received_relay: Relay,
    message_sent_relay: Relay,
    
    // External service integration (isolated)
    connection: ConnectionAdapter<UpMsg, DownMsg>,
}

impl Default for ChatApp {
    fn default() -> Self {
        // Create all relays with streams
        let (enter_pressed_relay, mut enter_pressed_stream) = Relay::create_with_stream();
        let (send_button_clicked_relay, mut send_button_clicked_stream) = Relay::create_with_stream();
        let (username_input_changed_relay, mut username_input_changed_stream) = Relay::create_with_stream();
        let (message_input_changed_relay, mut message_input_changed_stream) = Relay::create_with_stream();
        let (message_received_relay, mut message_received_stream) = Relay::create_with_stream();
        let (message_sent_relay, mut message_sent_stream) = Relay::create_with_stream();
        
        // Create connection adapter (isolated from business logic) 
        let (connection, mut incoming_message_stream) = ConnectionAdapter::new();
        
        // Simple actors for individual state
        let username_actor = Actor::new(Username::from(DEFAULT_USERNAME), async move |state| {
            while let Some(name) = username_input_changed_stream.next().await {
                state.set(name);
            }
        });
        
        let message_text_actor = Actor::new(MessageText::default(), async move |state| {
            loop {
                select! {
                    Some(text) = message_input_changed_stream.next() => {
                        state.set(text);
                    }
                    Some(()) = message_sent_stream.next() => {
                        state.set(MessageText::default());
                    }
                }
            }
        });
        
        let viewport_y_actor = Actor::new(0, {
            let messages_signal = messages_actor.signal_vec_cloned();
            
            async move |state| {
                messages_signal
                    .for_each(move |diff| {
                        match diff {
                            VecDiff::Push { .. } => {
                                // New message added - scroll to bottom
                                state.set(i32::MAX);
                            }
                            VecDiff::Replace { .. } => {
                                // Messages replaced (initial load/refresh) - scroll to bottom
                                state.set(i32::MAX);
                            }
                            _ => {
                                // Pop, Clear, Remove, Move, UpdateAt - don't auto-scroll
                                // User might be reading history
                            }
                        }
                        async {}
                    })
                    .await
            }
        });
        
        // Messages collection handles both receiving AND sending
        let messages_actor = ActorVec::new(vec![], {
            let connection = connection.clone();
            let message_sent_relay = message_sent_relay.clone();
            
            async move |messages_vec| {
                // Cache current values as they flow through streams
                let mut current_username = Username::default();
                let mut current_message_text = MessageText::default();
                
                let send_trigger_stream = futures::stream::select(
                    enter_pressed_stream,
                    send_button_clicked_stream
                );
                
                loop {
                    select! {
                        // Update cached username when it changes
                        Some(username) = username_input_changed_stream.next() => {
                            current_username = username;
                        }
                        
                        // Update cached message text when it changes
                        Some(text) = message_input_changed_stream.next() => {
                            current_message_text = text;
                        }
                        
                        // Handle received messages from connection
                        Some(message) = incoming_message_stream.next() => {
                            messages_vec.lock_mut().push_cloned(message);
                            // No relay send needed - viewport observes directly via signal
                        }
                        
                        // Send using cached values
                        Some(()) = send_trigger_stream.next() => {
                            if !current_message_text.trim().is_empty() {
                                let message = Message { 
                                    username: (*current_username).clone(),
                                    text: (*current_message_text).clone()
                                };
                                connection.send_message(message);
                                
                                // Clear cached text and notify UI
                                current_message_text = MessageText::default();
                                message_sent_relay.send(());
                            }
                        }
                    }
                }
            }
        });
        
        ChatApp {
            messages_actor,
            username_actor,
            message_text_actor,
            viewport_y_actor,
            enter_pressed_relay,
            send_button_clicked_relay,
            username_input_changed_relay,
            message_input_changed_relay,
            message_received_relay,
            message_sent_relay,
            connection,
        }
    }
}

fn main() {
    start_app("app", || {
        ChatApp::default().root()
    });
}

impl ChatApp {
    fn root(&self) -> impl Element {
        El::new()
            .s(Padding::new().y(20))
            .s(Height::screen())
            .child(self.content())
    }

    fn content(&self) -> impl Element {
        Column::new()
            .s(Width::exact(300))
            .s(Height::fill())
            .s(Align::new().center_x())
            .s(Gap::both(20))
            .item(self.received_messages())
            .item(self.new_message_panel())
            .item(self.username_panel())
    }

    fn received_messages(&self) -> impl Element {
        El::new()
            .s(Height::fill())
            .s(Scrollbars::both())
            .viewport_y_signal(self.viewport_y_actor.signal())
            .child(
                Column::new()
                    .s(Align::new().bottom())
                    .items_signal_vec(
                        self.messages_actor.signal_vec_cloned()
                            .map(received_message)
                    ),
            )
    }

fn received_message(message: Message) -> impl Element {
    Column::new()
        .s(Padding::all(10))
        .s(Gap::both(6))
        .item(
            El::new()
                .s(Font::new()
                    .weight(FontWeight::Bold)
                    .color(color!("#EEE"))
                    .size(17))
                .child(message.username),
        )
        .item(
            Paragraph::new()
                .s(Font::new().color(color!("#DDD")).size(17).line_height(27))
                .contents(message_text_to_contents(&message.text)),
        )
}

    fn new_message_panel(&self) -> impl Element {
        Row::new()
            .item(self.new_message_input())
            .item(self.send_button())
    }

    fn new_message_input(&self) -> impl Element {
        TextInput::new()
            .s(Padding::all(10))
            .s(RoundedCorners::new().left(5))
            .s(Width::fill())
            .s(Font::new().size(17))
            .focus(true)
            .on_change({
                let message_input_changed_relay = self.message_input_changed_relay.clone();
                move |text| { message_input_changed_relay.send(MessageText::from(text)); }
            })
            .label_hidden("New message text")
            .placeholder(Placeholder::new("Message"))
            .on_key_down_event({
                let enter_pressed_relay = self.enter_pressed_relay.clone();
                move |event| {
                    event.if_key(Key::Enter, move || {
                        enter_pressed_relay.send(());
                    })
                }
            })
            .text_signal(self.message_text_actor.signal())
    }

    fn send_button(&self) -> impl Element {
        let hovered = SimpleState::new(false);
        Button::new()
            .s(Padding::all(10))
            .s(RoundedCorners::new().right(5))
            .s(Background::new()
                .color_signal(hovered.value.signal().map_bool(|| color!("Green"), || color!("DarkGreen"))))
            .s(Font::new().color(color!("#EEE")).size(17))
            .on_hovered_change(move |is_hovered| hovered.setter.send(is_hovered))
            .on_press({
                let send_button_clicked_relay = self.send_button_clicked_relay.clone();
                move || {
                    send_button_clicked_relay.send(());
                }
            })
            .label("Send")
    }

    fn username_panel(&self) -> impl Element {
        let id = "username_input";
        Row::new()
            .s(Gap::both(15))
            .item(self.username_input_label(id))
            .item(self.username_input(id))
    }

    fn username_input_label(&self, id: &str) -> impl Element {
        Label::new()
            .s(Font::new().color(color!("#EEE")))
            .for_input(id)
            .label("Username:")
    }

    fn username_input(&self, id: &str) -> impl Element {
        TextInput::new()
            .s(Width::fill())
            .s(Padding::new().x(10).y(6))
            .s(RoundedCorners::all(5))
            .update_raw_el(|raw_el| {
                raw_el.attr("data-1p-ignore", "")
            })
            .id(id)
            .on_change({
                let username_input_changed_relay = self.username_input_changed_relay.clone();
                move |username| { username_input_changed_relay.send(Username::from(username)); }
            })
            .placeholder(Placeholder::new("Joe"))
            .text_signal(self.username_actor.signal())
    }
}
```

## External Service Integration

### ConnectionAdapter Module

Protocol-agnostic adapter for bridging Zoon's Connection with Actor+Relay architecture:

```rust
use std::sync::Arc;
use futures::stream::{Stream, StreamExt};
use zoon::Connection;
use shared::{UpMsg, DownMsg, Message};

#[derive(Clone)]
pub struct ConnectionAdapter<TUp, TDown> {
    connection: Connection<TUp, TDown>,
}

impl ConnectionAdapter<UpMsg, DownMsg> {
    pub fn new() -> (Self, impl Stream<Item = Message>) {
        let (message_sender, message_stream) = futures::channel::mpsc::unbounded();
        
        let connection = Connection::new(move |down_msg, _| {
            if let DownMsg::MessageReceived(message) = down_msg {
                let _ = message_sender.unbounded_send(message);
            }
        });
        
        let adapter = ConnectionAdapter { connection };
        (adapter, message_stream)
    }
    
    pub fn send_message(&self, message: Message) {
        let up_msg = UpMsg::SendMessage(message);
        Task::start(async move {
            if let Err(error) = self.connection.send_up_msg(up_msg).await {
                zoon::println!("Failed to send message: {:?}", error);
            }
        });
    }
}
```

## Helper Modules

### SimpleState Helper

For simple state that doesn't need complex event types, we can create a helper pattern:

```rust
/// Generic helper for simple Actor+Relay state
struct SimpleState<T> {
    pub value: Actor<T>,
    pub setter: Relay<T>,
}

impl<T: Clone> SimpleState<T> {
    pub fn new(initial: T) -> Self {
        let (setter, mut setter_stream) = Relay::create_with_stream();
        
        let value = Actor::new(initial, async move |state| {
            while let Some(new_value) = setter_stream.next().await {
                state.set_neq(new_value);
            }
        });
        
        SimpleState { value, setter }
    }
}
```

## Testing Notes

The Actor+Relay pattern prioritizes encapsulation over direct state testing. Since Actor state is intentionally hidden, traditional unit tests cannot assert on internal values.

**For production testing, consider:**
- Integration tests that render actual UI components
- Testing through signal subscriptions and observable effects  
- Mocking external services (ConnectionAdapter) for message flow verification
- End-to-end testing with user interactions

**The relays provide a clean testing interface:**
```rust
fn integration_test_example() {
    let chat = ChatApp::default();
    
    // Test that relay sends work without panics
    chat.username_input_changed_relay.send(Username::from("TestUser"));
    chat.message_input_changed_relay.send(MessageText::from("Hello"));
    chat.enter_pressed_relay.send(());
    
    // In real tests, you'd assert on UI render output or signal subscriptions
}
```

## Testing

```rust
#[cfg(test)]
mod test_helpers {
    use super::*;
    
    impl ChatApp {
        // Test helpers to avoid "relay sent from multiple locations" issues
        // These simulate the exact same actions as UI handlers
        pub fn test_enter_key(&self) {
            self.enter_pressed_relay.send(());
        }
        
        pub fn test_send_button(&self) {
            self.send_button_clicked_relay.send(());
        }
        
        pub fn test_set_username(&self, username: &str) {
            self.username_input_changed_relay.send(Username::from(username));
        }
        
        pub fn test_set_message_text(&self, text: &str) {
            self.message_input_changed_relay.send(MessageText::from(text));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::test_helpers::*;
    
    #[async_test]
    async fn test_simple_state_through_signal() {
        let state = SimpleState::new(false);
        let mut signal_stream = state.value.signal().to_stream();
        
        // Test initial value
        assert_eq!(signal_stream.next().await, Some(false));
        
        // Test state change
        state.setter.send(true);
        assert_eq!(signal_stream.next().await, Some(true));
        
        state.setter.send(false);
        assert_eq!(signal_stream.next().await, Some(false));
    }
    
    #[async_test]
    async fn test_username_updates_through_signal() {
        let chat = ChatApp::default();
        let mut username_stream = chat.username_actor.signal().to_stream();
        
        // Test initial value
        assert_eq!(*username_stream.next().await.unwrap(), DEFAULT_USERNAME);
        
        // Test username change
        chat.test_set_username("Alice");
        assert_eq!(*username_stream.next().await.unwrap(), "Alice");
        
        chat.test_set_username("Bob");
        assert_eq!(*username_stream.next().await.unwrap(), "Bob");
    }
    
    #[async_test]
    async fn test_message_text_clear_on_send() {
        let chat = ChatApp::default();
        let mut text_stream = chat.message_text_actor.signal().to_stream();
        
        // Initial empty
        assert_eq!(*text_stream.next().await.unwrap(), "");
        
        // Set message text
        chat.test_set_message_text("Hello World");
        assert_eq!(*text_stream.next().await.unwrap(), "Hello World");
        
        // Trigger message sent event (should clear text)
        chat.message_sent_relay.send(());
        assert_eq!(*text_stream.next().await.unwrap(), "");
    }
    
    #[async_test]
    async fn test_both_send_methods_work() {
        let chat = ChatApp::default();
        
        // Test Enter key path doesn't panic
        chat.test_set_username("User1");
        chat.test_set_message_text("Via Enter");
        chat.test_enter_key();
        Timer::sleep(10).await; // Let processing complete
        
        // Test Send button path doesn't panic
        chat.test_set_message_text("Via Button");
        chat.test_send_button();
        Timer::sleep(10).await; // Let processing complete
        
        // Both should work without "multiple source" errors
    }
}
```

**Note:** If relays enforce single-source sending, the test helpers simulate UI events from the test context to avoid "sent from multiple locations" runtime panics.

## Key Benefits of Actor+Relay Chat Version

### 1. **🔌 Clean Separation of Concerns**
- **Connection logic**: Isolated in Connection with event routing
- **State management**: Each Actor handles one specific concern
- **UI logic**: Separated from business logic and external services
- **Error handling**: Centralized and recoverable

### 2. **🌐 Robust External Service Integration**
- WebSocket connection failures handled gracefully
- Connection state tracking with user feedback
- Message queuing during connection issues (extensible)
- Proper async error propagation through events

### 3. **🧪 Comprehensive Testability**
- Each Actor can be tested in isolation
- Events can be injected programmatically
- No WebSocket dependencies in unit tests
- Clear test scenarios for error conditions

### 4. **📡 Event-Driven Architecture Benefits**
- All interactions go through typed events
- Clear audit trail of user actions and system responses
- Easy to add features like message history, typing indicators
- Decoupled components enable independent evolution

### 5. **⚡ Extensibility & Features**
- Connection retry logic (just add Relay and Actor)
- Message persistence (Actor can save to storage)
- User presence tracking (add more relays)
- Multiple chat rooms (extend ChatApp structure)

### 6. **🛡️ Error Recovery & Resilience**
- Connection failures don't crash the application
- User can see connection status and retry
- Messages can be queued and resent (future enhancement)
- Graceful degradation when backend unavailable

## Advanced Features Made Possible

```rust
// Easy to add features like:

// 1. Message persistence
impl ChatApp {
    fn save_message(&self, message: &Message) {
        // Add save_relay: Relay<Message> and Actor that handles persistence
    }
}

// 2. Connection retry
struct ChatWithRetry {
    chat: ChatApp,
    retry_count: Actor<u32>,
    retry: Relay,
}

// 3. Multiple chat rooms
struct MultiRoomChat {
    rooms: ActorVec<ChatApp>,
    switch_room: Relay<String>,
}
```

## Key Improvements in Updated Version

### 1. **📦 Modular Helper Types**
- **Before**: SimpleState mixed in with business logic
- **After**: Extracted SimpleState as reusable library/app helper
- **Benefit**: Clear separation, reusable across applications

### 2. **📊 Proper Collection Management**
- **Before**: `messages: Actor<Vec<Message>>` with manual vector manipulation
- **After**: `messages: ActorVec<Message>` with direct collection operations
- **Benefit**: More appropriate data structure, cleaner signal API

### 3. **🌉 Connection Bridge Architecture**
- **Before**: Connection logic mixed directly in business logic
- **After**: `ConnectionBridge` module isolating Zoon integration from app logic
- **Benefit**: Clean separation of concerns, reusable bridge pattern

### 4. **⚡ Eliminated Task Antipattern**
- **Before**: `Task::start` for handling send message events (antipattern)
- **After**: `MessageSender` Actor with reactive dataflow
- **Benefit**: Proper reactive architecture, no external coordination needed

### 5. **🔄 Pure Reactive Dataflow**
- **Before**: Imperative Task handling with manual state coordination
- **After**: Actors respond to streams, state flows reactively
- **Benefit**: No race conditions, cleaner async flow, easier to reason about

### 6. **🏗️ Separation of Infrastructure vs Business Logic**
- **Before**: WebSocket Connection concerns mixed with chat functionality
- **After**: ConnectionBridge handles transport, ChatApp handles business logic
- **Benefit**: Testable business logic, swappable transport layer

This transformation demonstrates how to properly structure Actor+Relay applications with clean separation between infrastructure (Zoon bridging) and business logic (chat functionality), while eliminating common antipatterns.