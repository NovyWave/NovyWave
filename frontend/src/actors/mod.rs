//! Business domain actors using Actor+Relay architecture
//! 
//! This module contains domain-specific state management built on top of
//! the dataflow primitives. It implements business logic for NovyWave's
//! waveform viewer functionality.
//!
//! ## Architecture
//!
//! The actors module uses the core dataflow primitives (Actor, Relay, etc.)
//! to implement domain-specific state management:
//!
//! - **TrackedFiles** - Manages loaded waveform files
//! - **SelectedVariables** - Manages variable selection
//! - **WaveformTimeline** - Timeline and cursor state
//! - **UserConfiguration** - User preferences and settings
//!
//! ## Core Principles
//! 
//! 1. **Domain-Driven Design** - Model what things ARE, not what they "manage"
//! 2. **Event-Source Relay Naming** - `{source}_{event}_relay` pattern only
//! 3. **No Raw Mutables** - All state uses Actor+Relay or Atom from dataflow
//! 4. **Signal-Based Access** - No .get() methods, reactive access only
//!
//! ## Usage Examples
//!
//! ```rust
//! use crate::actors::{TrackedFiles, SelectedVariables};
//! 
//! // Initialize domain actors
//! let tracked_files = initialize_tracked_files();
//! let selected_vars = initialize_selected_variables();
//!
//! // Use event relays
//! tracked_files.file_dropped_relay.send(vec![path]);
//! ```

// Re-export dataflow primitives for convenience
// This allows existing code to continue using crate::actors::{...}
pub use crate::dataflow::{
    // Core types
    Actor, ActorVec, ActorMap, Atom, Relay, RelayError,
    // Handle types
    ActorStateHandle, ActorVecHandle, ActorMapHandle,
    // Functions and traits
    relay, Stream, StreamExt, select
};
pub mod tracked_files;
pub mod selected_variables;
pub mod waveform_timeline;
pub mod user_configuration;
pub mod global_domains;
pub mod domain_bridges;
pub mod variable_helpers;
pub mod naming_validation;
pub mod testing;
pub use tracked_files::{TrackedFiles, initialize_tracked_files, get_tracked_files};
pub use selected_variables::{SelectedVariables, initialize_selected_variables, get_selected_variables};
pub use waveform_timeline::{WaveformTimeline, TimelineStats, initialize_waveform_timeline, get_waveform_timeline};
pub use user_configuration::{UserConfiguration, initialize_user_configuration, get_user_configuration};
pub use global_domains::{
    initialize_all_domains, 
    tracked_files_domain, 
    selected_variables_domain, 
    waveform_timeline_domain, 
    _user_configuration_domain,
    _are_domains_initialized
};
pub use domain_bridges::initialize_domain_bridges;
pub use variable_helpers::{create_selected_variable, _is_variable_selected};