# Actor+Relay Example: Chat Application (Global State Patterns)

> **⚠️ BRIDGE DOCUMENTATION**: This file contains global state patterns for Actor+Relay architecture. These patterns serve as a bridge between traditional MoonZoon globals and idiomatic local state. For production applications, prefer the local state patterns in `chat_example.md`.

This example shows how to implement a WebSocket-based chat application using global Actor+Relay patterns. While functional, this approach is less idiomatic than local state patterns but may serve as a stepping stone during migration.

## Original MoonZoon Chat Problems Reference

For reference on why the original MoonZoon approach with global Mutables was problematic, see the "Original MoonZoon Chat (Snippet)" section in `chat_example.md`.

## Global Actor+Relay Version

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

/// Global chat state with proper encapsulation
#[derive(Clone)]
struct ChatState {
    // State managed by Actors
    messages_actor: ActorVec<Message>,
    username_actor: Actor<Username>,
    message_text_actor: Actor<MessageText>,
    viewport_y_actor: Actor<i32>,
    
    // Events - event-source based naming
    enter_pressed_relay: Relay,
    send_button_clicked_relay: Relay,
    username_input_changed_relay: Relay<Username>,
    message_input_changed_relay: Relay<MessageText>,
    message_sent_relay: Relay,
    
    // External service integration (isolated)
    connection: ConnectionAdapter<UpMsg, DownMsg>,
}

impl Default for ChatState {
    fn default() -> Self {
        // Create all relays with streams
        let (enter_pressed_relay, mut enter_pressed_stream) = relay();
        let (send_button_clicked_relay, mut send_button_clicked_stream) = relay();
        let (username_input_changed_relay, mut username_input_changed_stream) = relay();
        let (message_input_changed_relay, mut message_input_changed_stream) = relay();
        let (message_sent_relay, mut message_sent_stream) = relay();
        
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
                            VecDiff::Push { .. } | VecDiff::Replace { .. } => {
                                state.set(i32::MAX);
                            }
                            _ => {
                                // Don't auto-scroll for other operations
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
        
        ChatState {
            messages_actor,
            username_actor,
            message_text_actor,
            viewport_y_actor,
            enter_pressed_relay,
            send_button_clicked_relay,
            username_input_changed_relay,
            message_input_changed_relay,
            message_sent_relay,
            connection,
        }
    }
}

// Global instance - properly encapsulated
static CHAT: Lazy<ChatState> = lazy::default();

fn main() {
    start_app("app", root);
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
        .item(received_messages())
        .item(new_message_panel())
        .item(username_panel())
}

fn received_messages() -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Scrollbars::both())
        .viewport_y_signal(CHAT.viewport_y_actor.signal())
        .child(
            Column::new()
                .s(Align::new().bottom())
                .items_signal_vec(
                    CHAT.messages_actor.signal_vec_cloned()
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
        .on_change(move |text| { 
            CHAT.message_input_changed_relay.send(MessageText::from(text)); 
        })
        .label_hidden("New message text")
        .placeholder(Placeholder::new("Message"))
        .on_key_down_event(move |event| {
            event.if_key(Key::Enter, move || {
                CHAT.enter_pressed_relay.send(());
            })
        })
        .text_signal(CHAT.message_text_actor.signal())
}

fn send_button() -> impl Element {
    let hovered = Atom::new(false);
    Button::new()
        .s(Padding::all(10))
        .s(RoundedCorners::new().right(5))
        .s(Background::new()
            .color_signal(hovered.value.signal().map_bool(|| color!("Green"), || color!("DarkGreen"))))
        .s(Font::new().color(color!("#EEE")).size(17))
        .on_hovered_change(move |is_hovered| hovered.setter.send(is_hovered))
        .on_press(move || {
            CHAT.send_button_clicked_relay.send(());
        })
        .label("Send")
}

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
        .on_change(move |username| { 
            CHAT.username_input_changed_relay.send(Username::from(username)); 
        })
        .placeholder(Placeholder::new("Joe"))
        .text_signal(CHAT.username_actor.signal())
}
```

## Global ConnectionAdapter Module

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

## Global Atom Helper

```rust
/// Generic helper for simple Actor+Relay state (global variant)
struct Atom<T> {
    pub value: Actor<T>,
    pub setter: Relay<T>,
}

impl<T: Clone> Atom<T> {
    pub fn new(initial: T) -> Self {
        let (setter, mut setter_stream) = relay();
        
        let value = Actor::new(initial, async move |state| {
            while let Some(new_value) = setter_stream.next().await {
                state.set_neq(new_value);
            }
        });
        
        Atom { value, setter }
    }
}
```

## Testing Global Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[async_test]
    async fn test_global_username_updates() {
        // Test global state through signals
        let mut username_stream = CHAT.username_actor.signal().to_stream();
        
        // Test initial value
        assert_eq!(*username_stream.next().await.unwrap(), DEFAULT_USERNAME);
        
        // Test username change through global relay
        CHAT.username_input_changed_relay.send(Username::from("Alice"));
        assert_eq!(*username_stream.next().await.unwrap(), "Alice");
    }
    
    #[async_test]
    async fn test_global_message_send() {
        // Test message sending through global relays
        CHAT.username_input_changed_relay.send(Username::from("TestUser"));
        CHAT.message_input_changed_relay.send(MessageText::from("Hello"));
        CHAT.enter_pressed_relay.send(());
        
        // Test that message text was cleared after send
        let mut text_stream = CHAT.message_text_actor.signal().to_stream();
        assert_eq!(*text_stream.next().await.unwrap(), "");
    }
}
```

## Benefits of Global Approach

### ✅ When Global Patterns Are Appropriate:
- **Singleton state**: When you need exactly one instance across the entire application
- **Migration bridge**: Transitioning from global Mutables to Actor+Relay incrementally  
- **Shared services**: Connection managers, configuration, logging systems
- **Cross-component communication**: When many unrelated components need the same state

### ⚠️ Trade-offs vs Local State:
- **Less testable**: Global state harder to isolate in tests
- **Implicit dependencies**: Components depend on globals without clear interfaces
- **Reduced composability**: Harder to reuse components in different contexts
- **Testing complexity**: Need to manage global state between test runs

## Migration Notes

This global pattern serves as a bridge between traditional MoonZoon globals and idiomatic Actor+Relay local state. For new applications, prefer the local state patterns shown in `chat_example.md`.

The key improvements over raw global Mutables:
- **Encapsulation**: State changes only through defined relays
- **Event traceability**: All mutations go through typed events
- **Controlled access**: No direct mutation of internal state
- **Atomic operations**: Relay events prevent race conditions