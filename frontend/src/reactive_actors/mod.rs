//! Actor+Relay Architecture Module
//! 
//! Provides reactive state management using Actor+Relay patterns for NovyWave.
//! This module implements the core Actor+Relay architecture to replace global
//! Mutables with domain-driven, event-source based state management.
//!
//! ## Core Principles
//! 
//! 1. **NO RAW MUTABLES** - All state uses Actor+Relay or SimpleState
//! 2. **Event-Source Relay Naming** - `{source}_{event}_relay` pattern only
//! 3. **Domain-Driven Design** - Model what things ARE, not what they "manage"
//! 4. **Signal-Based Testing** - No .get() methods, reactive testing only
//!
//! ## Architecture Components
//!
//! - **Relay<T>** - Type-safe event streaming with source constraints
//! - **Actor<T>** - Single-value reactive state container  
//! - **ActorVec<T>** - Reactive collection container
//! - **ActorBTreeMap<K,V>** - Reactive ordered map container
//! - **SimpleState<T>** - Local UI state wrapper (uses Actor+Relay internally)
//!
//! ## Usage Examples
//!
//! ```rust
//! use crate::reactive_actors::{Actor, ActorVec, Relay, SimpleState, relay};
//!
//! // Domain-driven state structure
//! struct TrackedFiles {
//!     files: ActorVec<TrackedFile>,
//!     // Event-source relay naming
//!     file_dropped_relay: Relay<Vec<PathBuf>>,     // User dropped files
//!     parse_completed_relay: Relay<ParseResult>,   // Parser finished
//! }
//!
//! // Local UI state
//! let dialog_open = SimpleState::new(false);
//! let filter_text = SimpleState::new(String::new());
//! ```

pub mod relay;
pub mod actor;
pub mod actor_vec; 
pub mod actor_btree_map;
pub mod simple_state;

// Core exports for easy importing
pub use relay::Relay;
pub use actor::Actor;
pub use actor_vec::ActorVec;
pub use actor_btree_map::ActorBTreeMap;
pub use simple_state::SimpleState;

// Re-export futures types commonly used with Actors
pub use futures::stream::Stream;
pub use futures::select;

/// Creates a new Relay with an associated stream using Rust channel patterns.
/// 
/// This is the idiomatic way to create event relays for use with Actors.
/// 
/// # Examples
/// 
/// ```rust
/// let (file_dropped_relay, file_dropped_stream) = relay();
/// let (parse_completed_relay, parse_completed_stream) = relay();
/// 
/// let files = ActorVec::new(vec![], async move |files_vec| {
///     loop {
///         select! {
///             Some(paths) = file_dropped_stream.next() => {
///                 // Handle file drop events
///             }
///             Some(result) = parse_completed_stream.next() => {
///                 // Handle parse completion events  
///             }
///         }
///     }
/// });
/// ```
pub fn relay<T>() -> (Relay<T>, impl Stream<Item = T>)
where 
    T: Clone + Send + Sync + 'static 
{
    let relay = Relay::new();
    let stream = relay.subscribe();
    (relay, stream)
}