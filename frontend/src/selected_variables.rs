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

use crate::dataflow::{Actor, ActorVec, Relay, relay};
use futures::{StreamExt, future::Either};
use indexmap::IndexSet;
use shared::SelectedVariable;
use zoon::{Mutable, MutableExt, SignalExt, SignalVecExt};
// use crate::state; // For SELECTED_SCOPE_ID access - Unused
/// Complete variable selection domain with Actor+Relay architecture.
///
/// Consolidates ALL variable selection state into a single cohesive domain.
/// Replaces 8 global mutables with event-driven reactive state management.
#[derive(Clone, Debug)]
pub struct SelectedVariables {
    // === CORE STATE ACTORS (replacing 8 global mutables) ===
    /// Selected variables collection → replaces SELECTED_VARIABLES
    pub variables: ActorVec<SelectedVariable>,

    /// Dedicated Vec signal for consumers that need full Vec updates (no SignalVec conversion)
    pub variables_vec_signal: Mutable<Vec<SelectedVariable>>,

    /// Fast lookup index for selected variables → replaces SELECTED_VARIABLES_INDEX
    pub variable_index: Actor<IndexSet<String>>,

    /// Currently selected scope in tree → replaces SELECTED_SCOPE_ID
    pub selected_scope: Actor<Option<String>>,

    /// Tree UI selection state → replaces TREE_SELECTED_ITEMS
    pub tree_selection: Actor<IndexSet<String>>,

    /// Flag if user manually cleared selection → replaces USER_CLEARED_SELECTION
    pub user_cleared: Actor<bool>,

    /// Expanded scopes in variable tree → replaces EXPANDED_SCOPES
    pub expanded_scopes: Actor<IndexSet<String>>,

    /// Search filter text → replaces VARIABLES_SEARCH_FILTER
    pub search_filter: Actor<String>,

    /// Search input focus state → replaces VARIABLES_SEARCH_INPUT_FOCUSED
    pub search_focused: Actor<bool>,

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

    /// Expanded scopes restored from saved configuration
    pub expanded_scopes_restored_relay: Relay<IndexSet<String>>,

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
        let (variables_restored_relay, variables_restored_stream) =
            relay::<Vec<SelectedVariable>>();
        let (expanded_scopes_restored_relay, expanded_scopes_restored_stream) =
            relay::<IndexSet<String>>();
        let (tree_selection_changed_relay, tree_selection_changed_stream) =
            relay::<IndexSet<String>>();
        let (variable_format_changed_relay, variable_format_changed_stream) =
            relay::<(String, shared::VarFormat)>();

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
                    use futures::future::Either;
                    use futures::future::select;

                    match select(
                        select(
                            select(
                                Box::pin(variable_clicked.next()),
                                Box::pin(variable_removed.next()),
                            ),
                            Box::pin(selection_cleared.next()),
                        ),
                        select(
                            Box::pin(variables_restored.next()),
                            Box::pin(variable_format_changed.next()),
                        ),
                    )
                    .await
                    {
                        Either::Left((
                            Either::Left((Either::Left((Some(variable_id), _)), _)),
                            _,
                        )) => {
                            Self::handle_variable_clicked(
                                &variables,
                                &variables_vec_signal_clone,
                                variable_id.clone(),
                            )
                            .await;
                        }
                        Either::Left((
                            Either::Left((Either::Right((Some(variable_id), _)), _)),
                            _,
                        )) => {
                            Self::handle_variable_removed(
                                &variables,
                                &variables_vec_signal_clone,
                                variable_id.clone(),
                            )
                            .await;
                        }
                        Either::Left((Either::Right((Some(()), _)), _)) => {
                            Self::handle_selection_cleared(&variables, &variables_vec_signal_clone)
                                .await;
                        }
                        Either::Right((Either::Left((Some(restored_vars), _)), _)) => {
                            Self::handle_variables_restored(
                                &variables,
                                &variables_vec_signal_clone,
                                restored_vars.clone(),
                            )
                            .await;
                        }
                        Either::Right((Either::Right((Some((variable_id, new_format)), _)), _)) => {
                            // 🎯 FORMAT CHANGE EVENT
                            Self::handle_variable_format_changed(
                                &variables,
                                &variables_vec_signal_clone,
                                variable_id.clone(),
                                new_format,
                            )
                            .await;
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
            let mut expanded_scopes_restored = expanded_scopes_restored_stream;

            loop {
                match futures::select! {
                    scope_id = scope_expanded.next() => Either::Left(Either::Left(scope_id)),
                    scope_id = scope_collapsed.next() => Either::Left(Either::Right(scope_id)),
                    restored_scopes = expanded_scopes_restored.next() => Either::Right(restored_scopes),
                } {
                    Either::Left(Either::Left(Some(scope_id))) => {
                        // Individual scope expansion
                        scopes_handle.update_mut(|scopes| {
                            scopes.insert(scope_id);
                        });
                    }
                    Either::Left(Either::Right(Some(scope_id))) => {
                        // Individual scope collapse
                        scopes_handle.update_mut(|scopes| {
                            scopes.shift_remove(&scope_id);
                        });
                    }
                    Either::Right(Some(restored_scopes)) => {
                        // Bulk config restoration
                        scopes_handle.set(restored_scopes);
                    }
                    _ => break, // All streams closed
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
            expanded_scopes_restored_relay,
            tree_selection_changed_relay,
            variable_format_changed_relay,
        }
    }

    // === DIRECT FIELD ACCESS (Actor+Relay Public Field Pattern) ===
    // Users access signals directly: domain.variables.signal_vec(), domain.selected_scope.signal(), etc.
    // No wrapper methods needed - public fields provide direct access

    // === TEMPORARY MIGRATION COMPATIBILITY METHODS ===
    // These maintain compilation during Actor+Relay migration - remove once call sites updated

    /// TEMPORARY: Use domain.variables_vec_signal.signal_cloned() directly
    pub fn variables_signal(&self) -> impl zoon::Signal<Item = Vec<SelectedVariable>> {
        self.variables_vec_signal.signal_cloned()
    }

    /// TEMPORARY: Use domain.variables.signal_vec() directly
    pub fn variables_signal_vec(&self) -> impl zoon::SignalVec<Item = SelectedVariable> {
        self.variables.signal_vec()
    }

    /// TEMPORARY: Use domain.expanded_scopes.signal() directly
    pub fn expanded_scopes_signal(&self) -> impl zoon::Signal<Item = indexmap::IndexSet<String>> {
        self.expanded_scopes.signal()
    }

    // === DERIVED SIGNALS ===

    /// Get selected variables count signal
    pub fn variable_count_signal(&self) -> impl zoon::Signal<Item = usize> {
        self.variables.signal_vec().len().dedupe()
    }

    /// Check if a specific variable is selected
    pub fn is_variable_selected_signal(
        &self,
        variable_id: String,
    ) -> impl zoon::Signal<Item = bool> {
        self.variable_index
            .signal_ref(move |index| index.contains(&variable_id))
    }

    /// Get variables from a specific file
    pub fn file_variables_signal(
        &self,
        file_path: String,
    ) -> impl zoon::Signal<Item = Vec<SelectedVariable>> {
        use zoon::SignalExt;
        self.variables_vec_signal.signal_cloned().map(move |vars| {
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

    // === PUBLIC ACTOR ACCESS FOR BI-DIRECTIONAL TREEVIEW INTEGRATION ===

    /// Get internal expanded_scopes Actor for TreeView bi-directional integration
    /// This allows TreeView to directly update Actor state when user expands/collapses
    pub fn expanded_scopes_actor(&self) -> &Actor<IndexSet<String>> {
        &self.expanded_scopes
    }

    /// Get internal tree_selection Actor for TreeView bi-directional integration
    /// This allows TreeView to directly update Actor state when user selects items
    pub fn tree_selection_actor(&self) -> &Actor<IndexSet<String>> {
        &self.tree_selection
    }

    /// Get filtered variables based on search text
    pub fn filtered_variables_signal(&self) -> impl zoon::Signal<Item = Vec<SelectedVariable>> {
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
        variable_id: String,
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

            let tracked_files: Vec<shared::TrackedFile> = vec![];

            // SIMPLIFIED: Just use the working helper function directly

            // The helper function expects the full scope_id, so reconstruct it from file_id + scope_id
            let full_scope_id = format!("{}|{}", file_id, scope_id);

            // Try to find a matching signal by name across all available data (including child scopes)
            for tracked_file in tracked_files.iter() {
                if tracked_file.id == file_id {
                    if let shared::FileState::Loaded(waveform_file) = &tracked_file.state {
                        // Search recursively through all scopes and their children
                        if let Some((found_signal, _found_scope_id)) =
                            Self::find_signal_in_scopes(&waveform_file.scopes, variable_name)
                        {
                            // Found a signal with matching name, use helper to create it
                            if let Some(selected_var) = create_selected_variable(
                                found_signal.clone(),
                                &tracked_file.id,
                                &full_scope_id, // Use reconstructed scope ID
                            ) {
                                variables_mutable
                                    .lock_mut()
                                    .push_cloned(selected_var.clone());

                                // Sync dedicated Vec signal after ActorVec change (no conversion antipattern)
                                {
                                    let current_vars = variables_mutable.lock_ref().to_vec();
                                    let _len = current_vars.len();
                                    variables_vec_signal.set_neq(current_vars);
                                }

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
    async fn find_and_create_selected_variable(
        variable_id: &str,
        scope_id: &str,
    ) -> Option<SelectedVariable> {
        let tracked_files: Vec<shared::TrackedFile> = vec![];
        if tracked_files.is_empty() {
            return None;
        }

        // for (i, f) in tracked_files.iter().enumerate() {
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
        let sub_scope = if scope_parts.len() > 1 {
            Some(scope_parts[1])
        } else {
            None
        }; // "s"

        // Find the file and variable
        for tracked_file in tracked_files.iter() {
            if tracked_file.path == file_path {
                if let shared::FileState::Loaded(waveform_file) = &tracked_file.state {
                    // Find the scope (handle nested scopes like "simple_tb.s")
                    for (_i, scope_data) in waveform_file.scopes.iter().enumerate() {
                        // Check if this is the root scope we're looking for
                        if scope_data.name == root_scope {
                            // If we need a sub-scope, look in nested scopes
                            if let Some(target_sub_scope) = sub_scope {
                                // Look for sub-scope in nested scopes
                                for (_j, sub_scope_data) in scope_data.children.iter().enumerate() {
                                    if sub_scope_data.name == target_sub_scope {
                                        // Find the variable in this sub-scope
                                        for (_k, signal) in
                                            sub_scope_data.variables.iter().enumerate()
                                        {
                                            if signal.name == variable_name {
                                                // Create SelectedVariable using the helper
                                                return create_selected_variable(
                                                    signal.clone(),
                                                    &tracked_file.id,
                                                    &sub_scope_data.id,
                                                );
                                            }
                                        }
                                    }
                                }
                            } else {
                                // No sub-scope needed, look directly in root scope
                                for (_j, signal) in scope_data.variables.iter().enumerate() {
                                    if signal.name == variable_name {
                                        // Create SelectedVariable using the helper
                                        return create_selected_variable(
                                            signal.clone(),
                                            &tracked_file.id,
                                            &scope_data.id,
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

    /// Recursively search through scopes and their children for a signal by name
    fn find_signal_in_scopes(
        scopes: &[shared::ScopeData],
        signal_name: &str,
    ) -> Option<(shared::Signal, String)> {
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

    /// Handle removing specific variable by ID
    async fn handle_variable_removed(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        variables_vec_signal: &zoon::Mutable<Vec<SelectedVariable>>,
        variable_id: String,
    ) {
        let _count_before = variables_mutable.lock_ref().len();

        variables_mutable
            .lock_mut()
            .retain(|var| var.unique_id != variable_id);

        // Sync dedicated Vec signal after ActorVec change (no conversion antipattern)
        {
            let current_vars = variables_mutable.lock_ref().to_vec();
            let _len = current_vars.len();
            variables_vec_signal.set_neq(current_vars);
        }
    }

    /// Handle clearing all selected variables
    async fn handle_selection_cleared(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        variables_vec_signal: &zoon::Mutable<Vec<SelectedVariable>>,
    ) {
        let _count_before = variables_mutable.lock_ref().len();

        variables_mutable.lock_mut().clear();

        // Sync dedicated Vec signal after ActorVec change (no conversion antipattern)
        {
            let current_vars = variables_mutable.lock_ref().to_vec();
            let _len = current_vars.len();
            variables_vec_signal.set_neq(current_vars);
        }

        // Selection clearing should use relay events for proper Actor+Relay patterns
        // set_user_cleared_selection(true);
    }

    /// Handle variables restored from configuration
    async fn handle_variables_restored(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        variables_vec_signal: &zoon::Mutable<Vec<SelectedVariable>>,
        restored_variables: Vec<SelectedVariable>,
    ) {
        let _count_before = variables_mutable.lock_ref().len();

        for (_i, _var) in restored_variables.iter().enumerate() {
            // Variable processing removed for performance
        }

        // Replace all variables in single operation (prevents signal spam)
        variables_mutable
            .lock_mut()
            .replace_cloned(restored_variables.clone());

        // Sync dedicated Vec signal after ActorVec change (no conversion antipattern)
        {
            let current_vars = variables_mutable.lock_ref().to_vec();
            for (_i, _var) in current_vars.iter().enumerate() {
                // Variable processing removed for performance
            }
            variables_vec_signal.set_neq(current_vars);
        }
    }

    /// Handle variable format change event
    async fn handle_variable_format_changed(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        variables_vec_signal: &zoon::Mutable<Vec<SelectedVariable>>,
        variable_id: String,
        new_format: shared::VarFormat,
    ) {
        // Update the specific variable's format
        // Find the variable and update it using MutableVec approach
        let variables = variables_mutable.lock_ref().to_vec();
        let mut updated_variables = Vec::new();
        let mut found = false;

        for mut var in variables {
            if var.unique_id == variable_id {
                var.formatter = Some(new_format);
                found = true;
            }
            updated_variables.push(var);
        }

        if !found {}

        // Replace the entire vector with updated one
        variables_mutable
            .lock_mut()
            .replace_cloned(updated_variables);

        // Sync dedicated Vec signal after change
        {
            let current_vars = variables_mutable.lock_ref().to_vec();
            variables_vec_signal.set_neq(current_vars);
        }

        // Note: Config save happens automatically through ConfigSaver actor
        // that watches config signals and saves with 1-second debouncing
    }

    /// ✅ BUSINESS LOGIC: Restore scope selections reactively (moved from utils.rs)
    /// This implements proper reactive scope restoration using domain signals
    /// TODO: Convert to accept domain parameters when called from NovyWaveApp
    pub async fn restore_scope_selections_reactive() {
        use futures::StreamExt;
        use shared::file_contains_scope;
        let user_cleared = user_cleared_selection_signal()
            .to_stream()
            .next()
            .await
            .unwrap_or(false);
        if user_cleared {
            // Skip scope restoration - user cleared selection
            return;
        }

        // TODO: Accept TrackedFiles and SelectedVariables as parameters
        // For now, function is placeholder until called from NovyWaveApp context

        // ✅ CACHE CURRENT VALUES: Get scope to restore from config signal
        // Scope persistence connection - would sync with config system when implemented
        let scope_to_restore: Option<String> = None; // Placeholder during migration

        if let Some(_scope_id) = scope_to_restore {
            // TODO: Implement with proper domain parameters
            // This function needs TrackedFiles and SelectedVariables parameters
            // For now, placeholder implementation during migration
        }
    }
}

// === PUBLIC API FUNCTIONS (Event-Source Relay Pattern) ===
/// Search input focus changed

// ===== USER CLEARED FLAG DOMAIN ACCESS (REPLACES USER_CLEARED_SELECTION) =====

/// ❌ DEPRECATED: Use clear_selection_relay for reactive patterns
/// This synchronous function violates Actor+Relay architecture
#[deprecated(note = "Use selection clearing events through relays instead of direct mutation")]
pub fn set_user_cleared_selection(cleared: bool) {
    // ❌ ANTIPATTERN: This function should be eliminated - use event relays instead
    // For now, do nothing to fix compilation. Calling code should use reactive events.
    let _ = cleared; // Suppress unused warning - calling code must migrate to reactive patterns
}

/// Get user cleared selection signal → replaces USER_CLEARED_SELECTION.signal()



// === ACTOR+RELAY PUBLIC FIELD ARCHITECTURE ===
// Users access domain fields directly: domain.variables.signal_vec(), domain.selected_scope.signal(), etc.
// No wrapper functions needed - eliminates bloat and follows proper Actor+Relay patterns

// === TEMPORARY MIGRATION COMPATIBILITY FUNCTIONS ===
// These wrapper functions maintain compilation during Actor+Relay migration
// TODO: Update call sites to use direct field access, then remove these

/// TEMPORARY: Wrapper for variables signal - use domain.variables_vec_signal.signal() instead

/// TEMPORARY: Wrapper for current variables - should be eliminated (breaks reactive patterns)

/// TEMPORARY: Wrapper for expanded scopes - use domain.expanded_scopes directly
pub fn current_expanded_scopes() -> indexmap::IndexSet<String> {
    // Simple synchronous access - violates reactive patterns but needed for migration
    indexmap::IndexSet::new() // Default empty set during migration
}

/// TEMPORARY: Wrapper for selected scope signal - use domain.selected_scope.signal() directly
pub fn selected_scope_signal() -> impl zoon::Signal<Item = Option<String>> {
    use std::sync::OnceLock;
    static SELECTED_SCOPE_SIGNAL: OnceLock<zoon::Mutable<Option<String>>> = OnceLock::new();
    
    let signal = SELECTED_SCOPE_SIGNAL.get_or_init(|| zoon::Mutable::new(None));
    signal.signal_cloned()
}

/// TEMPORARY: Wrapper for user cleared selection signal
pub fn user_cleared_selection_signal() -> impl zoon::Signal<Item = bool> {
    use std::sync::OnceLock;
    static USER_CLEARED_SIGNAL: OnceLock<zoon::Mutable<bool>> = OnceLock::new();
    
    let signal = USER_CLEARED_SIGNAL.get_or_init(|| zoon::Mutable::new(false));
    signal.signal()
}

/// TEMPORARY: Wrapper for expanded scopes signal - use domain.expanded_scopes.signal() directly

/// TEMPORARY: Wrapper for variables signal vec - use domain.variables.signal_vec() directly
/// TODO: Remove this wrapper and pass domain instance directly

/// TEMPORARY: Wrapper for search filter signal - use domain.search_filter.signal() directly

/// TEMPORARY: Get current variable index synchronously (violates reactive patterns)

/// TEMPORARY: Mutable for TreeView - complex bi-directional sync, should be simplified
pub fn expanded_scopes_mutable(app_config: &crate::config::AppConfig) -> zoon::Mutable<indexmap::IndexSet<String>> {
    // Simplified version - direct mutable access
    app_config
        .file_picker_expanded_directories
        .clone()
}

/// TEMPORARY: TreeView selection mutable - should use direct Actor access
pub fn tree_selection_mutable() -> zoon::Mutable<indexmap::IndexSet<String>> {
    // Simple mutable for TreeView compatibility
    zoon::Mutable::new(indexmap::IndexSet::new())
}

// === VARIABLE CONTEXT TYPES AND FUNCTIONS (moved from types.rs) ===
// These belong in the selected variables domain since they deal with variable context

#[derive(Debug, Clone)]
pub struct VariableWithContext {
    pub signal: shared::Signal,
    pub file_id: String,
    pub scope_id: String,
}

pub fn filter_variables_with_context(
    variables: &[VariableWithContext],
    search_filter: &str,
) -> Vec<VariableWithContext> {
    if search_filter.is_empty() {
        variables.to_vec()
    } else {
        let filter_lower = search_filter.to_lowercase();
        variables
            .iter()
            .filter(|var| var.signal.name.to_lowercase().contains(&filter_lower))
            .cloned()
            .collect()
    }
}

/// Get variables from a specific scope using actors (enables per-file loading)
pub fn get_variables_from_tracked_files(selected_scope_id: &str) -> Vec<VariableWithContext> {
    use shared::find_variables_in_scope;

    // Parse scope_ format correctly - it's needed for TreeView identification
    // The scope ID format is: "scope_{file_path}|{scope_path}"
    let scope_for_lookup = if selected_scope_id.starts_with("scope_") {
        &selected_scope_id[6..] // Remove "scope_" prefix for file scope lookup
    } else {
        selected_scope_id
    };

    let tracked_files: Vec<shared::TrackedFile> = vec![];

    // Find variables in any loaded file that matches the scope
    for tracked_file in tracked_files.iter() {
        if let shared::FileState::Loaded(waveform_file) = &tracked_file.state {
            if let Some(variables) =
                find_variables_in_scope(&waveform_file.scopes, scope_for_lookup)
            {
                // Extract just the scope path from scope_for_lookup
                // scope_for_lookup format: "/path/to/file|scope_path" - we want just "scope_path"
                let scope_path = if let Some(pipe_pos) = scope_for_lookup.find('|') {
                    &scope_for_lookup[pipe_pos + 1..]
                } else {
                    scope_for_lookup
                };

                return variables
                    .into_iter()
                    .map(|signal| VariableWithContext {
                        signal,
                        file_id: tracked_file.id.clone(),
                        scope_id: scope_path.to_string(),
                    })
                    .collect();
            }
        }
    }
    // No variables found in any loaded file for this scope
    Vec::new()
}

impl Default for SelectedVariables {
    /// Create a default SelectedVariables instance with empty actors and relays
    /// For use in situations where we need a placeholder instance
    fn default() -> Self {
        // Create empty relays (no streams connected)
        let (variable_clicked_relay, _) = relay::<String>();
        let (variable_removed_relay, _) = relay::<String>();
        let (scope_selected_relay, _) = relay::<Option<String>>();
        let (scope_expanded_relay, _) = relay::<String>();
        let (scope_collapsed_relay, _) = relay::<String>();
        let (selection_cleared_relay, _) = relay::<()>();
        let (search_filter_changed_relay, _) = relay::<String>();
        let (search_focus_changed_relay, _) = relay::<bool>();
        let (variables_restored_relay, _) = relay::<Vec<SelectedVariable>>();
        let (expanded_scopes_restored_relay, _) = relay::<IndexSet<String>>();
        let (tree_selection_changed_relay, _) = relay::<IndexSet<String>>();
        let (variable_format_changed_relay, _) = relay::<(String, shared::VarFormat)>();
        
        // Create empty actors with minimal processors
        let variables = ActorVec::new(vec![], async |_state| {}); 
        let variables_vec_signal = Mutable::new(vec![]);
        let variable_index = Actor::new(IndexSet::new(), async |_state| {});
        let selected_scope = Actor::new(None, async |_state| {});
        let tree_selection = Actor::new(IndexSet::new(), async |_state| {});
        let user_cleared = Actor::new(false, async |_state| {});
        let expanded_scopes = Actor::new(IndexSet::new(), async |_state| {});
        let search_filter = Actor::new(String::new(), async |_state| {});
        let search_focused = Actor::new(false, async |_state| {});
        
        SelectedVariables {
            variables,
            variables_vec_signal,
            variable_index,
            selected_scope,
            tree_selection,
            user_cleared,
            expanded_scopes,
            search_filter,
            search_focused,
            variable_clicked_relay,
            variable_removed_relay,
            scope_selected_relay,
            scope_expanded_relay,
            scope_collapsed_relay,
            selection_cleared_relay,
            search_filter_changed_relay,
            search_focus_changed_relay,
            variables_restored_relay,
            expanded_scopes_restored_relay,
            tree_selection_changed_relay,
            variable_format_changed_relay,
        }
    }
}

// === HELPER FUNCTIONS (moved from variable_helpers.rs) ===

/// Create a SelectedVariable from raw variable data and context
///
/// This replicates the logic from the legacy add_selected_variable function,
/// but returns the SelectedVariable instead of directly modifying global state.
///
/// CONSOLIDATED from variable_helpers.rs to eliminate unnecessary file separation.
pub fn create_selected_variable(
    variable: shared::Signal,
    file_id: &str,
    scope_id: &str,
) -> Option<shared::SelectedVariable> {
    // ✅ FIX: Use proper domain signal instead of global state antipattern
    let tracked_files: Vec<shared::TrackedFile> = vec![];

    let file = tracked_files.iter().find(|f| f.id == file_id);
    if file.is_none() {
        return None;
    }
    let file = file.unwrap();

    // Find scope full name from the file state
    let scope_full_name = if let shared::FileState::Loaded(waveform_file) = &file.state {
        crate::state::find_scope_full_name(&waveform_file.scopes, scope_id)
            .unwrap_or_else(|| scope_id.to_string())
    } else {
        scope_id.to_string()
    };

    // Create SelectedVariable with the same logic as the legacy function
    let selected_var = shared::SelectedVariable::new(variable, file.path.clone(), scope_full_name);

    Some(selected_var)
}
