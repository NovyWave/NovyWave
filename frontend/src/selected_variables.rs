//! SelectedVariables domain using Actor+Relay architecture

#![allow(dead_code)] // Actor+Relay API not yet fully integrated

use crate::dataflow::{Actor, ActorVec, Relay, relay};
use futures::StreamExt;
use indexmap::IndexSet;
use shared::SelectedVariable;
use zoon::{MutableExt, SignalExt};
#[derive(Clone, Debug)]
pub struct SelectedVariables {
    // State actors
    pub variables: ActorVec<SelectedVariable>,
    pub variables_vec_actor: Actor<Vec<SelectedVariable>>,
    pub selected_scope: Actor<Option<String>>,
    pub tree_selection: Actor<IndexSet<String>>,
    pub expanded_scopes: Actor<IndexSet<String>>,
    pub search_filter: Actor<String>,
    pub search_focused: Actor<bool>,

    // Event relays
    pub variable_clicked_relay: Relay<String>,
    pub variable_removed_relay: Relay<String>,
    pub scope_selected_relay: Relay<Option<String>>,
    pub scope_expanded_relay: Relay<String>,
    pub scope_collapsed_relay: Relay<String>,
    pub selection_cleared_relay: Relay<()>,
    pub search_filter_changed_relay: Relay<String>,
    pub search_focus_changed_relay: Relay<bool>,
    pub variables_restored_relay: Relay<Vec<SelectedVariable>>,
    pub expanded_scopes_restored_relay: Relay<IndexSet<String>>,
    pub tree_selection_changed_relay: Relay<IndexSet<String>>,
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
        let tree_selection_for_scope_stream = tree_selection_changed_relay.subscribe();
        let (variable_format_changed_relay, variable_format_changed_stream) =
            relay::<(String, shared::VarFormat)>();

        // Create dedicated Vec Actor that syncs with ActorVec changes (no conversion antipattern)
        let (variables_vec_updated_relay, variables_vec_updated_stream) =
            relay::<Vec<SelectedVariable>>();
        let variables_vec_actor = Actor::new(vec![], async move |state| {
            let mut vec_updates = variables_vec_updated_stream;
            while let Some(new_vec) = vec_updates.next().await {
                state.set(new_vec);
            }
        });

        // Create comprehensive variables actor with complete event handling
        let variables = ActorVec::new(vec![], {
            let variables_vec_relay_clone = variables_vec_updated_relay.clone();
            async move |variables| {
                let mut variable_clicked = variable_clicked_stream;
                let mut variable_removed = variable_removed_stream;
                let mut selection_cleared = selection_cleared_stream;
                let mut variables_restored = variables_restored_stream;
                let mut variable_format_changed = variable_format_changed_stream;

                loop {
                    use futures::select;

                    select! {
                        variable_id = variable_clicked.next() => {
                            if let Some(variable_id) = variable_id {
                                Self::handle_variable_clicked(
                                    &variables,
                                    &variables_vec_relay_clone,
                                    variable_id,
                                )
                                .await;
                            } else {
                                break; // Stream closed
                            }
                        }
                        variable_id = variable_removed.next() => {
                            if let Some(variable_id) = variable_id {
                                Self::handle_variable_removed(
                                    &variables,
                                    &variables_vec_relay_clone,
                                    variable_id,
                                )
                                .await;
                            } else {
                                break; // Stream closed
                            }
                        }
                        _cleared = selection_cleared.next() => {
                            Self::handle_selection_cleared(&variables, &variables_vec_relay_clone)
                                .await;
                        }
                        restored_vars = variables_restored.next() => {
                            if let Some(restored_vars) = restored_vars {
                                Self::handle_variables_restored(
                                    &variables,
                                    &variables_vec_relay_clone,
                                    restored_vars,
                                )
                                .await;
                            } else {
                                break; // Stream closed
                            }
                        }
                        format_change = variable_format_changed.next() => {
                            if let Some((variable_id, new_format)) = format_change {
                                Self::handle_variable_format_changed(
                                    &variables,
                                    &variables_vec_relay_clone,
                                    variable_id,
                                    new_format,
                                )
                                .await;
                            } else {
                                break; // Stream closed
                            }
                        }
                    }
                }
            }
        });

        // Create selected scope actor
        let selected_scope = Actor::new(None, async move |scope_handle| {
            let mut scope_selected = scope_selected_stream;
            let mut tree_selection_updates = tree_selection_for_scope_stream;

            loop {
                use futures::select;

                select! {
                    scope_id = scope_selected.next() => {
                        if let Some(scope_id) = scope_id {
                            let _ = &scope_handle;
                            scope_handle.set(scope_id);
                        } else {
                            break; // Stream closed
                        }
                    }
                    selection = tree_selection_updates.next() => {
                        if let Some(selection) = selection {
                            let next_scope = selection
                                .iter()
                                .find(|raw_id| raw_id.starts_with("scope_"))
                                .and_then(|raw_id| raw_id
                                    .strip_prefix("scope_")
                                    .map(|clean| clean.to_string()));
                            scope_handle.set(next_scope);
                        } else {
                            break; // Stream closed
                        }
                    }
                }
            }
        });

        // Create tree selection actor for UI state
        let tree_selection = Actor::new(IndexSet::new(), async move |tree_handle| {
            let mut tree_selection_changed = tree_selection_changed_stream;

            while let Some(selection) = tree_selection_changed.next().await {
                tree_handle.set(selection);
            }
        });

        // Create expanded scopes actor
        let expanded_scopes = Actor::new(IndexSet::new(), async move |scopes_handle| {
            let mut scope_expanded = scope_expanded_stream;
            let mut scope_collapsed = scope_collapsed_stream;
            let mut expanded_scopes_restored = expanded_scopes_restored_stream;

            loop {
                use futures::select;

                select! {
                    scope_id = scope_expanded.next() => {
                        if let Some(scope_id) = scope_id {
                            // Individual scope expansion
                            scopes_handle.update_mut(|scopes| {
                                scopes.insert(scope_id);
                            });
                        } else {
                            break; // Stream closed
                        }
                    }
                    scope_id = scope_collapsed.next() => {
                        if let Some(scope_id) = scope_id {
                            // Individual scope collapse
                            scopes_handle.update_mut(|scopes| {
                                scopes.shift_remove(&scope_id);
                            });
                        } else {
                            break; // Stream closed
                        }
                    }
                    restored_scopes = expanded_scopes_restored.next() => {
                        if let Some(restored_scopes) = restored_scopes {
                            // Bulk config restoration
                            scopes_handle.set(restored_scopes);
                        } else {
                            break; // Stream closed
                        }
                    }
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

        // Note: variables_vec_actor gets updated through relay events in proper Actor+Relay pattern

        Self {
            // State actors
            variables,
            variables_vec_actor,
            selected_scope,
            tree_selection,
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

    pub fn file_variables_signal(
        &self,
        file_path: String,
    ) -> impl zoon::Signal<Item = Vec<SelectedVariable>> {
        self.variables_vec_actor.signal().map(move |vars| {
            vars.iter()
                .filter(|v| v.file_path().as_ref() == Some(&file_path))
                .cloned()
                .collect()
        })
    }

    /// Get signal for all selected variables
    pub fn variables_signal(&self) -> impl zoon::Signal<Item = Vec<SelectedVariable>> {
        self.variables_vec_actor.signal()
    }

    pub fn expanded_scopes_actor(&self) -> &Actor<IndexSet<String>> {
        &self.expanded_scopes
    }

    pub fn tree_selection_actor(&self) -> &Actor<IndexSet<String>> {
        &self.tree_selection
    }
}

impl SelectedVariables {
    async fn handle_variable_clicked(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        variables_vec_relay: &Relay<Vec<SelectedVariable>>,
        variable_id: String,
    ) {
        let is_already_selected = {
            let vars = variables_mutable.lock_ref();
            vars.iter().any(|v| v.unique_id == variable_id)
        };

        if is_already_selected {
            return;
        }
        let placeholder_var = shared::SelectedVariable {
            unique_id: variable_id,
            formatter: None,
        };

        variables_mutable.lock_mut().push_cloned(placeholder_var);

        let current_vars = variables_mutable.lock_ref().to_vec();
        variables_vec_relay.send(current_vars);
    }

    #[allow(dead_code)]
    async fn find_and_create_selected_variable(
        _variable_id: &str,
        _scope_id: &str,
    ) -> Option<SelectedVariable> {
        None
    }

    fn find_signal_in_scopes(
        scopes: &[shared::ScopeData],
        signal_name: &str,
    ) -> Option<(shared::Signal, String)> {
        for scope in scopes {
            for signal in &scope.variables {
                if signal.name == signal_name {
                    return Some((signal.clone(), scope.id.clone()));
                }
            }
            if let Some(result) = Self::find_signal_in_scopes(&scope.children, signal_name) {
                return Some(result);
            }
        }
        None
    }

    async fn handle_variable_removed(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        variables_vec_relay: &Relay<Vec<SelectedVariable>>,
        variable_id: String,
    ) {
        variables_mutable
            .lock_mut()
            .retain(|var| var.unique_id != variable_id);
        let current_vars = variables_mutable.lock_ref().to_vec();
        variables_vec_relay.send(current_vars);
    }

    async fn handle_selection_cleared(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        variables_vec_relay: &Relay<Vec<SelectedVariable>>,
    ) {
        variables_mutable.lock_mut().clear();
        let current_vars = variables_mutable.lock_ref().to_vec();
        variables_vec_relay.send(current_vars);
    }

    async fn handle_variables_restored(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        variables_vec_relay: &Relay<Vec<SelectedVariable>>,
        restored_variables: Vec<SelectedVariable>,
    ) {
        variables_mutable
            .lock_mut()
            .replace_cloned(restored_variables);
        let current_vars = variables_mutable.lock_ref().to_vec();
        variables_vec_relay.send(current_vars);
    }

    async fn handle_variable_format_changed(
        variables_mutable: &zoon::MutableVec<SelectedVariable>,
        variables_vec_relay: &Relay<Vec<SelectedVariable>>,
        variable_id: String,
        new_format: shared::VarFormat,
    ) {
        let variables = variables_mutable.lock_ref().to_vec();
        let updated_variables: Vec<_> = variables
            .into_iter()
            .map(|mut var| {
                if var.unique_id == variable_id {
                    var.formatter = Some(new_format);
                }
                var
            })
            .collect();

        variables_mutable
            .lock_mut()
            .replace_cloned(updated_variables);
        let current_vars = variables_mutable.lock_ref().to_vec();
        variables_vec_relay.send(current_vars);
    }
}

// Variable context types and functions

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

pub fn create_selected_variable(
    variable: shared::Signal,
    file_id: &str,
    scope_id: &str,
) -> Option<shared::SelectedVariable> {
    if file_id.is_empty() {
        return None;
    }

    let scope_full_name = scope_id
        .split_once('|')
        .map(|(_, scope)| scope.to_string())
        .unwrap_or_else(|| scope_id.to_string());

    Some(shared::SelectedVariable::new(
        variable,
        file_id.to_string(),
        scope_full_name,
    ))
}

/// Get variables from tracked files for a specific scope
pub fn get_variables_from_tracked_files(
    scope_id: &str,
    tracked_files: &[shared::TrackedFile],
) -> Vec<VariableWithContext> {
    let mut variables_with_context = Vec::new();

    // Iterate through all loaded waveform files
    for tracked_file in tracked_files.iter() {
        if let shared::FileState::Loaded(waveform_file) = &tracked_file.state {
            // Use existing utility function to find variables in the specific scope
            if let Some(signals) = shared::find_variables_in_scope(&waveform_file.scopes, scope_id)
            {
                // Convert each signal to VariableWithContext
                for signal in signals {
                    variables_with_context.push(VariableWithContext {
                        signal,
                        file_id: tracked_file.canonical_path.clone(),
                        scope_id: scope_id.to_string(),
                    });
                }
            }
        }
    }

    variables_with_context
}

/// Helper function to find scope full name in the file structure
pub fn find_scope_full_name(scopes: &[shared::ScopeData], target_scope_id: &str) -> Option<String> {
    for scope in scopes {
        if scope.id == target_scope_id {
            return Some(scope.full_name.clone());
        }
        // Recursively search children
        if let Some(name) = find_scope_full_name(&scope.children, target_scope_id) {
            return Some(name);
        }
    }
    None
}
