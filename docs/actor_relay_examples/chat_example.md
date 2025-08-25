# Actor+Relay Example: Chat Application

This example shows how to transform a WebSocket-based chat application from traditional MoonZoon patterns to the Actor+Relay architecture, demonstrating async Actor patterns and external service integration.

## Original MoonZone Chat

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
```

### Problems with Original Approach:
- **Mixed concerns**: Connection logic mixed with UI state management
- **Global state pollution**: Multiple global statics with unclear relationships
- **No error recovery**: Connection failures not handled gracefully
- **Tight coupling**: UI directly accesses and mutates state
- **No message queuing**: Messages can be lost during connection issues
- **Testing difficulty**: WebSocket dependencies make unit testing hard

## Actor+Relay Version

```rust
use shared::{DownMsg, Message, UpMsg};
use zoon::{eprintln, *};

// Event types for different aspects of chat functionality
#[derive(Clone, Debug)]
struct SendMessage {
    username: String,
    text: String,
}

#[derive(Clone, Debug)]
struct MessageReceived(Message);

#[derive(Clone, Debug)]
struct ConnectionStatusChanged {
    connected: bool,
    error: Option<String>,
}

#[derive(Clone, Debug)]
struct UpdateUsername(String);

#[derive(Clone, Debug)]
struct UpdateMessageText(String);

#[derive(Clone, Debug)]
struct ScrollToBottom;

// Connection state with proper encapsulation
#[derive(Clone, Debug)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// Chat system with proper separation of concerns and event-driven architecture
#[derive(Clone)]
struct ChatSystem {
    // State Actors - each handles one concern
    messages: Actor<Vec<Message>>,
    connection_state: Actor<ConnectionState>,
    username: Actor<String>,
    message_text: Actor<String>,
    viewport_y: Actor<i32>,
    
    // Event Relays - clear interaction points
    user_actions: UserActionRelays,
    connection_events: ConnectionRelays,
    ui_events: UIRelays,
    
    // External service integration
    connection: Connection<UpMsg, DownMsg>,
    connection_task: Mutable<Option<TaskHandle>>,
}

#[derive(Clone)]
struct UserActionRelays {
    send_message: Relay<SendMessage>,
    update_username: Relay<UpdateUsername>,
    update_message_text: Relay<UpdateMessageText>,
}

#[derive(Clone)]
struct ConnectionRelays {
    message_received: Relay<MessageReceived>,
    connection_status_changed: Relay<ConnectionStatusChanged>,
}

#[derive(Clone)]
struct UIRelays {
    scroll_to_bottom: Relay<ScrollToBottom>,
}

impl ChatSystem {
    pub fn new() -> Self {
        // Create all Relays first
        let user_actions = UserActionRelays {
            send_message: Relay::new(),
            update_username: Relay::new(),
            update_message_text: Relay::new(),
        };
        
        let connection_events = ConnectionRelays {
            message_received: Relay::new(),
            connection_status_changed: Relay::new(),
        };
        
        let ui_events = UIRelays {
            scroll_to_bottom: Relay::new(),
        };
        
        // Create Connection with proper event routing
        let connection = Connection::new({
            let message_relay = connection_events.message_received.clone();
            let status_relay = connection_events.connection_status_changed.clone();
            
            move |down_msg, connection_state| {
                match down_msg {
                    DownMsg::MessageReceived(message) => {
                        let _ = message_relay.send(MessageReceived(message));
                    }
                }
                
                // Monitor connection state changes
                let connected = matches!(connection_state, zoon::ConnectionState::Connected);
                let _ = status_relay.send(ConnectionStatusChanged { 
                    connected, 
                    error: None 
                });
            }
        });
        
        // Create State Actors with event processing
        let messages = Actor::new(Vec::new(), {
            let message_relay = connection_events.message_received.clone();
            let scroll_relay = ui_events.scroll_to_bottom.clone();
            
            clone!((message_relay, scroll_relay) async move |state| {
                // Handle incoming messages
                Task::start_droppable(clone!((state, scroll_relay) async move {
                    message_relay.subscribe().for_each(clone!((state, scroll_relay) async move |MessageReceived(message)| {
                        let mut current_messages = state.get();
                        current_messages.push(message);
                        state.set(current_messages);
                        
                        // Trigger UI scroll
                        let _ = scroll_relay.send(ScrollToBottom);
                    })).await;
                }));
            })
        });
        
        let connection_state = Actor::new(ConnectionState::Disconnected, {
            let status_relay = connection_events.connection_status_changed.clone();
            
            clone!((status_relay) async move |state| {
                Task::start_droppable(clone!((state) async move {
                    status_relay.subscribe().for_each(clone!((state) async move |status_change| {
                        let new_state = if status_change.connected {
                            ConnectionState::Connected
                        } else if let Some(error) = status_change.error {
                            ConnectionState::Error(error)
                        } else {
                            ConnectionState::Disconnected
                        };
                        state.set(new_state);
                    })).await;
                }));
            })
        });
        
        let username = Actor::new("John".to_string(), {
            let update_relay = user_actions.update_username.clone();
            
            clone!((update_relay) async move |state| {
                Task::start_droppable(clone!((state) async move {
                    update_relay.subscribe().for_each(clone!((state) async move |UpdateUsername(name)| {
                        state.set(name);
                    })).await;
                }));
            })
        });
        
        let message_text = Actor::new(String::new(), {
            let update_relay = user_actions.update_message_text.clone();
            
            clone!((update_relay) async move |state| {
                Task::start_droppable(clone!((state) async move {
                    update_relay.subscribe().for_each(clone!((state) async move |UpdateMessageText(text)| {
                        state.set(text);
                    })).await;
                }));
            })
        });
        
        let viewport_y = Actor::new(0, {
            let scroll_relay = ui_events.scroll_to_bottom.clone();
            
            clone!((scroll_relay) async move |state| {
                Task::start_droppable(clone!((state) async move {
                    scroll_relay.subscribe().for_each(clone!((state) async move |ScrollToBottom| {
                        state.set(i32::MAX);
                    })).await;
                }));
            })
        });
        
        let chat_system = ChatSystem {
            messages,
            connection_state,
            username,
            message_text,
            viewport_y,
            user_actions,
            connection_events,
            ui_events,
            connection,
            connection_task: Mutable::new(None),
        };
        
        // Set up async message sending with proper error handling
        chat_system.setup_message_sender();
        
        chat_system
    }
    
    fn setup_message_sender(&self) {
        let send_relay = self.user_actions.send_message.clone();
        let connection = self.connection.clone();
        let message_text_actor = self.message_text.clone();
        let status_relay = self.connection_events.connection_status_changed.clone();
        
        let task_handle = Task::start_droppable(clone!((send_relay, connection, message_text_actor, status_relay) async move {
            send_relay.subscribe().for_each(clone!((connection, message_text_actor, status_relay) async move |SendMessage { username, text }| {
                // Send message with proper error handling
                let message = Message { username, text };
                
                match connection.send_up_msg(UpMsg::SendMessage(message)).await {
                    Ok(_) => {
                        // Clear message text on successful send
                        message_text_actor.set(String::new());
                    }
                    Err(error) => {
                        // Report connection error
                        let error_msg = format!("Failed to send message: {:?}", error);
                        eprintln!("{}", error_msg);
                        let _ = status_relay.send(ConnectionStatusChanged {
                            connected: false,
                            error: Some(error_msg),
                        });
                    }
                }
            })).await;
        }));
        
        self.connection_task.set(Some(task_handle));
    }
    
    // Public API - only through events
    pub fn send_message(&self, text: String) -> Result<(), RelayError> {
        let username = self.username.get();
        self.user_actions.send_message.send(SendMessage { username, text })
    }
    
    pub fn update_username(&self, username: String) -> Result<(), RelayError> {
        self.user_actions.update_username.send(UpdateUsername(username))
    }
    
    pub fn update_message_text(&self, text: String) -> Result<(), RelayError> {
        self.user_actions.update_message_text.send(UpdateMessageText(text))
    }
    
    // Reactive state access
    pub fn messages_signal(&self) -> impl Signal<Item = Vec<Message>> {
        self.messages.signal()
    }
    
    pub fn connection_state_signal(&self) -> impl Signal<Item = ConnectionState> {
        self.connection_state.signal()
    }
    
    pub fn username_signal(&self) -> impl Signal<Item = String> {
        self.username.signal()
    }
    
    pub fn message_text_signal(&self) -> impl Signal<Item = String> {
        self.message_text.signal()
    }
    
    pub fn viewport_y_signal(&self) -> impl Signal<Item = i32> {
        self.viewport_y.signal()
    }
    
    // Initialize connection
    pub fn init_connection(&self) {
        self.connection.init_lazy();
    }
}

// Global instance - now properly encapsulated
static CHAT: Lazy<ChatSystem> = Lazy::new(|| ChatSystem::new());

fn main() {
    start_app("app", root);
    CHAT.init_connection();
}

fn root() -> impl Element {
    El::new()
        .s(Padding::new().y(20))
        .s(Height::screen())
        .child(content())
}

fn content() -> impl Element {
    Column::new()
        .s(Width::exact(300))
        .s(Height::fill())
        .s(Align::new().center_x())
        .s(Gap::both(20))
        .item(connection_status())  // New: Connection status display
        .item(received_messages())
        .item(new_message_panel())
        .item(username_panel())
}

// ------ Connection Status ------

fn connection_status() -> impl Element {
    El::new()
        .child_signal(CHAT.connection_state_signal().map(|state| {
            let (text, color) = match state {
                ConnectionState::Connected => ("Connected", color!("Green")),
                ConnectionState::Connecting => ("Connecting...", color!("Orange")), 
                ConnectionState::Disconnected => ("Disconnected", color!("Red")),
                ConnectionState::Error(error) => return El::new()
                    .s(Font::new().color(color!("Red")))
                    .child(Text::new(format!("Error: {}", error))).into_element(),
            };
            
            El::new()
                .s(Font::new().color(color))
                .child(Text::new(text))
                .into_element()
        }))
}

// ------ Received Messages ------

fn received_messages() -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Scrollbars::both())
        .viewport_y_signal(CHAT.viewport_y_signal())
        .child(
            Column::new()
                .s(Align::new().bottom())
                .items_signal_vec(
                    CHAT.messages_signal()
                        .to_signal_vec()
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

// ------ New Message Panel ------

fn new_message_panel() -> impl Element {
    Row::new()
        .item(new_message_input())
        .item(send_button())
}

fn new_message_input() -> impl Element {
    TextInput::new()
        .s(Padding::all(10))
        .s(RoundedCorners::new().left(5))
        .s(Width::fill())
        .s(Font::new().size(17))
        .focus(true)
        .on_change(|text| { CHAT.update_message_text(text); })
        .label_hidden("New message text")
        .placeholder(Placeholder::new("Message"))
        .on_key_down_event(|event| {
            event.if_key(Key::Enter, || {
                let text = CHAT.message_text_signal().sample_cloned();
                CHAT.send_message(text);
            })
        })
        .text_signal(CHAT.message_text_signal())
}

fn send_button() -> impl Element {
    let (hovered, hovered_signal) = Mutable::new_and_signal(false);
    Button::new()
        .s(Padding::all(10))
        .s(RoundedCorners::new().right(5))
        .s(Background::new()
            .color_signal(hovered_signal.map_bool(|| color!("Green"), || color!("DarkGreen"))))
        .s(Font::new().color(color!("#EEE")).size(17))
        .on_hovered_change(move |is_hovered| hovered.set(is_hovered))
        .on_press(|| {
            let text = CHAT.message_text_signal().sample_cloned();
            CHAT.send_message(text);
        })
        .label("Send")
}

// ------ Username Panel ------

fn username_panel() -> impl Element {
    let id = "username_input";
    Row::new()
        .s(Gap::both(15))
        .item(username_input_label(id))
        .item(username_input(id))
}

fn username_input_label(id: &str) -> impl Element {
    Label::new()
        .s(Font::new().color(color!("#EEE")))
        .for_input(id)
        .label("Username:")
}

fn username_input(id: &str) -> impl Element {
    TextInput::new()
        .s(Width::fill())
        .s(Padding::new().x(10).y(6))
        .s(RoundedCorners::all(5))
        .update_raw_el(|raw_el| {
            raw_el.attr("data-1p-ignore", "")
        })
        .id(id)
        .on_change(|username| { CHAT.update_username(username); })
        .placeholder(Placeholder::new("Joe"))
        .text_signal(CHAT.username_signal())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[async_test]
    async fn test_message_flow() {
        let chat = ChatSystem::new();
        
        // Simulate receiving a message
        let test_message = Message {
            username: "TestUser".to_string(),
            text: "Hello World".to_string(),
        };
        
        chat.connection_events.message_received.send(MessageReceived(test_message.clone())).unwrap();
        
        // Wait for actor to process
        Timer::sleep(10).await;
        
        let messages = chat.messages.get();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].username, "TestUser");
        assert_eq!(messages[0].text, "Hello World");
    }
    
    #[async_test]
    async fn test_username_update() {
        let chat = ChatSystem::new();
        
        chat.update_username("NewUser".to_string()).unwrap();
        Timer::sleep(10).await;
        
        assert_eq!(chat.username.get(), "NewUser");
    }
    
    #[async_test] 
    async fn test_connection_state_tracking() {
        let chat = ChatSystem::new();
        
        chat.connection_events.connection_status_changed.send(ConnectionStatusChanged {
            connected: false,
            error: Some("Network error".to_string()),
        }).unwrap();
        
        Timer::sleep(10).await;
        
        match chat.connection_state.get() {
            ConnectionState::Error(error) => assert_eq!(error, "Network error"),
            _ => panic!("Expected error state"),
        }
    }
}
```

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
- User presence tracking (extend ConnectionRelays)
- Multiple chat rooms (extend ChatSystem structure)

### 6. **🛡️ Error Recovery & Resilience**
- Connection failures don't crash the application
- User can see connection status and retry
- Messages can be queued and resent (future enhancement)
- Graceful degradation when backend unavailable

## Advanced Features Made Possible

```rust
// Easy to add features like:

// 1. Message persistence
struct ChatWithPersistence {
    chat: ChatSystem,
    storage: Actor<MessageStorage>,
    save_trigger: Relay<SaveMessages>,
}

// 2. Typing indicators
struct TypingRelays {
    user_typing: Relay<UserTyping>,
    user_stopped_typing: Relay<UserStoppedTyping>,
}

// 3. Multiple chat rooms
struct MultiChatSystem {
    rooms: ActorVec<ChatSystem>,
    current_room: Actor<String>,
    room_management: RoomRelays,
}

// 4. Connection retry with exponential backoff
struct ConnectionManager {
    retry_count: Actor<u32>,
    retry_delay: Actor<Duration>,
    auto_retry: Relay<AttemptReconnect>,
}

// 5. Message reactions and threading
struct ExtendedMessage {
    base: Message,
    reactions: ActorVec<Reaction>,
    thread_id: Option<String>,
}
```

This transformation demonstrates how Actor+Relay patterns handle complex async operations, external service integration, and error recovery while maintaining clean separation of concerns and comprehensive testability.