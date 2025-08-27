# NovyWave Variable Selection with Actor+Relay

## Waveform Variable Selection Pattern

This example shows how NovyWave manages variable selection for waveform display using Actor+Relay architecture, replacing the global `SELECTED_VARIABLES` pattern.

## Domain-Specific Types

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct SelectedVariable {
    pub id: String,
    pub name: String,
    pub scope: String,
    pub file_id: String,
    pub bit_width: u32,
    pub display_format: SignalFormat,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SignalFormat {
    Binary,
    Decimal,
    Hexadecimal,
    ASCII,
}

#[derive(Clone, Debug)]
pub struct ScopeHierarchy {
    pub name: String,
    pub children: Vec<ScopeHierarchy>,
    pub variables: Vec<VariableInfo>,
}

#[derive(Clone, Debug)]
pub struct VariableInfo {
    pub name: String,
    pub bit_width: u32,
    pub variable_type: String, // reg, wire, etc.
}
```

## Variable Selection Manager

```rust
#[derive(Clone, Debug)]
pub struct VariableSelection {
    pub variables: ActorVec<SelectedVariable>,
    
    // Selection operations
    pub add_variable: Relay<SelectedVariable>,
    pub remove_variable: Relay<String>, // variable ID
    pub clear_selection: Relay,
    pub toggle_variable: Relay<SelectedVariable>,
    pub update_format: Relay<(String, SignalFormat)>, // (variable_id, format)
    
    // Batch operations for scope selection
    pub select_scope_variables: Relay<(String, String)>, // (file_id, scope_path)
    pub deselect_scope_variables: Relay<(String, String)>,
}

impl VariableSelection {
    pub fn new() -> Self {
        let (add_variable, mut add_stream) = relay();
        let (remove_variable, mut remove_stream) = relay();
        let (clear_selection, mut clear_stream) = relay();
        let (toggle_variable, mut toggle_stream) = relay();
        let (update_format, mut format_stream) = relay();
        let (select_scope_variables, mut select_scope_stream) = relay();
        let (deselect_scope_variables, mut deselect_scope_stream) = relay();
        
        let variables = ActorVec::new(vec![], async move |vars_vec| {
            loop {
                select! {
                    Some(var) = add_stream.next() => {
                        // Add if not already present
                        let mut vars = vars_vec.lock_mut();
                        if !vars.iter().any(|v| v.id == var.id) {
                            vars.push_cloned(var);
                        }
                    }
                    Some(var_id) = remove_stream.next() => {
                        vars_vec.lock_mut().retain(|v| v.id != var_id);
                    }
                    Some(()) = clear_stream.next() => {
                        vars_vec.lock_mut().clear();
                    }
                    Some(var) = toggle_stream.next() => {
                        let mut vars = vars_vec.lock_mut();
                        if let Some(pos) = vars.iter().position(|v| v.id == var.id) {
                            vars.remove(pos);
                        } else {
                            vars.push_cloned(var);
                        }
                    }
                    Some((var_id, format)) = format_stream.next() => {
                        let mut vars = vars_vec.lock_mut();
                        if let Some(var) = vars.iter_mut().find(|v| v.id == var_id) {
                            var.display_format = format;
                        }
                    }
                    Some((file_id, scope_path)) = select_scope_stream.next() => {
                        // Get all variables in scope from file manager
                        let scope_vars = get_scope_variables(&file_id, &scope_path);
                        let mut vars = vars_vec.lock_mut();
                        for var in scope_vars {
                            if !vars.iter().any(|v| v.id == var.id) {
                                vars.push_cloned(var);
                            }
                        }
                    }
                    Some((file_id, scope_path)) = deselect_scope_stream.next() => {
                        vars_vec.lock_mut().retain(|v| {
                            !(v.file_id == file_id && v.scope.starts_with(&scope_path))
                        });
                    }
                }
            }
        });
        
        VariableSelection {
            variables,
            add_variable,
            remove_variable,
            clear_selection,
            toggle_variable,
            update_format,
            select_scope_variables,
            deselect_scope_variables,
        }
    }
    
    // Derived signals for UI
    pub fn selection_count_signal(&self) -> impl Signal<Item = usize> {
        self.variables.signal_vec_cloned()
            .map(|vars| vars.len())
    }
    
    pub fn has_variables_signal(&self) -> impl Signal<Item = bool> {
        self.variables.signal_vec_cloned()
            .map(|vars| !vars.is_empty())
    }
    
    pub fn variables_by_file_signal(&self) -> impl Signal<Item = BTreeMap<String, Vec<SelectedVariable>>> {
        self.variables.signal_vec_cloned()
            .map(|vars| {
                let mut by_file = BTreeMap::new();
                for var in vars.iter() {
                    by_file.entry(var.file_id.clone())
                        .or_insert_with(Vec::new)
                        .push(var.clone());
                }
                by_file
            })
    }
}

// Helper function to get variables from a specific scope
fn get_scope_variables(file_id: &str, scope_path: &str) -> Vec<SelectedVariable> {
    // In real implementation, this would query the file manager
    // for variables in the specified scope
    vec![] // Placeholder
}
```

## Integration with Timeline for Cursor Values

```rust
#[derive(Clone, Debug)]
pub struct VariableValueTracker {
    pub variable_values: Actor<BTreeMap<String, String>>,
    
    // Value update events
    pub cursor_position_changed: Relay<f64>,
    pub variables_changed: Relay<Vec<SelectedVariable>>,
}

impl VariableValueTracker {
    pub fn new(variable_selection: &VariableSelection) -> Self {
        let (cursor_position_changed, mut cursor_stream) = relay();
        let (variables_changed, mut vars_stream) = relay();
        
        let variable_values = Actor::new(BTreeMap::new(), async move |values_map| {
            let mut current_cursor = 0.0;
            let mut current_variables = Vec::new();
            
            loop {
                select! {
                    Some(cursor_pos) = cursor_stream.next() => {
                        current_cursor = cursor_pos;
                        let new_values = query_variable_values_at_time(&current_variables, current_cursor).await;
                        values_map.set_neq(new_values);
                    }
                    Some(variables) = vars_stream.next() => {
                        current_variables = variables;
                        let new_values = query_variable_values_at_time(&current_variables, current_cursor).await;
                        values_map.set_neq(new_values);
                    }
                }
            }
        });
        
        // Connect to variable selection changes
        let vars_relay = variables_changed.clone();
        Task::start(
            variable_selection.variables.signal_vec_cloned()
                .for_each(move |vars| {
                    let vars_relay = vars_relay.clone();
                    async move {
                        vars_relay.send(vars.clone());
                    }
                })
        );
        
        VariableValueTracker {
            variable_values,
            cursor_position_changed,
            variables_changed,
        }
    }
}

async fn query_variable_values_at_time(
    variables: &[SelectedVariable], 
    time: f64
) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    
    for var in variables {
        // Query the variable's value at the specified time
        // This would involve looking up transitions in the waveform data
        let value = format!("value_at_{}s", time); // Placeholder
        values.insert(var.id.clone(), value);
    }
    
    values
}
```

## Scope Tree Integration

```rust
#[derive(Clone, Debug)]
pub struct ScopeTreeManager {
    pub expanded_scopes: Actor<HashSet<String>>,
    pub selected_scopes: Actor<HashSet<String>>,
    
    // Scope operations
    pub expand_scope: Relay<String>,
    pub collapse_scope: Relay<String>,
    pub toggle_scope_expansion: Relay<String>,
    pub select_scope: Relay<String>,
}

impl ScopeTreeManager {
    pub fn new() -> Self {
        let (expand_scope, mut expand_stream) = relay();
        let (collapse_scope, mut collapse_stream) = relay();
        let (toggle_scope_expansion, mut toggle_stream) = relay();
        let (select_scope, mut select_stream) = relay();
        
        let expanded_scopes = Actor::new(HashSet::new(), async move |expanded| {
            loop {
                select! {
                    Some(scope) = expand_stream.next() => {
                        expanded.lock_mut().insert(scope);
                    }
                    Some(scope) = collapse_stream.next() => {
                        expanded.lock_mut().remove(&scope);
                    }
                    Some(scope) = toggle_stream.next() => {
                        let mut exp = expanded.lock_mut();
                        if exp.contains(&scope) {
                            exp.remove(&scope);
                        } else {
                            exp.insert(scope);
                        }
                    }
                }
            }
        });
        
        let selected_scopes = Actor::new(HashSet::new(), async move |selected| {
            while let Some(scope) = select_stream.next().await {
                selected.lock_mut().insert(scope);
            }
        });
        
        ScopeTreeManager {
            expanded_scopes,
            selected_scopes,
            expand_scope,
            collapse_scope,
            toggle_scope_expansion,
            select_scope,
        }
    }
}
```

## UI Integration

```rust
// Variables panel component
pub fn variables_panel(
    variable_selection: &VariableSelection,
    value_tracker: &VariableValueTracker,
) -> impl Element {
    Column::new()
        .item(variables_header(variable_selection))
        .item(selected_variables_list(variable_selection, value_tracker))
}

fn variables_header(variable_selection: &VariableSelection) -> impl Element {
    Row::new()
        .item(
            El::new()
                .child_signal(
                    variable_selection.selection_count_signal()
                        .map(|count| Text::new(&format!("Variables ({})", count)))
                )
        )
        .item(
            button()
                .label("Clear All")
                .on_press({
                    let clear_relay = variable_selection.clear_selection.clone();
                    move || clear_relay.send(())
                })
        )
}

fn selected_variables_list(
    variable_selection: &VariableSelection,
    value_tracker: &VariableValueTracker,
) -> impl Element {
    Column::new()
        .items_signal_vec(
            variable_selection.variables.signal_vec_cloned()
                .map(move |var| variable_item(var, value_tracker))
        )
}

fn variable_item(
    variable: SelectedVariable,
    value_tracker: &VariableValueTracker,
) -> impl Element {
    Row::new()
        .item(Text::new(&variable.name))
        .item(
            El::new()
                .child_signal(
                    value_tracker.variable_values.signal()
                        .map({
                            let var_id = variable.id.clone();
                            move |values| {
                                let value = values.get(&var_id)
                                    .cloned()
                                    .unwrap_or_else(|| "N/A".to_string());
                                Text::new(&value)
                            }
                        })
                )
        )
        .item(
            dropdown()
                .selected_signal(always(variable.display_format.clone()))
                .on_change({
                    let update_relay = variable_selection.update_format.clone();
                    let var_id = variable.id.clone();
                    move |format| {
                        update_relay.send((var_id.clone(), format));
                    }
                })
        )
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[async_test]
    async fn test_variable_selection_operations() {
        let selection = VariableSelection::new();
        let mut vars_stream = selection.variables.signal_vec_cloned()
            .to_signal_cloned()
            .to_stream();
        
        // Test initial empty state
        assert_eq!(vars_stream.next().await.unwrap().len(), 0);
        
        // Test adding variable
        let var = SelectedVariable {
            id: "var1".to_string(),
            name: "clock".to_string(),
            scope: "top.cpu".to_string(),
            file_id: "file1".to_string(),
            bit_width: 1,
            display_format: SignalFormat::Binary,
        };
        
        selection.add_variable.send(var.clone());
        let vars = vars_stream.next().await.unwrap();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "clock");
        
        // Test toggling (should remove)
        selection.toggle_variable.send(var.clone());
        assert_eq!(vars_stream.next().await.unwrap().len(), 0);
        
        // Test toggling again (should add)
        selection.toggle_variable.send(var);
        assert_eq!(vars_stream.next().await.unwrap().len(), 1);
    }
    
    #[async_test]
    async fn test_format_update() {
        let selection = VariableSelection::new();
        
        // Add a variable
        let var = SelectedVariable {
            id: "var1".to_string(),
            name: "data".to_string(),
            scope: "top".to_string(),
            file_id: "file1".to_string(),
            bit_width: 8,
            display_format: SignalFormat::Binary,
        };
        selection.add_variable.send(var);
        
        // Update format
        selection.update_format.send(("var1".to_string(), SignalFormat::Hexadecimal));
        
        // Verify format was updated
        let vars = selection.variables.signal_vec_cloned().to_signal_cloned().to_stream().next().await.unwrap();
        assert_eq!(vars[0].display_format, SignalFormat::Hexadecimal);
    }
}
```

This pattern replaces NovyWave's global `SELECTED_VARIABLES` with a clean, testable variable selection system that integrates with timeline cursor tracking and scope management.