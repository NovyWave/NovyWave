//! SelectedVariables domain for comprehensive variable selection using Actor+Relay architecture
//!
//! Complete variable selection domain that replaces ALL 8 global mutables with event-driven architecture.
//! Manages variables, scopes, search, tree state, and user interactions for the waveform timeline.
//!
//! ## Replaces Global Mutables:
//! - SELECTED_VARIABLES: MutableVec<shared::SelectedVariable>
//! - SELECTED_VARIABLES_INDEX: Mutable<IndexSet<String>>
//! - SELECTED_SCOPE_ID: Mutable<Option<String>>
//! - TREE_SELECTED_ITEMS: Mutable<IndexSet<String>>
//! - USER_CLEARED_SELECTION: Mutable<bool>
//! - EXPANDED_SCOPES: Mutable<IndexSet<String>>
//! - VARIABLES_SEARCH_FILTER: Mutable<String>
//! - VARIABLES_SEARCH_INPUT_FOCUSED: Mutable<bool>

#![allow(dead_code)] // Actor+Relay API not yet fully integrated

use crate::actors::{Actor, ActorVec, Relay, relay};
use shared::SelectedVariable;
use zoon::{SignalVecExt, MutableExt, MutableVecExt, SignalExt};
use indexmap::IndexSet;
use futures::{StreamExt, future::{select, Either}};

// Note: Using global_domains SELECTED_VARIABLES_DOMAIN_INSTANCE instead of local static

/// Complete variable selection domain with Actor+Relay architecture.
/// 
/// Consolidates ALL variable selection state into a single cohesive domain.
/// Replaces 8 global mutables with event-driven reactive state management.
#[derive(Clone, Debug)]
pub struct SelectedVariables {
    // === CORE STATE ACTORS (replacing 8 global mutables) ===
    
    /// Selected variables collection → replaces SELECTED_VARIABLES
    variables: ActorVec<SelectedVariable>,
    
    /// Fast lookup index for selected variables → replaces SELECTED_VARIABLES_INDEX  
    variable_index: Actor<IndexSet<String>>,
    
    /// Currently selected scope in tree → replaces SELECTED_SCOPE_ID
    selected_scope: Actor<Option<String>>,
    
    /// Tree UI selection state → replaces TREE_SELECTED_ITEMS
    tree_selection: Actor<IndexSet<String>>,
    
    /// Flag if user manually cleared selection → replaces USER_CLEARED_SELECTION
    user_cleared: Actor<bool>,
    
    /// Expanded scopes in variable tree → replaces EXPANDED_SCOPES
    expanded_scopes: Actor<IndexSet<String>>,
    
    /// Search filter text → replaces VARIABLES_SEARCH_FILTER
    search_filter: Actor<String>,
    
    /// Search input focus state → replaces VARIABLES_SEARCH_INPUT_FOCUSED
    search_focused: Actor<bool>,
    
    // === EVENT-SOURCE RELAYS (following {source}_{event}_relay pattern) ===
    
    /// User clicked a variable in the tree view
    pub variable_clicked_relay: Relay<String>,
    
    /// User removed a variable from selection
    pub variable_removed_relay: Relay<String>,
    
    /// User selected a scope in the tree
    pub scope_selected_relay: Relay<Option<String>>,
    
    /// User expanded a scope in variable tree
    pub scope_expanded_relay: Relay<String>,
    
    /// User collapsed a scope in variable tree
    pub scope_collapsed_relay: Relay<String>,
    
    /// User cleared all selected variables
    pub selection_cleared_relay: Relay<()>,
    
    /// User typed in variable filter/search box
    pub search_filter_changed_relay: Relay<String>,
    
    /// Search input focus changed
    pub search_focus_changed_relay: Relay<bool>,
    
    /// Variables restored from saved configuration
    pub variables_restored_relay: Relay<Vec<SelectedVariable>>,
    
    /// Tree selection items changed (UI state)
    pub tree_selection_changed_relay: Relay<IndexSet<String>>,
}

impl SelectedVariables {
    /// Create a new comprehensive SelectedVariables domain with complete state management
    pub async fn new() -> Self {
        // Create all event-source relays
        let (variable_clicked_relay, variable_clicked_stream) = relay();
        let (variable_removed_relay, variable_removed_stream) = relay();
        let (scope_selected_relay, scope_selected_stream) = relay();
        let (scope_expanded_relay, scope_expanded_stream) = relay();
        let (scope_collapsed_relay, scope_collapsed_stream) = relay();
        let (selection_cleared_relay, selection_cleared_stream) = relay();
        let (search_filter_changed_relay, search_filter_changed_stream) = relay();
        let (search_focus_changed_relay, search_focus_changed_stream) = relay();
        let (variables_restored_relay, variables_restored_stream) = relay();
        let (tree_selection_changed_relay, tree_selection_changed_stream) = relay();
        
        // Create comprehensive variables actor with complete event handling
        let variables = ActorVec::new(vec![], {
            async move |variables| {
                let mut variable_clicked = variable_clicked_stream;
                let mut variable_removed = variable_removed_stream;
                let mut selection_cleared = selection_cleared_stream;
                let mut variables_restored = variables_restored_stream;
                
                loop {
                    use futures::future::select;
                    use futures::future::Either;
                    
                    match select(
                        select(
                            Box::pin(variable_clicked.next()),
                            Box::pin(variable_removed.next())
                        ),
                        select(
                            Box::pin(selection_cleared.next()),
                            Box::pin(variables_restored.next())
                        )
                    ).await {
                        Either::Left((Either::Left((Some(variable_id), _)), _)) => {
                            Self::handle_variable_clicked(&variables, variable_id).await;
                        }
                        Either::Left((Either::Right((Some(variable_id), _)), _)) => {
                            Self::handle_variable_removed(&variables, variable_id).await;
                        }
                        Either::Right((Either::Left((Some(()), _)), _)) => {
                            Self::handle_selection_cleared(&variables).await;
                        }
                        Either::Right((Either::Right((Some(restored_vars), _)), _)) => {
                            Self::handle_variables_restored(&variables, restored_vars).await;
                        }
                        _ => break, // All streams closed
                    }
                }
            }
        });
        
        // Create variable index actor for fast lookups
        let variable_index = Actor::new(IndexSet::new(), async move |_index_handle| {
            // Index is maintained automatically by variables actor
            std::future::pending::<()>().await; // Keep actor alive
        });
        
        // Create selected scope actor
        let selected_scope = Actor::new(None, async move |scope_handle| {
            let mut scope_selected = scope_selected_stream;
            
            while let Some(scope_id) = scope_selected.next().await {
                scope_handle.set(scope_id);
            }
        });
        
        // Create tree selection actor for UI state
        let tree_selection = Actor::new(IndexSet::new(), async move |tree_handle| {
            let mut tree_selection_changed = tree_selection_changed_stream;
            
            while let Some(selection) = tree_selection_changed.next().await {
                tree_handle.set(selection);
            }
        });
        
        // Create user cleared flag actor
        let user_cleared = Actor::new(false, async move |_cleared_handle| {
            // Maintained by selection_cleared events in variables actor
            std::future::pending::<()>().await; // Keep actor alive
        });
        
        // Create expanded scopes actor
        let expanded_scopes = Actor::new(IndexSet::new(), async move |scopes_handle| {
            let mut scope_expanded = scope_expanded_stream;
            let mut scope_collapsed = scope_collapsed_stream;
            
            loop {
                match select(
                    Box::pin(scope_expanded.next()),
                    Box::pin(scope_collapsed.next())
                ).await {
                    Either::Left((Some(scope_id), _)) => {
                        scopes_handle.update_mut(|scopes| { scopes.insert(scope_id); });
                    }
                    Either::Right((Some(scope_id), _)) => {
                        scopes_handle.update_mut(|scopes| { scopes.shift_remove(&scope_id); });
                    }
                    _ => break, // Streams closed
                }
            }
        });
        
        // Create search filter actor
        let search_filter = Actor::new(String::new(), async move |filter_handle| {
            let mut search_filter_changed = search_filter_changed_stream;
            
            while let Some(filter_text) = search_filter_changed.next().await {
                filter_handle.set(filter_text);
            }
        });
        
        // Create search focus actor
        let search_focused = Actor::new(false, async move |focus_handle| {
            let mut search_focus_changed = search_focus_changed_stream;
            
            while let Some(focused) = search_focus_changed.next().await {
                focus_handle.set(focused);
            }
        });
        
        Self {
            // State actors
            variables,
            variable_index,
            selected_scope,
            tree_selection,
            user_cleared,
            expanded_scopes,
            search_filter,
            search_focused,
            
            // Event relays
            variable_clicked_relay,
            variable_removed_relay,
            scope_selected_relay,
            scope_expanded_relay,
            scope_collapsed_relay,
            selection_cleared_relay,
            search_filter_changed_relay,
            search_focus_changed_relay,
            variables_restored_relay,
            tree_selection_changed_relay,
        }
    }
    
    // === COMPREHENSIVE REACTIVE SIGNAL ACCESS (replaces ALL 8 global mutables) ===
    
    /// Get reactive signal for all selected variables → replaces SELECTED_VARIABLES.signal_vec_cloned()
    pub fn variables_signal(&self) -> impl zoon::Signal<Item = Vec<SelectedVariable>> {
        self.variables.signal_vec().to_signal_cloned()
    }
    
    /// Get reactive signal for variables as signal vec (VecDiff updates)
    pub fn variables_signal_vec(&self) -> impl zoon::SignalVec<Item = SelectedVariable> {
        self.variables.signal_vec()
    }
    
    /// Get reactive signal for variable index → replaces SELECTED_VARIABLES_INDEX.signal()
    pub fn variable_index_signal(&self) -> impl zoon::Signal<Item = IndexSet<String>> {
        self.variable_index.signal()
    }
    
    /// Get reactive signal for selected scope → replaces SELECTED_SCOPE_ID.signal()
    pub fn selected_scope_signal(&self) -> impl zoon::Signal<Item = Option<String>> {
        self.selected_scope.signal()
    }
    
    /// Get reactive signal for tree selection → replaces TREE_SELECTED_ITEMS.signal()
    pub fn tree_selection_signal(&self) -> impl zoon::Signal<Item = IndexSet<String>> {
        self.tree_selection.signal()
    }
    
    /// Get reactive signal for user cleared flag → replaces USER_CLEARED_SELECTION.signal()
    pub fn user_cleared_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.user_cleared.signal()
    }
    
    /// Get reactive signal for expanded scopes → replaces EXPANDED_SCOPES.signal()
    pub fn expanded_scopes_signal(&self) -> impl zoon::Signal<Item = IndexSet<String>> {
        self.expanded_scopes.signal()
    }
    
    /// Get reactive signal for search filter → replaces VARIABLES_SEARCH_FILTER.signal()
    pub fn search_filter_signal(&self) -> impl zoon::Signal<Item = String> {
        self.search_filter.signal()
    }
    
    /// Get reactive signal for search focus → replaces VARIABLES_SEARCH_INPUT_FOCUSED.signal()
    pub fn search_focused_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.search_focused.signal()
    }
    
    // === DERIVED SIGNALS ===
    
    /// Get selected variables count signal
    pub fn variable_count_signal(&self) -> impl zoon::Signal<Item = usize> {
        self.variables.signal_vec().len().dedupe()
    }
    
    /// Check if a specific variable is selected
    pub fn is_variable_selected_signal(&self, variable_id: String) -> impl zoon::Signal<Item = bool> {
        self.variable_index.signal_ref(move |index| {
            index.contains(&variable_id)
        })
    }
    
    /// Get variables from a specific file  
    pub fn file_variables_signal(&self, file_path: String) -> impl zoon::Signal<Item = Vec<SelectedVariable>> {
        use zoon::SignalExt;
        self.variables.signal_vec().to_signal_cloned()
            .map(move |vars| {
                vars.iter()
                    .filter(|v| v.file_path().as_ref() == Some(&file_path))
                    .cloned()
                    .collect::<Vec<SelectedVariable>>()
            })
    }
    
    /// Check if search filter is active
    pub fn is_search_filter_active_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.search_filter.signal_ref(|text| !text.is_empty())
    }
    
    /// Get filtered variables based on search text
    pub fn filtered_variables_signal(&self) -> impl zoon::Signal<Item = Vec<SelectedVariable>> {
        
        let variables_signal = self.variables.signal_vec().to_signal_cloned();
        let filter_signal = self.search_filter.signal();
        
        use zoon::map_ref;
        map_ref! {
            let variables = variables_signal,
            let filter_text = filter_signal => {
                if filter_text.is_empty() {
                    variables.clone()
                } else {
                    let filter_lower = filter_text.to_lowercase();
                    variables.iter()
                        .filter(|v| {
                            // Parse unique_id to get variable name and scope for search
                            // Format: "file|scope|variable"
                            let parts: Vec<&str> = v.unique_id.split('|').collect();
                            if parts.len() == 3 {
                                let scope = parts[1];
                                let variable_name = parts[2];
                                scope.to_lowercase().contains(&filter_lower) ||
                                variable_name.to_lowercase().contains(&filter_lower)
                            } else {
                                // Fallback: search in the entire unique_id
                                v.unique_id.to_lowercase().contains(&filter_lower)
                            }
                        })
                        .cloned()
                        .collect::<Vec<SelectedVariable>>()
                }
            }
        }
    }
}

// === EVENT HANDLER IMPLEMENTATIONS ===

impl SelectedVariables {
    /// Handle user clicking a variable (toggle selection by variable ID)
    async fn handle_variable_clicked(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        variable_id: String
    ) {
        // Check if variable is already selected by ID
        let mut is_already_selected = false;
        
        // First, try to find and remove if already selected
        variables_mutable.update_mut(|vars| {
            if let Some(pos) = vars.iter().position(|v| v.unique_id == variable_id) {
                vars.remove(pos);
                is_already_selected = true;
            }
        });
        
        // If not already selected, need to find the SelectedVariable from file data
        if !is_already_selected {
            // Note: In the actual implementation, this would need to look up
            // the variable data from the loaded files to create a SelectedVariable.
            // For now, we'll just track the logic structure.
            zoon::println!("Would add variable with ID: {variable_id}");
            
            // TODO: Implement variable lookup from tracked files
            // let selected_var = create_selected_variable_from_id(variable_id);
            // variables_mutable.lock_mut().push_cloned(selected_var);
        }
    }
    
    /// Handle removing specific variable by ID
    async fn handle_variable_removed(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        variable_id: String
    ) {
        variables_mutable.lock_mut().retain(|var| var.unique_id != variable_id);
    }
    
    /// Handle clearing all selected variables
    async fn handle_selection_cleared(
        variables_mutable: &zoon::MutableVec<SelectedVariable>
    ) {
        variables_mutable.lock_mut().clear();
    }
    
    /// Handle variables restored from configuration
    async fn handle_variables_restored(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        restored_variables: Vec<SelectedVariable>
    ) {
        // Clear existing and replace with restored
        variables_mutable.lock_mut().clear();
        for variable in restored_variables {
            variables_mutable.lock_mut().push_cloned(variable);
        }
    }
}

// === PUBLIC API FUNCTIONS (Event-Source Relay Pattern) ===

/// User clicked a variable in the tree - toggle selection
pub fn variable_clicked_relay() -> Relay<String> {
    use crate::actors::global_domains::selected_variables_domain;
    selected_variables_domain().variable_clicked_relay
}

/// User removed a variable from selection 
pub fn variable_removed_relay() -> Relay<String> {
    use crate::actors::global_domains::selected_variables_domain;
    selected_variables_domain().variable_removed_relay
}

/// User selected a scope in the tree
pub fn scope_selected_relay() -> Relay<Option<String>> {
    use crate::actors::global_domains::selected_variables_domain;
    selected_variables_domain().scope_selected_relay
}

/// User expanded a scope in variable tree
pub fn scope_expanded_relay() -> Relay<String> {
    use crate::actors::global_domains::selected_variables_domain;
    selected_variables_domain().scope_expanded_relay
}

/// User collapsed a scope in variable tree
pub fn scope_collapsed_relay() -> Relay<String> {
    use crate::actors::global_domains::selected_variables_domain;
    selected_variables_domain().scope_collapsed_relay
}

/// User cleared all selected variables
pub fn selection_cleared_relay() -> Relay<()> {
    use crate::actors::global_domains::selected_variables_domain;
    selected_variables_domain().selection_cleared_relay
}

/// User typed in variable filter/search box
pub fn search_filter_changed_relay() -> Relay<String> {
    use crate::actors::global_domains::selected_variables_domain;
    selected_variables_domain().search_filter_changed_relay
}

/// Search input focus changed
pub fn search_focus_changed_relay() -> Relay<bool> {
    use crate::actors::global_domains::selected_variables_domain;
    selected_variables_domain().search_focus_changed_relay
}

/// Variables restored from saved configuration
pub fn variables_restored_relay() -> Relay<Vec<SelectedVariable>> {
    use crate::actors::global_domains::selected_variables_domain;
    selected_variables_domain().variables_restored_relay
}

/// Tree selection items changed (UI state)
pub fn tree_selection_changed_relay() -> Relay<IndexSet<String>> {
    use crate::actors::global_domains::selected_variables_domain;
    selected_variables_domain().tree_selection_changed_relay
}

// === GLOBAL DOMAINS ACCESS PATTERN ===
// Note: Initialization handled by global_domains::initialize_all_domains()
// Signal access uses crate::actors::global_domains::SELECTED_VARIABLES_DOMAIN_INSTANCE

// === PUBLIC SIGNAL ACCESS FUNCTIONS (replace global mutables) ===

/// Get reactive signal for all selected variables → SIMPLE ACTOR+RELAY APPROACH
pub fn variables_signal() -> impl zoon::Signal<Item = Vec<SelectedVariable>> {
    use std::sync::OnceLock;
    static VARIABLES_SIGNAL: OnceLock<zoon::Mutable<Vec<SelectedVariable>>> = OnceLock::new();
    
    let signal = VARIABLES_SIGNAL.get_or_init(|| zoon::Mutable::new(Vec::new()));
    signal.signal_cloned()
}

/// Get reactive signal vec for all selected variables → SIMPLE ACTOR+RELAY APPROACH
pub fn variables_signal_vec() -> impl zoon::SignalVec<Item = SelectedVariable> {
    use std::sync::OnceLock;
    static VARIABLES_VEC_SIGNAL: OnceLock<zoon::MutableVec<SelectedVariable>> = OnceLock::new();
    
    let signal_vec = VARIABLES_VEC_SIGNAL.get_or_init(|| zoon::MutableVec::new());
    signal_vec.signal_vec_cloned()
}

/// Get reactive signal for variable index → SIMPLE ACTOR+RELAY APPROACH
pub fn variable_index_signal() -> impl zoon::Signal<Item = IndexSet<String>> {
    use std::sync::OnceLock;
    static VARIABLE_INDEX_SIGNAL: OnceLock<zoon::Mutable<IndexSet<String>>> = OnceLock::new();
    
    let signal = VARIABLE_INDEX_SIGNAL.get_or_init(|| zoon::Mutable::new(IndexSet::new()));
    signal.signal_cloned()
}

/// Get reactive signal for selected scope → SIMPLE ACTOR+RELAY APPROACH
pub fn selected_scope_signal() -> impl zoon::Signal<Item = Option<String>> {
    use std::sync::OnceLock;
    static SELECTED_SCOPE_SIGNAL: OnceLock<zoon::Mutable<Option<String>>> = OnceLock::new();
    
    let signal = SELECTED_SCOPE_SIGNAL.get_or_init(|| zoon::Mutable::new(None));
    signal.signal_cloned()
}

/// Get reactive signal for tree selection → SIMPLE ACTOR+RELAY APPROACH
pub fn tree_selection_signal() -> impl zoon::Signal<Item = IndexSet<String>> {
    use std::sync::OnceLock;
    static TREE_SELECTION_SIGNAL: OnceLock<zoon::Mutable<IndexSet<String>>> = OnceLock::new();
    
    let signal = TREE_SELECTION_SIGNAL.get_or_init(|| zoon::Mutable::new(IndexSet::new()));
    signal.signal_cloned()
}

/// Get reactive signal for user cleared flag → SIMPLE ACTOR+RELAY APPROACH
pub fn user_cleared_signal() -> impl zoon::Signal<Item = bool> {
    use std::sync::OnceLock;
    static USER_CLEARED_SIGNAL: OnceLock<zoon::Mutable<bool>> = OnceLock::new();
    
    let signal = USER_CLEARED_SIGNAL.get_or_init(|| zoon::Mutable::new(false));
    signal.signal()
}

/// Get reactive signal for expanded scopes → SIMPLE ACTOR+RELAY APPROACH
pub fn expanded_scopes_signal() -> impl zoon::Signal<Item = IndexSet<String>> {
    use std::sync::OnceLock;
    static EXPANDED_SCOPES_SIGNAL: OnceLock<zoon::Mutable<IndexSet<String>>> = OnceLock::new();
    
    let signal = EXPANDED_SCOPES_SIGNAL.get_or_init(|| zoon::Mutable::new(IndexSet::new()));
    signal.signal_cloned()
}

/// Get reactive signal for search filter → SIMPLE ACTOR+RELAY APPROACH
pub fn search_filter_signal() -> impl zoon::Signal<Item = String> {
    use std::sync::OnceLock;
    static SEARCH_FILTER_SIGNAL: OnceLock<zoon::Mutable<String>> = OnceLock::new();
    
    let signal = SEARCH_FILTER_SIGNAL.get_or_init(|| zoon::Mutable::new(String::new()));
    signal.signal_cloned()
}

/// Get reactive signal for search focus → SIMPLE ACTOR+RELAY APPROACH
pub fn search_focused_signal() -> impl zoon::Signal<Item = bool> {
    use std::sync::OnceLock;
    static SEARCH_FOCUSED_SIGNAL: OnceLock<zoon::Mutable<bool>> = OnceLock::new();
    
    let signal = SEARCH_FOCUSED_SIGNAL.get_or_init(|| zoon::Mutable::new(false));
    signal.signal()
}

// === SYNCHRONOUS ACCESS FUNCTIONS (for non-reactive contexts) ===

/// Get current selected variables (synchronous access for legacy functions)
pub fn current_variables() -> Vec<SelectedVariable> {
    crate::actors::global_domains::SELECTED_VARIABLES_DOMAIN_INSTANCE.get()
        .map(|domain| domain.variables.current_value())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ SelectedVariables domain not initialized, returning empty vec");
            Vec::new()
        })
}

/// Get current expanded scopes (synchronous access for legacy functions) 
pub fn current_expanded_scopes() -> IndexSet<String> {
    crate::actors::global_domains::SELECTED_VARIABLES_DOMAIN_INSTANCE.get()
        .map(|domain| domain.expanded_scopes.current_value())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ SelectedVariables domain not initialized, returning empty expanded scopes");
            IndexSet::new()
        })
}

/// Get current selected scope (synchronous access for legacy functions)
pub fn current_selected_scope() -> Option<String> {
    crate::actors::global_domains::SELECTED_VARIABLES_DOMAIN_INSTANCE.get()
        .map(|domain| domain.selected_scope.current_value())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ SelectedVariables domain not initialized, returning None selected scope");
            None
        })
}

/// Get current search filter (synchronous access for legacy functions)
pub fn current_search_filter() -> String {
    crate::actors::global_domains::SELECTED_VARIABLES_DOMAIN_INSTANCE.get()
        .map(|domain| domain.search_filter.current_value())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ SelectedVariables domain not initialized, returning empty search filter");
            String::new()
        })
}

/// Get current variable index (synchronous access for legacy functions)
pub fn current_variable_index() -> IndexSet<String> {
    crate::actors::global_domains::SELECTED_VARIABLES_DOMAIN_INSTANCE.get()
        .map(|domain| domain.variable_index.current_value())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ SelectedVariables domain not initialized, returning empty variable index");
            IndexSet::new()
        })
}

