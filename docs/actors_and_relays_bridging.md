# Actor+Relay Architecture: Bridging External APIs

This document covers how to integrate Actor+Relay architecture with external APIs and services, particularly global Zoon framework APIs that weren't designed for the Actor+Relay pattern.

## Table of Contents

1. [ConnectionAdapter Pattern](#connectionadapter-pattern)
2. [WebSocket Integration](#websocket-integration)
3. [HTTP Client Bridging](#http-client-bridging)
4. [Timer and Event System Integration](#timer-and-event-system-integration)
5. [File System Access Bridging](#file-system-access-bridging)
6. [Best Practices for API Bridging](#best-practices-for-api-bridging)

## ConnectionAdapter Pattern

The ConnectionAdapter pattern isolates external APIs from Actor+Relay business logic, providing a clean boundary between infrastructure and application code.

### Basic ConnectionAdapter Structure

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
        let connection = self.connection.clone();
        Task::start(async move {
            let up_msg = UpMsg::SendMessage(message);
            if let Err(error) = connection.send_up_msg(up_msg).await {
                zoon::println!("Failed to send message: {:?}", error);
            }
        });
    }
    
    pub fn get_connection_state(&self) -> impl Signal<Item = ConnectionState> {
        // Bridge connection state to reactive signals
        self.connection.state_signal().map(|state| match state {
            zoon::ConnectionState::Connecting => ConnectionState::Connecting,
            zoon::ConnectionState::Open => ConnectionState::Connected,
            zoon::ConnectionState::Closed => ConnectionState::Disconnected,
        })
    }
}

#[derive(Clone, Debug)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Disconnected,
}
```

### Integration with Actor System

```rust
use futures::select;

#[derive(Clone)]
struct ChatService {
    pub messages: ActorVec<Message>,
    pub send_message: Relay<Message>,
    pub connection_state: Actor<ConnectionState>,
}

impl Default for ChatService {
    fn default() -> Self {
        let (send_message, mut send_stream) = relay();
        let (connection_adapter, mut incoming_stream) = ConnectionAdapter::new();
        
        let connection_state = Actor::new(ConnectionState::Connecting, {
            let state_signal = connection_adapter.get_connection_state();
            async move |state| {
                state_signal.for_each(|conn_state| {
                    state.set_neq(conn_state);
                    async {}
                }).await
            }
        });
        
        let messages = ActorVec::new(vec![], {
            let connection_adapter = connection_adapter.clone();
            
            async move |messages_vec| {
                loop {
                    select! {
                        // Handle incoming messages from connection
                        Some(message) = incoming_stream.next() => {
                            messages_vec.lock_mut().push_cloned(message);
                        }
                        
                        // Handle outgoing messages from UI
                        Some(message) = send_stream.next() => {
                            connection_adapter.send_message(message);
                        }
                    }
                }
            }
        });
        
        ChatService { messages, send_message, connection_state }
    }
}
```

## WebSocket Integration

### Raw WebSocket Bridge

```rust
use web_sys::{WebSocket, MessageEvent, Event, CloseEvent};
use wasm_bindgen::{prelude::*, JsCast};

#[derive(Clone)]
pub struct WebSocketAdapter {
    pub messages: Relay<String>,
    pub send: Relay<String>,
    pub connection_state: Actor<ConnectionState>,
}

impl WebSocketAdapter {
    pub fn new(url: &str) -> Self {
        let (messages, _) = relay();
        let (send, mut send_stream) = relay();
        
        let ws = WebSocket::new(url).unwrap();
        let ws_clone = ws.clone();
        
        let connection_state = Actor::new(ConnectionState::Connecting, async move |state| {
            // Set up WebSocket event handlers
            let onopen_callback = Closure::wrap(Box::new(move |_: Event| {
                state.set_neq(ConnectionState::Connected);
            }) as Box<dyn FnMut(Event)>);
            ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
            onopen_callback.forget();
            
            let onclose_callback = Closure::wrap(Box::new(move |_: CloseEvent| {
                state.set_neq(ConnectionState::Disconnected);
            }) as Box<dyn FnMut(CloseEvent)>);
            ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
            onclose_callback.forget();
            
            let onmessage_callback = Closure::wrap(Box::new({
                let messages = messages.clone();
                move |event: MessageEvent| {
                    if let Ok(text) = event.data().dyn_into::<js_sys::JsString>() {
                        messages.send(String::from(text));
                    }
                }
            }) as Box<dyn FnMut(MessageEvent)>);
            ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
            onmessage_callback.forget();
            
            // Handle outgoing messages
            while let Some(message) = send_stream.next().await {
                let _ = ws_clone.send_with_str(&message);
            }
        });
        
        WebSocketAdapter { messages, send, connection_state }
    }
}
```

## HTTP Client Bridging

### Fetch API Integration

```rust
use web_sys::{Request, RequestInit, Response};
use wasm_bindgen_futures::JsFuture;

#[derive(Clone)]
pub struct HttpAdapter {
    base_url: String,
}

impl HttpAdapter {
    pub fn new(base_url: String) -> Self {
        HttpAdapter { base_url }
    }
    
    pub async fn get<T>(&self, path: &str) -> Result<T, HttpError> 
    where 
        T: for<'de> serde::Deserialize<'de>
    {
        let url = format!("{}{}", self.base_url, path);
        
        let mut opts = RequestInit::new();
        opts.method("GET");
        
        let request = Request::new_with_str_and_init(&url, &opts)?;
        
        let window = web_sys::window().unwrap();
        let response_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let response: Response = response_value.dyn_into().unwrap();
        
        let json = JsFuture::from(response.json()?).await?;
        let data: T = serde_wasm_bindgen::from_value(json)?;
        
        Ok(data)
    }
    
    pub async fn post<T, U>(&self, path: &str, data: &T) -> Result<U, HttpError>
    where
        T: serde::Serialize,
        U: for<'de> serde::Deserialize<'de>,
    {
        let url = format!("{}{}", self.base_url, path);
        
        let mut opts = RequestInit::new();
        opts.method("POST");
        opts.body(Some(&JsValue::from_str(&serde_json::to_string(data)?)));
        
        let request = Request::new_with_str_and_init(&url, &opts)?;
        request.headers().set("Content-Type", "application/json")?;
        
        let window = web_sys::window().unwrap();
        let response_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let response: Response = response_value.dyn_into().unwrap();
        
        let json = JsFuture::from(response.json()?).await?;
        let result: U = serde_wasm_bindgen::from_value(json)?;
        
        Ok(result)
    }
}

// Integration with Actor system
#[derive(Clone)]
struct ApiService {
    pub data: Actor<Option<ApiData>>,
    pub loading_state: Actor<LoadingState>,
    pub fetch_data: Relay<String>,
}

impl Default for ApiService {
    fn default() -> Self {
        let (fetch_data, mut fetch_stream) = relay();
        let http_adapter = HttpAdapter::new("https://api.example.com".to_string());
        
        let loading_state = Actor::new(LoadingState::Idle, async move |_state| {
            // Loading state managed by data actor
        });
        
        let data = Actor::new(None, {
            let http_adapter = http_adapter.clone();
            let loading_state = loading_state.clone();
            
            async move |state| {
                while let Some(endpoint) = fetch_stream.next().await {
                    loading_state.set(LoadingState::Loading);
                    
                    match http_adapter.get::<ApiData>(&endpoint).await {
                        Ok(response) => {
                            state.set_neq(Some(response));
                            loading_state.set(LoadingState::Success);
                        }
                        Err(error) => {
                            zoon::println!("API Error: {:?}", error);
                            loading_state.set(LoadingState::Error(error.to_string()));
                        }
                    }
                }
            }
        });
        
        ApiService { data, loading_state, fetch_data }
    }
}

#[derive(Clone, Debug)]
pub enum LoadingState {
    Idle,
    Loading,
    Success,
    Error(String),
}

#[derive(thiserror::Error, Debug)]
pub enum HttpError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("WASM error: {0}")]
    Wasm(#[from] wasm_bindgen::JsValue),
}
```

## Timer and Event System Integration

### Timer-Based Operations

```rust
use zoon::Timer;

#[derive(Clone)]
pub struct TimerService {
    pub ticks: ActorVec<std::time::Instant>,
    pub start_timer: Relay<u32>, // milliseconds
    pub stop_timer: Relay,
}

impl Default for TimerService {
    fn default() -> Self {
        let (start_timer, mut start_stream) = relay();
        let (stop_timer, mut stop_stream) = relay();
        
        let ticks = ActorVec::new(vec![], async move |ticks_vec| {
            let mut current_timer: Option<TaskHandle> = None;
            
            loop {
                select! {
                    Some(interval_ms) = start_stream.next() => {
                        // Stop existing timer
                        current_timer = None;
                        
                        // Start new timer
                        current_timer = Some(Task::start_droppable({
                            let ticks_vec = ticks_vec.clone();
                            async move {
                                loop {
                                    Timer::sleep(interval_ms).await;
                                    ticks_vec.lock_mut().push_cloned(std::time::Instant::now());
                                }
                            }
                        }));
                    }
                    
                    Some(()) = stop_stream.next() => {
                        current_timer = None; // Drops the handle, stopping the timer
                    }
                }
            }
        });
        
        TimerService { ticks, start_timer, stop_timer }
    }
}
```

### Event System Bridge

```rust
use web_sys::{window, Event, EventTarget};
use wasm_bindgen::prelude::*;

#[derive(Clone)]
pub struct GlobalEventAdapter {
    pub window_resize: Relay<(u32, u32)>,
    pub visibility_change: Relay<bool>,
}

impl Default for GlobalEventAdapter {
    fn default() -> Self {
        let (window_resize, _) = relay();
        let (visibility_change, _) = relay();
        
        // Set up global event listeners
        let window = window().unwrap();
        
        // Window resize events
        let resize_callback = Closure::wrap(Box::new({
            let window_resize = window_resize.clone();
            move |_: Event| {
                let window = window().unwrap();
                let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
                let height = window.inner_height().unwrap().as_f64().unwrap() as u32;
                window_resize.send((width, height));
            }
        }) as Box<dyn FnMut(Event)>);
        
        window.add_event_listener_with_callback(
            "resize", 
            resize_callback.as_ref().unchecked_ref()
        ).unwrap();
        resize_callback.forget();
        
        // Visibility change events
        let visibility_callback = Closure::wrap(Box::new({
            let visibility_change = visibility_change.clone();
            move |_: Event| {
                let document = window().unwrap().document().unwrap();
                let hidden = js_sys::Reflect::get(&document, &"hidden".into())
                    .unwrap().as_bool().unwrap_or(false);
                visibility_change.send(!hidden);
            }
        }) as Box<dyn FnMut(Event)>);
        
        window.document().unwrap().add_event_listener_with_callback(
            "visibilitychange",
            visibility_callback.as_ref().unchecked_ref()
        ).unwrap();
        visibility_callback.forget();
        
        GlobalEventAdapter { window_resize, visibility_change }
    }
}
```

## File System Access Bridging

### File Upload/Download Bridge

```rust
use web_sys::{File, FileReader, HtmlInputElement};
use wasm_bindgen_futures::JsFuture;

#[derive(Clone)]
pub struct FileSystemAdapter {
    pub file_selected: Relay<File>,
    pub file_content: Actor<Option<Vec<u8>>>,
    pub download_file: Relay<(String, Vec<u8>)>, // (filename, content)
}

impl Default for FileSystemAdapter {
    fn default() -> Self {
        let (file_selected, mut file_stream) = relay();
        let (download_file, mut download_stream) = relay();
        
        let file_content = Actor::new(None, async move |state| {
            while let Some(file) = file_stream.next().await {
                let file_reader = FileReader::new().unwrap();
                let file_reader_clone = file_reader.clone();
                
                let (sender, receiver) = futures::channel::oneshot::channel();
                let mut sender = Some(sender);
                
                let onload = Closure::wrap(Box::new(move |_: Event| {
                    if let Some(sender) = sender.take() {
                        let result = file_reader_clone.result().unwrap();
                        let array = js_sys::Uint8Array::new(&result);
                        let data = array.to_vec();
                        let _ = sender.send(data);
                    }
                }) as Box<dyn FnMut(Event)>);
                
                file_reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                file_reader.read_as_array_buffer(&file).unwrap();
                onload.forget();
                
                if let Ok(content) = receiver.await {
                    state.set_neq(Some(content));
                }
            }
        });
        
        // Handle file downloads
        Task::start(async move {
            while let Some((filename, content)) = download_stream.next().await {
                let array = js_sys::Uint8Array::from(&content[..]);
                let blob_parts = js_sys::Array::new();
                blob_parts.push(&array);
                
                let blob = web_sys::Blob::new_with_u8_array_sequence(&blob_parts).unwrap();
                let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
                
                let document = window().unwrap().document().unwrap();
                let anchor = document.create_element("a").unwrap()
                    .dyn_into::<web_sys::HtmlAnchorElement>().unwrap();
                    
                anchor.set_href(&url);
                anchor.set_download(&filename);
                anchor.click();
                
                web_sys::Url::revoke_object_url(&url).unwrap();
            }
        });
        
        FileSystemAdapter { file_selected, file_content, download_file }
    }
    
    pub fn create_file_input(&self) -> impl Element {
        let file_selected = self.file_selected.clone();
        
        Input::new()
            .input_type(InputType::File)
            .on_change(move |input_element| {
                let files = input_element.files();
                if let Some(file) = files.and_then(|list| list.get(0)) {
                    file_selected.send(file);
                }
            })
    }
}
```

## Best Practices for API Bridging

### 1. **Isolation Principle**
Always create adapter structs that isolate external APIs from your Actor+Relay business logic:

```rust
// ✅ GOOD: Clean separation
struct ExternalServiceAdapter {
    connection: ExternalService,
}

impl ExternalServiceAdapter {
    pub fn new() -> (Self, impl Stream<Item = ServiceEvent>) {
        // Adapter logic here
    }
}

// ❌ BAD: External API mixed with business logic
struct BusinessLogic {
    external_service: ExternalService, // Direct dependency
    data: ActorVec<Data>,
}
```

### 2. **Error Boundary Pattern**
Handle external API errors at the adapter boundary, not in business logic:

```rust
impl ConnectionAdapter {
    async fn handle_external_error(&self, error: ExternalError) -> Option<InternalEvent> {
        match error {
            ExternalError::NetworkTimeout => {
                // Retry logic here
                Some(InternalEvent::RetryConnection)
            }
            ExternalError::AuthFailure => {
                Some(InternalEvent::RequiresReauth)
            }
            ExternalError::ServiceUnavailable => {
                Some(InternalEvent::ServiceDown)
            }
            _ => None // Don't propagate unknown errors
        }
    }
}
```

### 3. **Resource Management**
Properly manage external resources in adapter destructors:

```rust
impl Drop for WebSocketAdapter {
    fn drop(&mut self) {
        if let Some(ws) = &self.websocket {
            let _ = ws.close();
        }
    }
}
```

### 4. **Type Safety at Boundaries**
Convert external types to internal types at the adapter boundary:

```rust
impl ConnectionAdapter {
    fn convert_external_message(external_msg: ExternalMessage) -> Option<InternalMessage> {
        match external_msg {
            ExternalMessage::Data { payload } => {
                Some(InternalMessage::DataReceived { 
                    data: payload.into_internal_format() 
                })
            }
            ExternalMessage::Unknown(_) => None, // Filter unknown message types
        }
    }
}
```

### 5. **Connection State Management**
Always expose connection state as reactive signals:

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
    Reconnecting,
}

impl ConnectionAdapter {
    pub fn connection_state(&self) -> impl Signal<Item = ConnectionState> {
        self.state.signal()
    }
}
```

## Related Documentation

- **[actors_and_relays.md](actors_and_relays.md)** - Core Actor+Relay concepts
- **[actors_and_relays_patterns.md](actors_and_relays_patterns.md)** - Migration and architecture patterns
- **[actors_and_relays_testing.md](actors_and_relays_testing.md)** - Testing external integrations
- **[chat_example.md](actor_relay_examples/chat_example.md)** - ConnectionAdapter usage example
- **[chat_example_global.md](actor_relay_examples/chat_example_global.md)** - Global bridging patterns