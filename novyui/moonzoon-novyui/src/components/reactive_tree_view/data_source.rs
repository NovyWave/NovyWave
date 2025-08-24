use zoon::{Signal, SignalVec, MutableVec, Mutable};
use std::collections::HashMap;
use std::hash::Hash;

/// Generic tree item that can represent different data types
/// Used internally for unified handling across contexts
#[derive(Debug, Clone, PartialEq)]
pub struct TreeItem {
    /// Unique identifier for this item (used for diffing)
    pub key: String,
    
    /// Parent key for hierarchy (None = root level)
    pub parent_key: Option<String>,
    
    /// Display properties
    pub label: String,
    pub icon: Option<String>,
    pub tooltip: Option<String>,
    
    /// State flags
    pub expandable: bool,
    pub expanded: bool,
    pub selectable: bool,
    pub selected: bool,
    
    /// Visual properties
    pub disabled: bool,
    pub error: bool,
    pub loading: bool,
    
    /// Custom data for context-specific rendering
    pub custom_data: TreeItemData,
}

/// Context-specific data that can be attached to tree items
#[derive(Debug, Clone, PartialEq)]
pub enum TreeItemData {
    /// Files & Scopes context data
    FileScope {
        file_id: Option<String>,
        scope_path: Option<String>,
        file_state: Option<String>, // Loading, Loaded, Error, etc.
    },
    
    /// Load Files context data  
    FileSystem {
        path: String,
        is_directory: bool,
        file_size: Option<u64>,
        children_loaded: bool,
    },
    
    /// Generic data for future extensibility
    Generic(HashMap<String, String>),
}

impl Default for TreeItemData {
    fn default() -> Self {
        TreeItemData::Generic(HashMap::new())
    }
}

/// Abstraction over different data sources
/// Allows ReactiveTreeView to work with different signal types
pub enum DataSource<T> {
    /// Signal vector for reactive collections (e.g., TrackedFiles)
    SignalVec(Box<dyn SignalVec<Item = T> + Unpin>),
    
    /// Single signal with collection data (e.g., FILE_TREE_CACHE)
    Signal(Box<dyn Signal<Item = Vec<T>> + Unpin>),
    
    /// Static data for testing/simple cases
    Static(Vec<T>),
}

impl<T> DataSource<T> {
    /// Create from a SignalVec (most common for reactive collections)
    pub fn from_signal_vec<S>(signal_vec: S) -> Self 
    where S: SignalVec<Item = T> + Unpin + 'static
    {
        DataSource::SignalVec(Box::new(signal_vec))
    }
    
    /// Create from a Signal<Vec<T>> (common for cached collections)
    pub fn from_signal<S>(signal: S) -> Self 
    where 
        S: Signal<Item = Vec<T>> + Unpin + 'static
    {
        DataSource::Signal(Box::new(signal))
    }
    
    /// Create from static data (mainly for testing)
    pub fn from_static(data: Vec<T>) -> Self {
        DataSource::Static(data)
    }
}

/// Trait for converting domain objects to TreeItem
/// Each context implements this for their data types
pub trait ToTreeItem {
    fn to_tree_item(&self, context: super::TreeViewContext) -> TreeItem;
    fn get_key(&self) -> String;
    fn get_parent_key(&self, context: super::TreeViewContext) -> Option<String>;
}

// Example implementation for common patterns
// These would be implemented in the frontend code for specific types

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn tree_item_creation() {
        let item = TreeItem {
            key: "test-key".to_string(),
            parent_key: None,
            label: "Test Item".to_string(),
            icon: Some("file".to_string()),
            tooltip: Some("Test tooltip".to_string()),
            expandable: false,
            expanded: false,
            selectable: true,
            selected: false,
            disabled: false,
            error: false,
            loading: false,
            custom_data: TreeItemData::Generic(HashMap::new()),
        };
        
        assert_eq!(item.key, "test-key");
        assert_eq!(item.label, "Test Item");
        assert!(item.selectable);
        assert!(!item.expandable);
    }
    
    #[test]
    fn file_scope_data() {
        let data = TreeItemData::FileScope {
            file_id: Some("file1".to_string()),
            scope_path: Some("TOP.cpu".to_string()),
            file_state: Some("Loaded".to_string()),
        };
        
        match data {
            TreeItemData::FileScope { file_id, scope_path, file_state } => {
                assert_eq!(file_id, Some("file1".to_string()));
                assert_eq!(scope_path, Some("TOP.cpu".to_string()));
                assert_eq!(file_state, Some("Loaded".to_string()));
            },
            _ => panic!("Expected FileScope data"),
        }
    }
}