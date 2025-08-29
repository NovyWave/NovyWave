//! Core dataflow primitives for reactive state management
//! 
//! This module provides the foundational Actor+Relay architecture
//! components that are independent of business logic. These primitives
//! form the basis for all reactive state management in NovyWave.
//!
//! # Core Components
//!
//! - **[`Relay`]** - Type-safe event streaming using simple channels
//! - **[`Actor`]** - Single-value reactive state container
//! - **[`ActorVec`]** - Reactive collection container  
//! - **[`ActorMap`]** - Reactive key-value map container
//! - **[`Atom`]** - Convenient wrapper for local UI state
//!
//! # Architecture Principles
//!
//! 1. **No Raw Mutables** - All state uses Actor+Relay or Atom
//! 2. **Event-Source Naming** - Relays follow `{source}_{event}_relay` pattern
//! 3. **No Direct Access** - No `.get()` methods, all access through signals
//! 4. **Cache Values Only in Actors** - Value caching only inside Actor loops

pub mod relay;
pub mod actor;
pub mod actor_vec;
pub mod actor_map;
pub mod atom;

// Core exports
pub use relay::{Relay, RelayError, relay};
pub use actor::Actor;
pub use actor_vec::ActorVec;
pub use actor_map::ActorMap;
pub use atom::Atom;

// Re-export futures types commonly used with dataflow
pub use futures::stream::Stream;
pub use futures::StreamExt;

// Re-export futures::select! macro for Actor processing loops
// This is essential for the Actor+Relay pattern
pub use futures::select;