//! SelectedVariables domain for variable selection using Actor+Relay architecture
//!
//! Consolidated variable selection domain to replace global mutables with event-driven architecture.
//! Manages which variables are currently selected for display in the waveform timeline.

use crate::actors::{Actor, ActorVec, ActorVecHandle, Relay, relay};
use shared::SelectedVariable;
use zoon::Task;
use indexmap::IndexSet;
use futures::{select, StreamExt};

/// Domain-driven variable selection with Actor+Relay architecture.
/// 
/// Replaces variable selection global mutables with cohesive event-driven state management.
/// Tracks selected variables, their display order, and filtering state.
#[derive(Clone, Debug)]
pub struct SelectedVariables {
    /// Core selected variables collection
    variables: ActorVec<SelectedVariable>,
    
    /// Selection order tracking (for consistent display)
    selection_order: Actor<Vec<String>>,
    
    /// Current filter text for variable search
    filter_text: Actor<String>,
    
    /// Currently expanded scopes in the variable tree
    expanded_scopes: Actor<IndexSet<String>>,
    
    // === USER VARIABLE INTERACTION EVENTS ===
    /// User clicked a variable in the tree view
    pub variable_clicked_relay: Relay<SelectedVariable>,
    
    /// User removed a variable from selection
    pub variable_removed_relay: Relay<String>,
    
    /// User reordered variables via drag & drop
    pub variables_reordered_relay: Relay<Vec<String>>,
    
    /// User cleared all selected variables
    pub selection_cleared_relay: Relay<()>,
    
    /// User toggled selection of multiple variables
    pub batch_variables_toggled_relay: Relay<Vec<SelectedVariable>>,
    
    // === VARIABLE TREE INTERACTION EVENTS ===
    /// User expanded a scope in variable tree
    pub scope_expanded_relay: Relay<String>,
    
    /// User collapsed a scope in variable tree
    pub scope_collapsed_relay: Relay<String>,
    
    /// User typed in variable filter/search box
    pub filter_text_changed_relay: Relay<String>,
    
    /// User cleared the filter text
    pub filter_cleared_relay: Relay<()>,
    
    // === SYSTEM VARIABLE EVENTS ===
    /// Variables restored from saved configuration
    pub variables_restored_relay: Relay<Vec<SelectedVariable>>,
    
    /// File was removed, clean up its variables
    pub file_removed_relay: Relay<String>,
    
    /// All files cleared, reset variable selection
    pub all_files_cleared_relay: Relay<()>,
}

impl SelectedVariables {
    /// Create a new SelectedVariables domain with event processors
    pub async fn new() -> Self {
        // Create relays for variable operations
        let (variable_clicked_relay, variable_clicked_stream) = relay();
        let (variable_removed_relay, variable_removed_stream) = relay();
        let (variables_reordered_relay, variables_reordered_stream) = relay();
        let (selection_cleared_relay, selection_cleared_stream) = relay();
        let (batch_variables_toggled_relay, _batch_variables_toggled_stream) = relay();
        
        // Create relays for tree operations
        let (scope_expanded_relay, scope_expanded_stream) = relay();
        let (scope_collapsed_relay, _scope_collapsed_stream) = relay();
        let (filter_text_changed_relay, filter_text_changed_stream) = relay();
        let (filter_cleared_relay, filter_cleared_stream) = relay();
        
        // Create relays for system events
        let (variables_restored_relay, variables_restored_stream) = relay();
        let (file_removed_relay, file_removed_stream) = relay();
        let (all_files_cleared_relay, all_files_cleared_stream) = relay();
        
        // Create variables actor with event handling
        let variables = ActorVec::new(vec![], {
            async move |variables_handle| {
                let mut variable_clicked = variable_clicked_stream;
                let mut variable_removed = variable_removed_stream;
                let mut selection_cleared = selection_cleared_stream;
                let mut variables_restored = variables_restored_stream;
                let mut file_removed = file_removed_stream;
                let _all_files_cleared = all_files_cleared_stream;
                
                while let Some(variable) = variable_clicked.next().await {
                    Self::handle_variable_clicked(&variables_handle, variable);
                }
                // Additional event handlers will be added in future iterations
            }
        });
        
        // Create selection order actor
        let selection_order = Actor::new(vec![], async move |order_handle| {
            let mut variables_reordered = variables_reordered_stream;
            
            while let Some(new_order) = variables_reordered.next().await {
                order_handle.set(new_order);
            }
        });
        
        // Create filter text actor
        let filter_text = Actor::new(String::new(), async move |filter_handle| {
            let mut filter_changed = filter_text_changed_stream;
            let mut filter_cleared = filter_cleared_stream;
            
            loop {
                if let Some(text) = filter_changed.next().await {
                    filter_handle.set(text);
                } else if let Some(()) = filter_cleared.next().await {
                    filter_handle.set(String::new());
                } else {
                    break;
                }
            }
        });
        
        // Create expanded scopes actor
        let expanded_scopes = Actor::new(IndexSet::new(), async move |scopes_handle| {
            let mut scope_expanded = scope_expanded_stream;
            
            while let Some(scope_id) = scope_expanded.next().await {
                scopes_handle.update(|scopes| { scopes.insert(scope_id); });
            }
        });
        
        Self {
            variables,
            selection_order,
            filter_text,
            expanded_scopes,
            
            variable_clicked_relay,
            variable_removed_relay,
            variables_reordered_relay,
            selection_cleared_relay,
            batch_variables_toggled_relay,
            
            scope_expanded_relay,
            scope_collapsed_relay,
            filter_text_changed_relay,
            filter_cleared_relay,
            
            variables_restored_relay,
            file_removed_relay,
            all_files_cleared_relay,
        }
    }
    
    // === REACTIVE SIGNAL ACCESS ===
    
    /// Get reactive signal for all selected variables
    pub fn variables_signal(&self) -> impl zoon::Signal<Item = Vec<SelectedVariable>> {
        self.variables.signal()
    }
    
    /// Get reactive signal for variables as signal vec (VecDiff updates)  
    pub fn variables_signal_vec(&self) -> impl zoon::SignalVec<Item = SelectedVariable> {
        self.variables.signal_vec()
    }
    
    /// Get reactive signal for variable selection order
    pub fn selection_order_signal(&self) -> impl zoon::Signal<Item = Vec<String>> {
        self.selection_order.signal()
    }
    
    /// Get reactive signal for filter text
    pub fn filter_text_signal(&self) -> impl zoon::Signal<Item = String> {
        self.filter_text.signal()
    }
    
    /// Get reactive signal for expanded scopes
    pub fn expanded_scopes_signal(&self) -> impl zoon::Signal<Item = IndexSet<String>> {
        self.expanded_scopes.signal()
    }
    
    /// Get selected variables count signal
    pub fn variable_count_signal(&self) -> impl zoon::Signal<Item = usize> {
        self.variables.signal_ref(|vars| vars.len())
    }
    
    /// Check if a specific variable is selected
    pub fn is_variable_selected_signal(&self, variable_id: String) -> impl zoon::Signal<Item = bool> {
        self.variables.signal_ref(move |vars| {
            vars.iter().any(|v| v.unique_id == variable_id)
        })
    }
    
    /// Get variables from a specific file  
    pub fn file_variables_signal(&self, file_path: String) -> impl zoon::Signal<Item = Vec<SelectedVariable>> {
        use zoon::SignalExt;
        self.variables.signal()
            .map(move |vars| {
                vars.iter()
                    .filter(|v| v.file_path().as_ref() == Some(&file_path))
                    .cloned()
                    .collect::<Vec<SelectedVariable>>()
            })
    }
    
    /// Check if filter is active
    pub fn is_filter_active_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.filter_text.signal_ref(|text| !text.is_empty())
    }
}

// === EVENT HANDLER IMPLEMENTATIONS ===

impl SelectedVariables {
    /// Handle user clicking a variable (toggle selection)
    fn handle_variable_clicked(
        variables_handle: &ActorVecHandle<SelectedVariable>,
        variable: SelectedVariable
    ) {
        // Check if variable is already selected
        let mut is_already_selected = false;
        
        // First, try to find and remove if already selected
        variables_handle.update(|vars| {
            if let Some(pos) = vars.iter().position(|v| v.unique_id == variable.unique_id) {
                vars.remove(pos);
                is_already_selected = true;
            }
        });
        
        // If not already selected, add it
        if !is_already_selected {
            variables_handle.push_cloned(variable);
        }
    }
    
    /// Handle removing specific variable
    fn handle_variable_removed(
        variables_handle: &ActorVecHandle<SelectedVariable>,
        variable_id: String
    ) {
        variables_handle.retain(|var| var.unique_id != variable_id);
    }
    
    /// Handle clearing all selected variables
    fn handle_selection_cleared(
        variables_handle: &ActorVecHandle<SelectedVariable>
    ) {
        variables_handle.clear();
    }
    
    /// Handle batch variable toggle
    fn handle_batch_variables_toggled(
        variables_handle: &ActorVecHandle<SelectedVariable>,
        variables: Vec<SelectedVariable>
    ) {
        for variable in variables {
            Self::handle_variable_clicked(variables_handle, variable);
        }
    }
    
    /// Handle variables restored from configuration
    fn handle_variables_restored(
        variables_handle: &ActorVecHandle<SelectedVariable>,
        restored_variables: Vec<SelectedVariable>
    ) {
        // Clear existing and replace with restored
        variables_handle.clear();
        for variable in restored_variables {
            variables_handle.push_cloned(variable);
        }
    }
    
    /// Handle file removal - clean up variables from that file
    fn handle_file_removed(
        variables_handle: &ActorVecHandle<SelectedVariable>,
        file_path: String
    ) {
        variables_handle.retain(|var| var.file_path().as_ref() != Some(&file_path));
    }
    
    /// Handle all files cleared
    fn handle_all_files_cleared(
        variables_handle: &ActorVecHandle<SelectedVariable>
    ) {
        variables_handle.clear();
    }
}

// === CONVENIENCE FUNCTIONS FOR UI INTEGRATION ===

/// Global SelectedVariables instance
static SELECTED_VARIABLES_INSTANCE: std::sync::OnceLock<SelectedVariables> = std::sync::OnceLock::new();

/// Initialize the SelectedVariables domain (call once on app startup)
pub async fn initialize_selected_variables() -> SelectedVariables {
    let selected_variables = SelectedVariables::new().await;
    SELECTED_VARIABLES_INSTANCE.set(selected_variables.clone())
        .expect("SelectedVariables already initialized - initialize_selected_variables() should only be called once");
    selected_variables
}

/// Get the global SelectedVariables instance
pub fn get_selected_variables() -> SelectedVariables {
    SELECTED_VARIABLES_INSTANCE.get()
        .expect("SelectedVariables not initialized - call initialize_selected_variables() first")
        .clone()
}