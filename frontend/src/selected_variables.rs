//! SelectedVariables domain - simple state with direct methods
//!
//! Uses pure reactive dataflow: external state changes are observed via signals,
//! internal state is updated reactively.

use indexmap::IndexSet;
use shared::SelectedVariable;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;
use zoon::{Mutable, MutableExt, MutableVec, SignalExt, SignalVecExt, Task, TaskHandle, VecDiff};

#[derive(Clone, Debug)]
pub struct SignalGroup {
    pub name: String,
    pub member_ids: Vec<String>,
    pub collapsed: Mutable<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectedVariableOrGroup {
    Variable(SelectedVariable),
    GroupHeader {
        index: usize,
        name: String,
        collapsed: bool,
        member_count: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowHeightChange {
    pub seq: u64,
    pub unique_id: String,
    pub old_height: u32,
    pub new_height: u32,
}

#[derive(Clone)]
pub struct SelectedVariables {
    pub variables: MutableVec<SelectedVariable>,
    pub variables_vec_actor: Mutable<Vec<SelectedVariable>>,
    pub row_height_atoms: Mutable<BTreeMap<String, Mutable<u32>>>,
    pub selected_scope: Mutable<Option<String>>,
    pub tree_selection: Mutable<IndexSet<String>>,
    pub expanded_scopes: Mutable<IndexSet<String>>,
    pub search_filter: Mutable<String>,
    pub search_focused: Mutable<bool>,
    pub signal_groups: MutableVec<SignalGroup>,
    pub selected_for_grouping: Mutable<IndexSet<String>>,
    pub grouping_mode_active: Mutable<bool>,
    pub visible_items: Mutable<Vec<SelectedVariableOrGroup>>,
    pub total_content_height: Mutable<u32>,
    pub last_row_height_change: Mutable<Option<RowHeightChange>>,
    row_height_change_seq: Rc<Cell<u64>>,
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
                                VecDiff::RemoveAt { index } => {
                                    v.remove(index);
                                }
                                VecDiff::Move {
                                    old_index,
                                    new_index,
                                } => {
                                    let item = v.remove(old_index);
                                    v.insert(new_index, item);
                                }
                                VecDiff::Push { value } => v.push(value),
                                VecDiff::Pop {} => {
                                    v.pop();
                                }
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
            row_height_atoms: Mutable::new(BTreeMap::new()),
            selected_scope,
            tree_selection,
            expanded_scopes: Mutable::new(IndexSet::new()),
            search_filter: Mutable::new(String::new()),
            search_focused: Mutable::new(false),
            signal_groups: MutableVec::new(),
            selected_for_grouping: Mutable::new(IndexSet::new()),
            grouping_mode_active: Mutable::new(false),
            visible_items: Mutable::new(Vec::new()),
            total_content_height: Mutable::new(2 * 30),
            last_row_height_change: Mutable::new(None),
            row_height_change_seq: Rc::new(Cell::new(0)),
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
            signal_type: None,
            row_height: None,
            analog_limits: None,
        };

        self.variables.lock_mut().push_cloned(placeholder_var);
        self.sync_variables_vec();
    }

    pub fn remove_variable(&self, variable_id: String) {
        self.variables
            .lock_mut()
            .retain(|var| var.unique_id != variable_id);
        self.remove_variable_from_groups(&variable_id);
        self.selected_for_grouping.update_mut(|selection| {
            selection.shift_remove(&variable_id);
        });
        self.sync_variables_vec();
    }

    pub fn clear_selection(&self) {
        self.variables.lock_mut().clear();
        self.signal_groups.lock_mut().clear();
        self.selected_for_grouping.set(IndexSet::new());
        self.grouping_mode_active.set(false);
        self.sync_variables_vec();
    }

    pub fn restore_variables(&self, restored_variables: Vec<SelectedVariable>) {
        self.variables.lock_mut().replace_cloned(restored_variables);
        self.prune_group_memberships();
        self.sync_variables_vec();
    }

    pub fn change_variable_format(&self, variable_id: String, new_format: shared::VarFormat) {
        self.update_variable_with_visibility_refresh(&variable_id, true, |var| {
            var.formatter = Some(new_format);
        });
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
        self.sync_row_heights(&current_vars);
        self.variables_vec_actor.set_neq(current_vars);
        self.refresh_visible_items();
    }

    fn sync_variables_vec_without_refresh(&self) {
        let current_vars = self.variables.lock_ref().to_vec();
        self.sync_row_heights(&current_vars);
        self.variables_vec_actor.set_neq(current_vars);
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

    pub fn toggle_variable_selection(&self, unique_id: &str) {
        self.selected_for_grouping.update_mut(|set| {
            if set.contains(unique_id) {
                set.shift_remove(unique_id);
            } else {
                set.insert(unique_id.to_string());
            }
        });
    }

    pub fn update_row_height(&self, unique_id: &str, row_height: u32) {
        self.update_variable_with_visibility_refresh(unique_id, false, |var| {
            var.row_height = Some(row_height);
        });
    }

    pub fn set_live_row_height(&self, unique_id: &str, row_height: u32) {
        let atom = self.ensure_row_height_atom(unique_id, row_height);
        let old_height = atom.get_cloned();
        if old_height == row_height {
            return;
        }

        atom.set_neq(row_height);
        self.adjust_total_content_height(unique_id, old_height, row_height);

        let next_seq = self.row_height_change_seq.get().saturating_add(1);
        self.row_height_change_seq.set(next_seq);
        self.last_row_height_change.set(Some(RowHeightChange {
            seq: next_seq,
            unique_id: unique_id.to_string(),
            old_height,
            new_height: row_height,
        }));
    }

    pub fn live_row_height(&self, unique_id: &str) -> u32 {
        self.row_height_atoms
            .get_cloned()
            .get(unique_id)
            .map(|atom| atom.get_cloned())
            .unwrap_or_else(|| self.committed_row_height(unique_id))
    }

    pub fn live_row_height_signal(
        &self,
        unique_id: String,
        fallback: u32,
    ) -> impl zoon::Signal<Item = u32> + 'static {
        self.ensure_row_height_atom(&unique_id, fallback).signal()
    }

    pub fn committed_row_height(&self, unique_id: &str) -> u32 {
        self.variables_vec_actor
            .get_cloned()
            .iter()
            .find(|variable| variable.unique_id == unique_id)
            .map(Self::row_height_for_variable)
            .unwrap_or(30)
    }

    pub fn commit_live_row_height(&self, unique_id: &str) -> bool {
        let live_row_height = self.live_row_height(unique_id);
        let committed_row_height = self.committed_row_height(unique_id);
        if live_row_height == committed_row_height {
            return false;
        }

        self.update_row_height(unique_id, live_row_height);
        true
    }

    pub fn update_analog_limits(
        &self,
        unique_id: &str,
        analog_limits: Option<shared::AnalogLimits>,
    ) {
        self.update_variable_with_visibility_refresh(unique_id, true, |var| {
            var.analog_limits = analog_limits.clone();
        });
    }

    pub fn synchronize_metadata_from_files(&self, files: &[shared::TrackedFile]) {
        let mut updated_any = false;
        let mut current = {
            let variables = self.variables.lock_ref();
            variables.to_vec()
        };

        for selected in &mut current {
            let Some((file_path, scope_path, variable_name)) = selected.parse_unique_id() else {
                continue;
            };
            let Some(signal) = lookup_signal(files, &file_path, &scope_path, &variable_name) else {
                continue;
            };

            let default_row_height =
                shared::SelectedVariable::default_row_height_for_signal_type(&signal.signal_type);
            let is_real = signal.signal_type == "Real";

            if selected.signal_type.as_deref() != Some(signal.signal_type.as_str()) {
                selected.signal_type = Some(signal.signal_type.clone());
                updated_any = true;
            }
            if selected.row_height.is_none() {
                selected.row_height = Some(default_row_height);
                updated_any = true;
            }
            if is_real && selected.analog_limits.is_none() {
                selected.analog_limits = Some(shared::AnalogLimits::auto());
                updated_any = true;
            }
            if !is_real && selected.analog_limits.is_some() {
                selected.analog_limits = None;
                updated_any = true;
            }
        }

        if updated_any {
            self.variables.lock_mut().replace_cloned(current);
            self.sync_variables_vec();
        }
    }

    pub fn create_group(&self, name: String) {
        let selected = self.selected_for_grouping.get_cloned();
        self.create_group_from_members(name, selected.into_iter().collect());
    }

    pub fn create_group_from_members(&self, name: String, member_ids: Vec<String>) {
        let selected: IndexSet<String> = member_ids.into_iter().collect();
        if selected.len() < 2 {
            return;
        }
        let member_ids: Vec<String> = selected.into_iter().collect();
        self.remove_members_from_existing_groups(&member_ids);
        self.signal_groups.lock_mut().push_cloned(SignalGroup {
            name,
            member_ids: member_ids.clone(),
            collapsed: Mutable::new(false),
        });
        self.selected_for_grouping.set(IndexSet::new());
        self.grouping_mode_active.set(false);
        self.prune_group_memberships();
        self.refresh_visible_items();
    }

    pub fn ungroup(&self, index: usize) {
        let mut lock = self.signal_groups.lock_mut();
        if index < lock.len() {
            lock.remove(index);
        }
        drop(lock);
        self.refresh_visible_items();
    }

    pub fn rename_group(&self, index: usize, name: String) {
        let mut lock = self.signal_groups.lock_mut();
        if let Some(group) = lock.get(index) {
            let updated = SignalGroup {
                name,
                member_ids: group.member_ids.clone(),
                collapsed: Mutable::new(group.collapsed.get()),
            };
            lock.set_cloned(index, updated);
        }
        drop(lock);
        self.refresh_visible_items();
    }

    pub fn toggle_group_collapse(&self, index: usize) {
        let lock = self.signal_groups.lock_ref();
        if let Some(group) = lock.get(index) {
            let current = group.collapsed.get();
            group.collapsed.set(!current);
        }
        drop(lock);
        self.refresh_visible_items();
    }

    pub fn refresh_visible_items(&self) {
        let vars = self.variables.lock_ref().to_vec();
        let groups = self.signal_groups.lock_ref().to_vec();
        let items = Self::compute_visible_items(&vars, &groups);
        self.visible_items.set_neq(items.clone());
        self.total_content_height
            .set_neq(self.total_content_height_for_items(&items));
    }

    fn compute_visible_items(
        vars: &[SelectedVariable],
        groups: &[SignalGroup],
    ) -> Vec<SelectedVariableOrGroup> {
        let mut items = Vec::new();
        let mut grouped_ids: IndexSet<String> = IndexSet::new();

        for group in groups {
            for id in &group.member_ids {
                grouped_ids.insert(id.clone());
            }
        }

        let mut var_index = 0;
        let mut group_positions: Vec<(usize, usize)> = Vec::new();

        for (gi, group) in groups.iter().enumerate() {
            let first_member_pos = vars
                .iter()
                .position(|v| group.member_ids.contains(&v.unique_id));
            if let Some(pos) = first_member_pos {
                group_positions.push((pos, gi));
            }
        }
        group_positions.sort_by_key(|&(pos, _)| pos);

        let mut emitted_groups: IndexSet<usize> = IndexSet::new();

        for var in vars {
            for &(pos, gi) in &group_positions {
                if pos == var_index && !emitted_groups.contains(&gi) {
                    emitted_groups.insert(gi);
                    let group = &groups[gi];
                    let collapsed = group.collapsed.get();
                    items.push(SelectedVariableOrGroup::GroupHeader {
                        index: gi,
                        name: group.name.clone(),
                        collapsed,
                        member_count: group.member_ids.len(),
                    });
                    if !collapsed {
                        for member_id in &group.member_ids {
                            if let Some(member_var) =
                                vars.iter().find(|v| &v.unique_id == member_id)
                            {
                                items.push(SelectedVariableOrGroup::Variable(member_var.clone()));
                            }
                        }
                    }
                }
            }

            if !grouped_ids.contains(&var.unique_id) {
                items.push(SelectedVariableOrGroup::Variable(var.clone()));
            }
            var_index += 1;
        }

        items
    }

    pub fn collapsed_variable_ids(&self) -> IndexSet<String> {
        let groups = self.signal_groups.lock_ref();
        let mut collapsed_ids = IndexSet::new();
        for group in groups.iter() {
            if group.collapsed.get() {
                for id in &group.member_ids {
                    collapsed_ids.insert(id.clone());
                }
            }
        }
        collapsed_ids
    }

    pub fn restore_signal_groups(&self, configs: Vec<shared::SignalGroupConfig>) {
        let groups: Vec<SignalGroup> = configs
            .into_iter()
            .map(|c| SignalGroup {
                name: c.name,
                member_ids: c.member_ids,
                collapsed: Mutable::new(c.collapsed),
            })
            .collect();
        self.signal_groups.lock_mut().replace_cloned(groups);
        self.prune_group_memberships();
        self.refresh_visible_items();
    }

    pub fn signal_groups_as_config(&self) -> Vec<shared::SignalGroupConfig> {
        self.signal_groups
            .lock_ref()
            .iter()
            .map(|g| shared::SignalGroupConfig {
                name: g.name.clone(),
                member_ids: g.member_ids.clone(),
                collapsed: g.collapsed.get(),
            })
            .collect()
    }

    fn update_variable_with_visibility_refresh<F>(
        &self,
        unique_id: &str,
        refresh_visible_items: bool,
        mut f: F,
    ) where
        F: FnMut(&mut SelectedVariable),
    {
        let mut current = {
            let variables = self.variables.lock_ref();
            variables.to_vec()
        };
        let mut changed = false;
        for variable in &mut current {
            if variable.unique_id == unique_id {
                f(variable);
                changed = true;
                break;
            }
        }
        if changed {
            self.variables.lock_mut().replace_cloned(current);
            if refresh_visible_items {
                self.sync_variables_vec();
            } else {
                self.sync_variables_vec_without_refresh();
            }
        }
    }

    fn sync_row_heights(&self, variables: &[SelectedVariable]) {
        let mut next = self.row_height_atoms.get_cloned();
        let mut desired_ids = IndexSet::new();
        for variable in variables {
            desired_ids.insert(variable.unique_id.clone());
        }

        next.retain(|unique_id, _| desired_ids.contains(unique_id));

        for variable in variables {
            let next_height = Self::row_height_for_variable(variable);
            if let Some(atom) = next.get(&variable.unique_id) {
                atom.set_neq(next_height);
            } else {
                next.insert(variable.unique_id.clone(), Mutable::new(next_height));
            }
        }
        self.row_height_atoms.set(next);
    }

    fn row_height_for_variable(variable: &SelectedVariable) -> u32 {
        variable.row_height.unwrap_or_else(|| {
            variable
                .signal_type
                .as_deref()
                .map(shared::SelectedVariable::default_row_height_for_signal_type)
                .unwrap_or(30)
        })
    }

    fn remove_members_from_existing_groups(&self, member_ids: &[String]) {
        let selected: IndexSet<String> = member_ids.iter().cloned().collect();
        let groups = self.signal_groups.lock_ref().to_vec();
        let mut rebuilt = Vec::new();

        for group in groups {
            let remaining: Vec<String> = group
                .member_ids
                .into_iter()
                .filter(|id| !selected.contains(id))
                .collect();
            if remaining.len() >= 2 {
                rebuilt.push(SignalGroup {
                    name: group.name,
                    member_ids: remaining,
                    collapsed: Mutable::new(group.collapsed.get()),
                });
            }
        }

        self.signal_groups.lock_mut().replace_cloned(rebuilt);
    }

    fn remove_variable_from_groups(&self, unique_id: &str) {
        let groups = self.signal_groups.lock_ref().to_vec();
        let mut rebuilt = Vec::new();

        for group in groups {
            let remaining: Vec<String> = group
                .member_ids
                .into_iter()
                .filter(|member| member != unique_id)
                .collect();
            if remaining.len() >= 2 {
                rebuilt.push(SignalGroup {
                    name: group.name,
                    member_ids: remaining,
                    collapsed: Mutable::new(group.collapsed.get()),
                });
            }
        }

        self.signal_groups.lock_mut().replace_cloned(rebuilt);
    }

    fn ensure_row_height_atom(&self, unique_id: &str, fallback: u32) -> Mutable<u32> {
        if let Some(atom) = self.row_height_atoms.get_cloned().get(unique_id).cloned() {
            return atom;
        }

        let mut atoms = self.row_height_atoms.get_cloned();
        let atom = Mutable::new(fallback);
        atoms.insert(unique_id.to_string(), atom.clone());
        self.row_height_atoms.set(atoms);
        atom
    }

    fn total_content_height_for_items(&self, items: &[SelectedVariableOrGroup]) -> u32 {
        if items.is_empty() {
            return 2 * 30;
        }

        let variable_count = items
            .iter()
            .filter(|item| matches!(item, SelectedVariableOrGroup::Variable(_)))
            .count() as u32;
        let item_heights: u32 = items
            .iter()
            .map(|item| match item {
                SelectedVariableOrGroup::Variable(variable) => {
                    self.live_row_height(&variable.unique_id)
                }
                SelectedVariableOrGroup::GroupHeader { .. } => 30,
            })
            .sum();

        item_heights + variable_count * 3 + 30
    }

    fn adjust_total_content_height(&self, unique_id: &str, old_height: u32, new_height: u32) {
        let is_visible = self.visible_items.get_cloned().iter().any(|item| {
            matches!(
                item,
                SelectedVariableOrGroup::Variable(variable) if variable.unique_id == unique_id
            )
        });

        if !is_visible {
            return;
        }

        let delta = new_height as i64 - old_height as i64;
        self.total_content_height.update_mut(|height| {
            if delta >= 0 {
                *height = height.saturating_add(delta as u32);
            } else {
                *height = height.saturating_sub(delta.unsigned_abs() as u32);
            }
        });
    }

    fn prune_group_memberships(&self) {
        let variables: IndexSet<String> = self
            .variables
            .lock_ref()
            .iter()
            .map(|var| var.unique_id.clone())
            .collect();
        let mut seen = IndexSet::new();
        let groups = self.signal_groups.lock_ref().to_vec();
        let mut rebuilt = Vec::new();

        for group in groups {
            let mut member_ids = Vec::new();
            for id in group.member_ids {
                if variables.contains(&id) && seen.insert(id.clone()) {
                    member_ids.push(id);
                }
            }

            if member_ids.len() >= 2 {
                rebuilt.push(SignalGroup {
                    name: group.name,
                    member_ids,
                    collapsed: Mutable::new(group.collapsed.get()),
                });
            }
        }

        self.signal_groups.lock_mut().replace_cloned(rebuilt);
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

fn lookup_signal(
    tracked_files: &[shared::TrackedFile],
    file_path: &str,
    scope_path: &str,
    variable_name: &str,
) -> Option<shared::Signal> {
    let full_scope_id = format!("{file_path}|{scope_path}");
    tracked_files.iter().find_map(|tracked_file| {
        if tracked_file.canonical_path != file_path && tracked_file.path != file_path {
            return None;
        }
        let shared::FileState::Loaded(waveform_file) = &tracked_file.state else {
            return None;
        };
        shared::find_variables_in_scope(&waveform_file.scopes, &full_scope_id).and_then(|signals| {
            signals
                .into_iter()
                .find(|signal| signal.name == variable_name)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use zoon::MutableVec;

    #[test]
    fn regrouping_enforces_single_group_membership() {
        let selected_variables = SelectedVariables::new(MutableVec::new());
        selected_variables.add_variable("file|scope|a".to_string());
        selected_variables.add_variable("file|scope|b".to_string());
        selected_variables.add_variable("file|scope|c".to_string());

        selected_variables.create_group_from_members(
            "Group 1".to_string(),
            vec!["file|scope|a".to_string(), "file|scope|b".to_string()],
        );
        selected_variables.create_group_from_members(
            "Group 2".to_string(),
            vec!["file|scope|b".to_string(), "file|scope|c".to_string()],
        );

        let groups = selected_variables.signal_groups.lock_ref().to_vec();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "Group 2");
        assert_eq!(
            groups[0].member_ids,
            vec!["file|scope|b".to_string(), "file|scope|c".to_string()]
        );
    }

    #[test]
    fn removing_variable_drops_small_groups() {
        let selected_variables = SelectedVariables::new(MutableVec::new());
        selected_variables.add_variable("file|scope|a".to_string());
        selected_variables.add_variable("file|scope|b".to_string());
        selected_variables.create_group_from_members(
            "Group 1".to_string(),
            vec!["file|scope|a".to_string(), "file|scope|b".to_string()],
        );

        selected_variables.remove_variable("file|scope|a".to_string());

        assert!(selected_variables.signal_groups.lock_ref().is_empty());
    }
}
