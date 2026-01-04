//! SelectedVariables domain - simple state with direct methods
//!
//! Uses pure reactive dataflow: external state changes are observed via signals,
//! internal state is updated reactively.

#![allow(dead_code)] // API not yet fully integrated

use indexmap::IndexSet;
use shared::SelectedVariable;
use std::sync::Arc;
use zoon::{Mutable, MutableVec, MutableExt, SignalExt, SignalVecExt, Task, TaskHandle, VecDiff};

#[derive(Clone)]
pub struct SelectedVariables {
    pub variables: MutableVec<SelectedVariable>,
    pub variables_vec_actor: Mutable<Vec<SelectedVariable>>,
    pub selected_scope: Mutable<Option<String>>,
    pub tree_selection: Mutable<IndexSet<String>>,
    pub expanded_scopes: Mutable<IndexSet<String>>,
    pub search_filter: Mutable<String>,
    pub search_focused: Mutable<bool>,
    _scope_selection_observer: Arc<TaskHandle>,
}

impl SelectedVariables {
    pub fn new(files_selected_scope: MutableVec<String>) -> Self {
        let tree_selection = Mutable::new(IndexSet::new());
        let selected_scope = Mutable::new(None);

        // Pure dataflow: observe files_selected_scope and update internal state
        let _scope_selection_observer = {
            let tree_selection_clone = tree_selection.clone();
            let selected_scope_clone = selected_scope.clone();
            let current_selection = std::cell::RefCell::new(Vec::<String>::new());
            Arc::new(Task::start_droppable(
                files_selected_scope
                    .signal_vec_cloned()
                    .for_each(move |diff| {
                        {
                            let mut v = current_selection.borrow_mut();
                            match diff {
                                VecDiff::Replace { values } => *v = values,
                                VecDiff::InsertAt { index, value } => v.insert(index, value),
                                VecDiff::UpdateAt { index, value } => v[index] = value,
                                VecDiff::RemoveAt { index } => { v.remove(index); }
                                VecDiff::Move { old_index, new_index } => {
                                    let item = v.remove(old_index);
                                    v.insert(new_index, item);
                                }
                                VecDiff::Push { value } => v.push(value),
                                VecDiff::Pop {} => { v.pop(); }
                                VecDiff::Clear {} => v.clear(),
                            }
                        }
                        Self::handle_scope_selection(
                            current_selection.borrow().clone(),
                            &tree_selection_clone,
                            &selected_scope_clone,
                        );
                        async {}
                    }),
            ))
        };

        Self {
            variables: MutableVec::new(),
            variables_vec_actor: Mutable::new(vec![]),
            selected_scope,
            tree_selection,
            expanded_scopes: Mutable::new(IndexSet::new()),
            search_filter: Mutable::new(String::new()),
            search_focused: Mutable::new(false),
            _scope_selection_observer,
        }
    }

    fn handle_scope_selection(
        selected_ids: Vec<String>,
        tree_selection: &Mutable<IndexSet<String>>,
        selected_scope: &Mutable<Option<String>>,
    ) {
        let mut selection_set = IndexSet::new();
        let mut last_scope: Option<String> = None;

        for raw_id in selected_ids.into_iter() {
            if !raw_id.starts_with("scope_") {
                continue;
            }
            let cleaned = raw_id
                .strip_prefix("scope_")
                .unwrap_or(raw_id.as_str())
                .to_string();
            last_scope = Some(cleaned);
            selection_set.insert(raw_id);
        }

        selected_scope.set(last_scope);
        tree_selection.set(selection_set);
    }

    pub fn add_variable(&self, variable_id: String) {
        let is_already_selected = {
            let vars = self.variables.lock_ref();
            vars.iter().any(|v| v.unique_id == variable_id)
        };

        if is_already_selected {
            return;
        }

        let placeholder_var = shared::SelectedVariable {
            unique_id: variable_id,
            formatter: None,
        };

        self.variables.lock_mut().push_cloned(placeholder_var);
        self.sync_variables_vec();
    }

    pub fn remove_variable(&self, variable_id: String) {
        self.variables
            .lock_mut()
            .retain(|var| var.unique_id != variable_id);
        self.sync_variables_vec();
    }

    pub fn clear_selection(&self) {
        self.variables.lock_mut().clear();
        self.sync_variables_vec();
    }

    pub fn restore_variables(&self, restored_variables: Vec<SelectedVariable>) {
        self.variables
            .lock_mut()
            .replace_cloned(restored_variables);
        self.sync_variables_vec();
    }

    pub fn change_variable_format(&self, variable_id: String, new_format: shared::VarFormat) {
        let variables = self.variables.lock_ref().to_vec();
        let updated_variables: Vec<_> = variables
            .into_iter()
            .map(|mut var| {
                if var.unique_id == variable_id {
                    var.formatter = Some(new_format);
                }
                var
            })
            .collect();

        self.variables
            .lock_mut()
            .replace_cloned(updated_variables);
        self.sync_variables_vec();
    }

    pub fn select_scope(&self, scope_id: Option<String>) {
        self.selected_scope.set(scope_id);
    }

    pub fn expand_scope(&self, scope_id: String) {
        self.expanded_scopes.update_mut(|scopes| {
            scopes.insert(scope_id);
        });
    }

    pub fn collapse_scope(&self, scope_id: String) {
        self.expanded_scopes.update_mut(|scopes| {
            scopes.shift_remove(&scope_id);
        });
    }

    pub fn restore_expanded_scopes(&self, restored_scopes: IndexSet<String>) {
        self.expanded_scopes.set(restored_scopes);
    }

    pub fn set_tree_selection(&self, selection: IndexSet<String>) {
        self.tree_selection.set(selection.clone());
        let next_scope = selection
            .iter()
            .find(|raw_id| raw_id.starts_with("scope_"))
            .and_then(|raw_id| raw_id.strip_prefix("scope_").map(|clean| clean.to_string()));
        self.selected_scope.set(next_scope);
    }

    pub fn set_search_filter(&self, filter_text: String) {
        self.search_filter.set(filter_text);
    }

    pub fn set_search_focus(&self, focused: bool) {
        self.search_focused.set(focused);
    }

    fn sync_variables_vec(&self) {
        let current_vars = self.variables.lock_ref().to_vec();
        self.variables_vec_actor.set(current_vars);
    }

    pub fn file_variables_signal(
        &self,
        file_path: String,
    ) -> impl zoon::Signal<Item = Vec<SelectedVariable>> {
        self.variables_vec_actor.signal_cloned().map(move |vars| {
            vars.iter()
                .filter(|v| v.file_path().as_ref() == Some(&file_path))
                .cloned()
                .collect()
        })
    }

    pub fn variables_signal(&self) -> impl zoon::Signal<Item = Vec<SelectedVariable>> {
        self.variables_vec_actor.signal_cloned()
    }

    pub fn expanded_scopes_mutable(&self) -> &Mutable<IndexSet<String>> {
        &self.expanded_scopes
    }

    pub fn tree_selection_mutable(&self) -> &Mutable<IndexSet<String>> {
        &self.tree_selection
    }
}

impl SelectedVariables {
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
}

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

pub fn get_variables_from_tracked_files(
    scope_id: &str,
    tracked_files: &[shared::TrackedFile],
) -> Vec<VariableWithContext> {
    let mut variables_with_context = Vec::new();

    for tracked_file in tracked_files.iter() {
        if let shared::FileState::Loaded(waveform_file) = &tracked_file.state {
            if let Some(signals) = shared::find_variables_in_scope(&waveform_file.scopes, scope_id)
            {
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

pub fn find_scope_full_name(scopes: &[shared::ScopeData], target_scope_id: &str) -> Option<String> {
    for scope in scopes {
        if scope.id == target_scope_id {
            return Some(scope.full_name.clone());
        }
        if let Some(name) = find_scope_full_name(&scope.children, target_scope_id) {
            return Some(name);
        }
    }
    None
}
