use std::fmt;

/// Defines different contexts where ReactiveTreeView can be used
/// Each context has different behavioral requirements and data patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeViewContext {
    /// Files & Scopes panel: Complex nested objects with real-time state updates
    /// - Data: TrackedFile + recursive scope hierarchy  
    /// - Updates: High-frequency file loading states, progressive scope discovery
    /// - Selection: Single scope selection with persistent expansion
    FilesAndScopes,
    
    /// Load Files dialog: Simple filesystem representation with lazy loading
    /// - Data: Directory/file entries with boolean flags
    /// - Updates: User-driven navigation, lower frequency
    /// - Selection: Multi-file selection with directory navigation
    LoadFiles,
}

impl fmt::Display for TreeViewContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TreeViewContext::FilesAndScopes => write!(f, "FilesAndScopes"),
            TreeViewContext::LoadFiles => write!(f, "LoadFiles"),
        }
    }
}

/// Selection behavior modes for different contexts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    /// No selection allowed - display only
    None,
    
    /// Single item selection with replacement
    /// Used in Files & Scopes for scope selection
    Single,
    
    /// Multiple item selection
    /// Used in Load Files dialog for multi-file selection
    Multiple,
}

impl Default for SelectionMode {
    fn default() -> Self {
        SelectionMode::None
    }
}

/// Update strategy for different performance requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateStrategy {
    /// Reactive updates - immediate response to signal changes
    /// Used for Files & Scopes with real-time file loading
    Reactive,
    
    /// On-demand updates - triggered by user actions
    /// Used for Load Files with directory navigation
    OnDemand,
}

impl Default for UpdateStrategy {
    fn default() -> Self {
        UpdateStrategy::Reactive
    }
}

impl TreeViewContext {
    /// Get the typical selection mode for this context
    pub fn default_selection_mode(self) -> SelectionMode {
        match self {
            TreeViewContext::FilesAndScopes => SelectionMode::Single,
            TreeViewContext::LoadFiles => SelectionMode::Multiple,
        }
    }
    
    /// Get the typical update strategy for this context
    pub fn default_update_strategy(self) -> UpdateStrategy {
        match self {
            TreeViewContext::FilesAndScopes => UpdateStrategy::Reactive,
            TreeViewContext::LoadFiles => UpdateStrategy::OnDemand,
        }
    }
    
    /// Whether this context typically needs persistent expansion state
    pub fn needs_persistent_expansion(self) -> bool {
        match self {
            TreeViewContext::FilesAndScopes => true,  // Save expanded scopes to config
            TreeViewContext::LoadFiles => false,     // Session-only navigation state
        }
    }
}