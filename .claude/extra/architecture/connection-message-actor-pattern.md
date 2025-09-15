# ConnectionMessageActor Pattern for Global Static Elimination

## Overview

The ConnectionMessageActor pattern successfully replaces global static message routing with proper Actor+Relay architecture. This pattern transforms a raw message stream into typed relay subscriptions, eliminating the need for global MESSAGE_ROUTER and CONFIG_STORE patterns.

## Core Pattern

### Problem: Global Static Message Routing
```rust
// ❌ ANTIPATTERN: Global static routing
static MESSAGE_ROUTER: OnceLock<MessageRouter> = OnceLock::new();
static CONFIG_STORE: Lazy<Mutable<AppConfig>> = Lazy::new(|| Mutable::new(AppConfig::default()));

// Manual routing everywhere
MESSAGE_ROUTER.get().route_message(down_msg);
let config = CONFIG_STORE.get();
```

### Solution: ConnectionMessageActor with Typed Relays
```rust
// ✅ CORRECT: Centralized message dispatcher with typed relays
#[derive(Clone)]
pub struct ConnectionMessageActor {
    // Message-specific relays that domains can subscribe to
    pub config_loaded_relay: Relay<shared::AppConfig>,
    pub directory_contents_relay: Relay<(String, Vec<shared::FileSystemItem>)>,
    pub directory_error_relay: Relay<(String, String)>,
    pub file_loaded_relay: Relay<(String, shared::FileState)>,
    pub parsing_started_relay: Relay<(String, String)>,

    // Actor handles message processing
    _message_actor: Actor<()>,
}
```

## Implementation Steps

### Step 1: Create Message Dispatcher Actor
```rust
impl ConnectionMessageActor {
    pub async fn new(mut down_msg_stream: impl futures::stream::Stream<Item = DownMsg> + Unpin) -> Self {
        // Create all message-specific relays
        let (config_loaded_relay, _) = relay();
        let (directory_contents_relay, _) = relay();
        let (directory_error_relay, _) = relay();
        let (file_loaded_relay, _) = relay();
        let (parsing_started_relay, _) = relay();

        // Actor processes DownMsg stream and routes to appropriate relays
        let message_actor = Actor::new((), async move |_state| {
            loop {
                if let Some(down_msg) = down_msg_stream.next().await {
                    // Route each message type to its specific relay
                    match down_msg {
                        DownMsg::ConfigLoaded(config) => {
                            config_loaded_relay.send(config);
                        }
                        DownMsg::DirectoryContents { path, items } => {
                            directory_contents_relay.send((path, items));
                        }
                        DownMsg::DirectoryError { path, error } => {
                            directory_error_relay.send((path, error));
                        }
                        // ... other message types
                    }
                }
            }
        });

        Self {
            config_loaded_relay,
            directory_contents_relay,
            directory_error_relay,
            file_loaded_relay,
            parsing_started_relay,
            _message_actor: message_actor,
        }
    }
}
```

### Step 2: Domain Subscription Pattern
```rust
// Domains subscribe to relevant relays instead of polling globals
impl AppConfig {
    pub async fn new(connection_message_actor: ConnectionMessageActor) -> Self {
        let config_loaded_actor = {
            let config_loaded_stream = connection_message_actor.config_loaded_relay.subscribe();
            let theme_relay = theme_changed_relay.clone();
            let dock_relay = dock_mode_changed_relay.clone();

            Actor::new((), async move |_state| {
                let mut config_stream = config_loaded_stream;
                while let Some(loaded_config) = config_stream.next().await {
                    // Reactive config updates
                    theme_relay.send(loaded_config.ui.theme);
                    dock_relay.send(loaded_config.workspace.dock_mode);
                }
            })
        };

        // Store actor to keep it alive
        Self { _config_loaded_actor: config_loaded_actor, /* ... */ }
    }
}
```

### Step 3: Connection Integration Bridge
```rust
// Bridge existing Connection to new Actor stream processing
async fn create_connection_with_message_actor() -> (Connection<UpMsg, DownMsg>, ConnectionMessageActor) {
    use futures::channel::mpsc;

    let (down_msg_sender, down_msg_receiver) = mpsc::unbounded::<DownMsg>();

    // Create ConnectionMessageActor with the message stream
    let connection_message_actor = ConnectionMessageActor::new(down_msg_receiver).await;

    // Create connection that sends to the stream
    let connection = Connection::new(move |down_msg, _| {
        // Send all messages to ConnectionMessageActor for domain routing
        if let Err(_) = down_msg_sender.unbounded_send(down_msg) {
            zoon::println!("❌ Failed to send message to ConnectionMessageActor");
        }
    });

    (connection, connection_message_actor)
}
```

## Key Benefits

### 1. Eliminates Global State
- **Before**: Global statics accessible from anywhere
- **After**: Typed relay subscriptions with clear ownership

### 2. Type Safety
- **Before**: Generic routing with runtime message type checking
- **After**: Compile-time type safety with dedicated relays per message type

### 3. Clear Dependencies
- **Before**: Hidden global dependencies throughout codebase
- **After**: Explicit relay subscriptions showing what each domain needs

### 4. Reactive Architecture Compliance
- **Before**: Imperative polling of global state
- **After**: Event-driven updates through Actor+Relay patterns

## Migration Strategy

1. **Identify Message Types**: List all global routing cases (ConfigLoaded, DirectoryContents, etc.)

2. **Create Typed Relays**: Replace generic routing with message-specific relays

3. **Stream Processing Actor**: Single actor reads raw messages and routes to typed relays

4. **Domain Subscription**: Domains subscribe to relays instead of global access

5. **Integration Bridge**: Connect existing systems (Connection) to new architecture

## Success Metrics

- Zero compilation errors with only warnings remaining
- All domain parameters passed correctly through call chains
- No global static access remaining in codebase
- Proper Actor+Relay signal patterns throughout
- Functionality preserved (TreeView expansion, config loading, etc.)

## Real-World Results

This pattern was successfully applied to eliminate MESSAGE_ROUTER and CONFIG_STORE globals in NovyWave, achieving:
- ✅ Complete global static elimination
- ✅ TreeView expansion working perfectly from config restoration
- ✅ All message routing functioning without globals
- ✅ Type-safe domain communication through ConnectionMessageActor

The ConnectionMessageActor acts as a "message router Actor" providing the same functionality as global routing but within proper Actor+Relay architectural constraints.