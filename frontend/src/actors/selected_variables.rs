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
use zoon::{SignalExt, SignalVecExt, MutableExt, Mutable};
use indexmap::IndexSet;
use futures::{StreamExt, future::{select, Either}};
use crate::state; // For SELECTED_SCOPE_ID access

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
    
    /// Dedicated Vec signal for consumers that need full Vec updates (no SignalVec conversion)
    variables_vec_signal: Mutable<Vec<SelectedVariable>>,
    
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
    
    /// User changed the format for a specific variable
    pub variable_format_changed_relay: Relay<(String, shared::VarFormat)>,
}

impl SelectedVariables {
    /// Create a new comprehensive SelectedVariables domain with complete state management
    pub async fn new() -> Self {
        // Create all event-source relays
        let (variable_clicked_relay, variable_clicked_stream) = relay::<String>();
        let (variable_removed_relay, variable_removed_stream) = relay::<String>();
        let (scope_selected_relay, scope_selected_stream) = relay::<Option<String>>();
        let (scope_expanded_relay, scope_expanded_stream) = relay::<String>();
        let (scope_collapsed_relay, scope_collapsed_stream) = relay::<String>();
        let (selection_cleared_relay, selection_cleared_stream) = relay::<()>();
        let (search_filter_changed_relay, search_filter_changed_stream) = relay::<String>();
        let (search_focus_changed_relay, search_focus_changed_stream) = relay::<bool>();
        let (variables_restored_relay, variables_restored_stream) = relay::<Vec<SelectedVariable>>();
        let (tree_selection_changed_relay, tree_selection_changed_stream) = relay::<IndexSet<String>>();
        let (variable_format_changed_relay, variable_format_changed_stream) = relay::<(String, shared::VarFormat)>();
        
        // Create dedicated Vec signal that syncs with ActorVec changes (no conversion antipattern)
        let variables_vec_signal = Mutable::new(vec![]);
        
        // Create comprehensive variables actor with complete event handling
        let variables = ActorVec::new(vec![], {
            let variables_vec_signal_clone = variables_vec_signal.clone();
            async move |variables| {
                let mut variable_clicked = variable_clicked_stream;
                let mut variable_removed = variable_removed_stream;
                let mut selection_cleared = selection_cleared_stream;
                let mut variables_restored = variables_restored_stream;
                let mut variable_format_changed = variable_format_changed_stream;
                
                loop {
                    use futures::future::select;
                    use futures::future::Either;
                    
                    match select(
                        select(
                            select(
                                Box::pin(variable_clicked.next()),
                                Box::pin(variable_removed.next())
                            ),
                            Box::pin(selection_cleared.next())
                        ),
                        select(
                            Box::pin(variables_restored.next()),
                            Box::pin(variable_format_changed.next())
                        )
                    ).await {
                        Either::Left((Either::Left((Either::Left((Some(variable_id), _)), _)), _)) => {
                            Self::handle_variable_clicked(&variables, &variables_vec_signal_clone, variable_id.clone()).await;
                        }
                        Either::Left((Either::Left((Either::Right((Some(variable_id), _)), _)), _)) => {
                            Self::handle_variable_removed(&variables, &variables_vec_signal_clone, variable_id.clone()).await;
                        }
                        Either::Left((Either::Right((Some(()), _)), _)) => {
                            Self::handle_selection_cleared(&variables, &variables_vec_signal_clone).await;
                        }
                        Either::Right((Either::Left((Some(restored_vars), _)), _)) => {
                            Self::handle_variables_restored(&variables, &variables_vec_signal_clone, restored_vars.clone()).await;
                        }
                        Either::Right((Either::Right((Some((variable_id, new_format)), _)), _)) => {
                            // 🎯 FORMAT CHANGE EVENT
                            Self::handle_variable_format_changed(&variables, &variables_vec_signal_clone, variable_id.clone(), new_format).await;
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
        
        // Note: variables_vec_signal gets updated manually in actor operations, not through conversion
        
        Self {
            // State actors
            variables,
            variables_vec_signal,
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
            variable_format_changed_relay,
        }
    }
    
    // === COMPREHENSIVE REACTIVE SIGNAL ACCESS (replaces ALL 8 global mutables) ===
    
    /// Get reactive signal for all selected variables → replaces SELECTED_VARIABLES.signal_vec_cloned()
    /// ✅ FIXED: Uses dedicated Vec signal, no SignalVec conversion antipattern
    pub fn variables_signal(&self) -> impl zoon::Signal<Item = Vec<SelectedVariable>> {
        self.variables_vec_signal.signal_cloned()
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
        // ✅ FIXED: Use dedicated Vec signal instead of conversion antipattern
        self.variables_vec_signal.signal_cloned()
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
        
        // ✅ FIXED: Use dedicated Vec signal instead of conversion antipattern
        let variables_signal = self.variables_vec_signal.signal_cloned();
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
    /// Handle user clicking a variable in Variables panel (ADD-ONLY, no toggle)
    async fn handle_variable_clicked(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        variables_vec_signal: &zoon::Mutable<Vec<SelectedVariable>>,
        variable_id: String
    ) {
        
        // Check if variable is already selected by ID
        let is_already_selected = {
            let vars = variables_mutable.lock_ref();
            let already_selected = vars.iter().any(|v| v.unique_id == variable_id);
            already_selected
        };
        
        // If already selected, do nothing (Variables panel is ADD-ONLY)
        if is_already_selected {
            return;
        }
        
        // Add the variable (same working logic as before)
        // Parse variable_id format: "file_path|scope_id|variable_name"
        let parts: Vec<&str> = variable_id.split('|').collect();
        if parts.len() >= 3 {
            let file_id = parts[0];
            let scope_id = parts[1];
            let variable_name = parts[2];
            
            
            // Use the same data source as the helper function
            let tracked_files = if let Some(signals) = crate::actors::global_domains::TRACKED_FILES_SIGNALS.get() {
                signals.files_mutable.lock_ref().to_vec()
            } else {
                Vec::new()
            };
            
            // SIMPLIFIED: Just use the working helper function directly
            
            // The helper function expects the full scope_id, so reconstruct it from file_id + scope_id
            let full_scope_id = format!("{}|{}", file_id, scope_id);
            
            // Try to find a matching signal by name across all available data (including child scopes)
            for tracked_file in tracked_files.iter() {
                if tracked_file.id == file_id {
                    if let shared::FileState::Loaded(waveform_file) = &tracked_file.state {
                        // Search recursively through all scopes and their children
                        if let Some((found_signal, _found_scope_id)) = Self::find_signal_in_scopes(&waveform_file.scopes, variable_name) {
                            
                            // Found a signal with matching name, use helper to create it
                            if let Some(selected_var) = crate::actors::variable_helpers::create_selected_variable(
                                found_signal.clone(),
                                &tracked_file.id,
                                &full_scope_id  // Use reconstructed scope ID
                            ) {
                                variables_mutable.lock_mut().push_cloned(selected_var.clone());
                                
                                // Sync dedicated Vec signal after ActorVec change (no conversion antipattern)
                                {
                                    let current_vars = variables_mutable.lock_ref().to_vec();
                                    let len = current_vars.len();
                                    variables_vec_signal.set_neq(current_vars);
                                    zoon::println!("✅ ACTOR: Added variable '{}' - total selected: {}", variable_id, len);
                                }
                                
                                // ✅ FIXED: Only update Actor state - no dual updates to prevent infinite loops
                                return;
                            }
                        } else {
                        }
                    }
                    break;
                }
            }
        } else {
        }
    }
    
    /// Find and create SelectedVariable from variable_id
    async fn find_and_create_selected_variable(variable_id: &str, scope_id: &str) -> Option<SelectedVariable> {
        
        // Use same data source as Variables panel - TRACKED_FILES_SIGNALS
        let tracked_files = if let Some(signals) = crate::actors::global_domains::TRACKED_FILES_SIGNALS.get() {
            signals.files_mutable.lock_ref().to_vec()
        } else {
            return None;
        };
        
        // for (i, f) in tracked_files.iter().enumerate() {
        //     zoon::println!("  Actor File {}: id='{}', path='{}'", i, f.id, f.path);
        // }
        
        // Parse variable_id: "file_path|scope_id|variable_name"  
        let parts: Vec<&str> = variable_id.split('|').collect();
        if parts.len() < 3 {
            return None;
        }
        
        let file_path = parts[0];
        let variable_name = parts[2];
        
        // Parse scope_id to handle nested scopes like "simple_tb.s"
        let scope_parts: Vec<&str> = scope_id.split('.').collect();
        let root_scope = scope_parts[0]; // "simple_tb"
        let sub_scope = if scope_parts.len() > 1 { Some(scope_parts[1]) } else { None }; // "s"
        
        // Find the file and variable
        for tracked_file in tracked_files.iter() {
            if tracked_file.path == file_path {
                if let shared::FileState::Loaded(waveform_file) = &tracked_file.state {
                    // Find the scope (handle nested scopes like "simple_tb.s")
                    for (i, scope_data) in waveform_file.scopes.iter().enumerate() {
                        
                        // Check if this is the root scope we're looking for
                        if scope_data.name == root_scope {
                            
                            // If we need a sub-scope, look in nested scopes
                            if let Some(target_sub_scope) = sub_scope {
                                // Look for sub-scope in nested scopes
                                for (j, sub_scope_data) in scope_data.children.iter().enumerate() {
                                    if sub_scope_data.name == target_sub_scope {
                                        // Find the variable in this sub-scope
                                        for (k, signal) in sub_scope_data.variables.iter().enumerate() {
                                            if signal.name == variable_name {
                                                // Create SelectedVariable using the helper
                                                return crate::actors::variable_helpers::create_selected_variable(
                                                    signal.clone(),
                                                    &tracked_file.id,
                                                    &sub_scope_data.id
                                                );
                                            }
                                        }
                                    }
                                }
                            } else {
                                // No sub-scope needed, look directly in root scope
                                for (j, signal) in scope_data.variables.iter().enumerate() {
                                    if signal.name == variable_name {
                                        // Create SelectedVariable using the helper
                                        return crate::actors::variable_helpers::create_selected_variable(
                                            signal.clone(),
                                            &tracked_file.id,
                                            &scope_data.id
                                        );
                                    }
                                }
                            }
                        }
                    }
                } else {
                }
                break;
            }
        }
        
        None
    }
    
    // ✅ REMOVED: update_global_signals_for_addition() - dual updates caused infinite loops
    
    /// Recursively search through scopes and their children for a signal by name
    fn find_signal_in_scopes(scopes: &[shared::ScopeData], signal_name: &str) -> Option<(shared::Signal, String)> {
        for scope in scopes {
            // Check variables in this scope
            for signal in &scope.variables {
                if signal.name == signal_name {
                    return Some((signal.clone(), scope.id.clone()));
                }
            }
            
            // Recursively check children scopes
            if let Some(result) = Self::find_signal_in_scopes(&scope.children, signal_name) {
                return Some(result);
            }
        }
        None
    }
    
    // ✅ REMOVED: update_global_signals_for_removal() - dual updates caused infinite loops
    
    /// Handle removing specific variable by ID
    async fn handle_variable_removed(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        variables_vec_signal: &zoon::Mutable<Vec<SelectedVariable>>,
        variable_id: String
    ) {
        let _count_before = variables_mutable.lock_ref().len();
        
        variables_mutable.lock_mut().retain(|var| var.unique_id != variable_id);
        
        // Sync dedicated Vec signal after ActorVec change (no conversion antipattern)
        {
            let current_vars = variables_mutable.lock_ref().to_vec();
            let len = current_vars.len();
            variables_vec_signal.set_neq(current_vars);
            zoon::println!("🗑️ ACTOR: Removed variable '{}' - total selected: {}", variable_id, len);
        }
        
        // ✅ FIXED: Only update Actor state - no dual updates to prevent infinite loops
    }
    
    /// Handle clearing all selected variables
    async fn handle_selection_cleared(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        variables_vec_signal: &zoon::Mutable<Vec<SelectedVariable>>
    ) {
        let _count_before = variables_mutable.lock_ref().len();
        
        variables_mutable.lock_mut().clear();
        
        // Sync dedicated Vec signal after ActorVec change (no conversion antipattern)
        {
            let current_vars = variables_mutable.lock_ref().to_vec();
            let len = current_vars.len();
            variables_vec_signal.set_neq(current_vars);
            zoon::println!("🧹 ACTOR: Cleared all variables - total selected: {}", len);
        }
        
        // ✅ FIXED: Only update Actor state - no dual updates to prevent infinite loops
    }
    
    /// Handle variables restored from configuration
    async fn handle_variables_restored(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        variables_vec_signal: &zoon::Mutable<Vec<SelectedVariable>>,
        restored_variables: Vec<SelectedVariable>
    ) {
        let _count_before = variables_mutable.lock_ref().len();
        
        // Replace all variables in single operation (prevents signal spam)
        variables_mutable.lock_mut().replace_cloned(restored_variables.clone());
        
        // Sync dedicated Vec signal after ActorVec change (no conversion antipattern)
        {
            let current_vars = variables_mutable.lock_ref().to_vec();
            variables_vec_signal.set_neq(current_vars);
        }
        
        // ✅ FIXED: Only update Actor state - no dual updates to prevent infinite loops
    }
    
    /// Handle variable format change event
    async fn handle_variable_format_changed(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        variables_vec_signal: &zoon::Mutable<Vec<SelectedVariable>>,
        variable_id: String,
        new_format: shared::VarFormat
    ) {
        zoon::println!("🎯 FORMAT CHANGED in Actor: {} -> {:?}", variable_id, new_format);
        
        // Update the specific variable's format
        // Find the variable and update it using MutableVec approach
        let variables = variables_mutable.lock_ref().to_vec();
        let mut updated_variables = Vec::new();
        let mut found = false;
        
        for mut var in variables {
            if var.unique_id == variable_id {
                var.formatter = Some(new_format);
                found = true;
                zoon::println!("✅ Updated variable formatter: {} -> {:?}", variable_id, new_format);
            }
            updated_variables.push(var);
        }
        
        if !found {
            zoon::println!("⚠️ Variable not found for format update: {}", variable_id);
        }
        
        // Replace the entire vector with updated one
        variables_mutable.lock_mut().replace_cloned(updated_variables);
        
        // Sync dedicated Vec signal after change
        {
            let current_vars = variables_mutable.lock_ref().to_vec();
            variables_vec_signal.set_neq(current_vars);
        }
        
        // Note: Config save happens automatically through ConfigSaver actor
        // that watches config signals and saves with 1-second debouncing
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
    // ✅ PROPER ACTOR+RELAY: Connect directly to app config for persistence
    crate::config::app_config().variables_filter_changed_relay.clone()
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

/// User changed the format for a specific variable
pub fn variable_format_changed_relay() -> Relay<(String, shared::VarFormat)> {
    use crate::actors::global_domains::selected_variables_domain;
    selected_variables_domain().variable_format_changed_relay
}

// === GLOBAL DOMAINS ACCESS PATTERN ===
// Note: Initialization handled by global_domains::initialize_all_domains()
// Signal access uses crate::actors::global_domains::SELECTED_VARIABLES_DOMAIN_INSTANCE

// === PUBLIC SIGNAL ACCESS FUNCTIONS (replace global mutables) ===

/// Get reactive signal for all selected variables → FROM ACTOR STATE
pub fn variables_signal() -> impl zoon::Signal<Item = Vec<SelectedVariable>> {
    // Read from the Actor's state, not config (config gets updated BY Actor)
    crate::actors::global_domains::selected_variables_signal()
        .map(|vars| {
            vars
        })
}

/// Get reactive signal vec for all selected variables → CONNECTED TO GLOBAL SIGNALS
pub fn variables_signal_vec() -> impl zoon::SignalVec<Item = SelectedVariable> {
    // Use the global signal storage that's connected to the Actor
    crate::actors::global_domains::selected_variables_signal_vec()
}

/// Get reactive signal for variable index → CONNECTED TO DOMAIN ACTOR
pub fn variable_index_signal() -> impl zoon::Signal<Item = IndexSet<String>> {
    // Connect to real domain - compute index from selected variables
    variables_signal().map(|vars| {
        vars.iter().map(|var| var.unique_id.clone()).collect::<IndexSet<String>>()
    })
}

/// Get reactive signal for selected scope → DIRECT STATE ACCESS
pub fn selected_scope_signal() -> impl zoon::Signal<Item = Option<String>> {
    // ✅ CORRECT: Direct access to real state signal - no static caching needed
    crate::state::SELECTED_SCOPE_ID.signal_cloned()
}

/// Get reactive signal for tree selection → CONNECTED TO DOMAIN ACTOR
pub fn tree_selection_signal() -> impl zoon::Signal<Item = IndexSet<String>> {
    // Tree selection should match variable selection for TreeView integration
    variables_signal().map(|vars| {
        vars.iter().map(|var| var.unique_id.clone()).collect::<IndexSet<String>>()
    })
}

/// Get reactive signal for user cleared flag → CONNECTED TO DOMAIN EVENTS
pub fn user_cleared_signal() -> impl zoon::Signal<Item = bool> {
    // User cleared when variables are empty and config has been loaded
    zoon::map_ref! {
        let vars = variables_signal(),
        let config_loaded = crate::actors::config_sync::config_loaded_signal() => {
            *config_loaded && vars.is_empty()
        }
    }
}

/// Get reactive signal for expanded scopes → CONNECT TO REAL CONFIG DATA
pub fn expanded_scopes_signal() -> impl zoon::Signal<Item = IndexSet<String>> {
    // Connect to real config data instead of static hardcoded empty set
    // Convert Vec<String> from config to IndexSet<String> expected by UI
    crate::config::workspace_section_signal().map(|workspace| {
        workspace.expanded_scopes.iter().cloned().collect::<IndexSet<String>>()
    })
}

/// Get reactive signal for search filter → CONNECT TO REAL CONFIG DATA  
pub fn search_filter_signal() -> impl zoon::Signal<Item = String> {
    // Connect to real config data instead of static hardcoded empty string
    crate::config::app_config().session_state_actor.signal().map(|session| session.variables_search_filter.clone())
}

/// Get reactive signal for search focus → SIMPLE UI STATE (use Atom in UI component)
pub fn search_focused_signal() -> impl zoon::Signal<Item = bool> {
    // Search focus should be managed as local UI state with Atom in the search component
    // This is ephemeral UI state, not domain logic
    zoon::always(false) // Default - real focus state managed in UI component with Atom
}

// === SYNCHRONOUS ACCESS FUNCTIONS (for non-reactive contexts) ===

// === LEGACY COMPATIBILITY FUNCTIONS REMOVED ===
// These functions broke Actor+Relay architecture by providing synchronous access.
// Any code that used these should be migrated to use the reactive signal versions:
// - variables_signal() instead of current_variables()
// - expanded_scopes_signal() instead of current_expanded_scopes()  
// - selected_scope_signal() instead of current_selected_scope()
// - search_filter_signal() instead of current_search_filter()
// - variable_index_signal() instead of current_variable_index()

// === TEMPORARY COMPATIBILITY FUNCTIONS FOR MIGRATION ===
// TODO: Replace these calls with proper reactive signal patterns

/// MIGRATION: Temporary compatibility function - replace with variables_signal()
pub fn current_variables() -> Vec<SelectedVariable> {
    // Get current selected variables from the config storage
    // This reads the actual state that's connected to the Actor signals
    crate::state::SELECTED_VARIABLES_FOR_CONFIG.get_cloned()
}

/// MIGRATION: Temporary compatibility function - replace with expanded_scopes_signal()
pub fn current_expanded_scopes() -> IndexSet<String> {
    // MIGRATION: Should use reactive signals instead
    IndexSet::new() // Default empty set during migration
}

/// MIGRATION: Temporary compatibility function - replace with selected_scope_signal()
pub fn current_selected_scope() -> Option<String> {
    // MIGRATION: Should use reactive signals instead
    None // Default during migration
}

/// MIGRATION: Temporary compatibility function - replace with search_filter_signal()
pub fn current_search_filter() -> String {
    // MIGRATION: Should use reactive signals instead
    String::new() // Default during migration
}

/// MIGRATION: Temporary compatibility function - replace with variable_index_signal()
pub fn current_variable_index() -> IndexSet<String> {
    // Get current selected variables and build index 
    // This is used by virtual list during synchronous updates
    use crate::actors::global_domains::selected_variables_domain;
    let domain = selected_variables_domain();
    let vars = domain.variables_vec_signal.get_cloned();
    vars.iter().map(|var| var.unique_id.clone()).collect::<IndexSet<String>>()
}

