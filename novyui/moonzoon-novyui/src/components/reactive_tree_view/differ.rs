use std::collections::{HashMap, HashSet};
use super::{TreeItem, TreeViewContext};

/// Represents a change that needs to be applied to the DOM
#[derive(Debug, Clone, PartialEq)]
pub enum TreeChange {
    /// Add a new item to the tree
    Add {
        key: String,
        item: TreeItem,
        parent_key: Option<String>,
        index: Option<usize>, // Position within parent's children
    },
    
    /// Remove an item from the tree
    Remove {
        key: String,
    },
    
    /// Update an existing item's properties
    Update {
        key: String,
        old_item: TreeItem,
        new_item: TreeItem,
    },
    
    /// Move an item to a different parent (preserving the item itself)
    Move {
        key: String,
        old_parent_key: Option<String>,
        new_parent_key: Option<String>,
        new_index: Option<usize>,
    },
}

/// Efficient tree diffing algorithm
/// Compares old and new tree states to generate minimal set of changes
pub struct TreeDiffer {
    context: TreeViewContext,
}

impl TreeDiffer {
    pub fn new(context: TreeViewContext) -> Self {
        Self { context }
    }
    
    /// Generate minimal set of changes to transform old_items into new_items
    pub fn diff(&self, old_items: &[TreeItem], new_items: &[TreeItem]) -> Vec<TreeChange> {
        let mut changes = Vec::new();
        
        // Create lookup maps for efficient access
        let old_map: HashMap<String, &TreeItem> = old_items.iter()
            .map(|item| (item.key.clone(), item))
            .collect();
            
        let new_map: HashMap<String, &TreeItem> = new_items.iter()
            .map(|item| (item.key.clone(), item))
            .collect();
            
        let old_keys: HashSet<String> = old_map.keys().cloned().collect();
        let new_keys: HashSet<String> = new_map.keys().cloned().collect();
        
        // Find items to remove (in old but not in new)
        for key in old_keys.difference(&new_keys) {
            changes.push(TreeChange::Remove {
                key: key.clone(),
            });
        }
        
        // Find items to add (in new but not in old)
        for key in new_keys.difference(&old_keys) {
            if let Some(new_item) = new_map.get(key) {
                // Calculate insertion index based on sibling order
                let index = self.calculate_insertion_index(new_items, new_item);
                
                changes.push(TreeChange::Add {
                    key: key.clone(),
                    item: (*new_item).clone(),
                    parent_key: new_item.parent_key.clone(),
                    index,
                });
            }
        }
        
        // Find items to update or move (in both old and new)
        for key in old_keys.intersection(&new_keys) {
            if let (Some(old_item), Some(new_item)) = (old_map.get(key), new_map.get(key)) {
                // Check if item moved to different parent
                if old_item.parent_key != new_item.parent_key {
                    let new_index = self.calculate_insertion_index(new_items, new_item);
                    changes.push(TreeChange::Move {
                        key: key.clone(),
                        old_parent_key: old_item.parent_key.clone(),
                        new_parent_key: new_item.parent_key.clone(),
                        new_index,
                    });
                }
                
                // Check if item properties changed (content update needed)
                if self.items_differ_significantly(old_item, new_item) {
                    changes.push(TreeChange::Update {
                        key: key.clone(),
                        old_item: (*old_item).clone(),
                        new_item: (*new_item).clone(),
                    });
                }
            }
        }
        
        // Sort changes for optimal DOM update order:
        // 1. Removes first (to free up positions)
        // 2. Moves (reposition existing elements)
        // 3. Updates (modify existing elements)
        // 4. Adds last (insert new elements)
        changes.sort_by_key(|change| match change {
            TreeChange::Remove { .. } => 0,
            TreeChange::Move { .. } => 1,
            TreeChange::Update { .. } => 2,
            TreeChange::Add { .. } => 3,
        });
        
        changes
    }
    
    /// Check if two items differ enough to warrant a DOM update
    /// Some properties like `selected` or `expanded` are handled by external state
    fn items_differ_significantly(&self, old: &TreeItem, new: &TreeItem) -> bool {
        // Core visual properties that require DOM updates
        old.label != new.label ||
        old.icon != new.icon ||
        old.tooltip != new.tooltip ||
        old.disabled != new.disabled ||
        old.error != new.error ||
        old.loading != new.loading ||
        old.expandable != new.expandable ||
        old.selectable != new.selectable ||
        self.custom_data_differs(&old.custom_data, &new.custom_data)
    }
    
    /// Check if custom data differs in ways that affect rendering
    fn custom_data_differs(&self, old: &super::TreeItemData, new: &super::TreeItemData) -> bool {
        use super::TreeItemData;
        
        match (old, new) {
            (TreeItemData::FileScope { file_state: old_state, .. }, 
             TreeItemData::FileScope { file_state: new_state, .. }) => {
                // File state changes affect icon rendering
                old_state != new_state
            },
            
            (TreeItemData::FileSystem { children_loaded: old_loaded, .. },
             TreeItemData::FileSystem { children_loaded: new_loaded, .. }) => {
                // Children loaded state affects expandability  
                old_loaded != new_loaded
            },
            
            (TreeItemData::Generic(old_map), TreeItemData::Generic(new_map)) => {
                old_map != new_map
            },
            
            // Different data types always differ
            _ => true,
        }
    }
    
    /// Calculate optimal insertion index for new item within its parent's children
    fn calculate_insertion_index(&self, all_items: &[TreeItem], new_item: &TreeItem) -> Option<usize> {
        // Find all siblings (items with same parent)
        let siblings: Vec<&TreeItem> = all_items.iter()
            .filter(|item| item.parent_key == new_item.parent_key)
            .collect();
            
        if siblings.is_empty() {
            return Some(0);
        }
        
        // For now, use simple append strategy
        // TODO: Implement smart ordering based on context
        // - FilesAndScopes: Sort by file order, then scope hierarchy
        // - LoadFiles: Sort directories first, then files alphabetically
        Some(siblings.len())
    }
    
    /// Create hierarchical tree structure from flat item list
    /// Returns (root_items, children_map)
    pub fn build_hierarchy(&self, items: &[TreeItem]) -> (Vec<&TreeItem>, HashMap<String, Vec<&TreeItem>>) {
        let mut root_items = Vec::new();
        let mut children_map: HashMap<String, Vec<&TreeItem>> = HashMap::new();
        
        for item in items {
            if let Some(parent_key) = &item.parent_key {
                children_map
                    .entry(parent_key.clone())
                    .or_insert_with(Vec::new)
                    .push(item);
            } else {
                root_items.push(item);
            }
        }
        
        // Sort children based on context-specific rules
        for children in children_map.values_mut() {
            self.sort_children(children);
        }
        
        self.sort_children(&mut root_items);
        
        (root_items, children_map)
    }
    
    /// Sort children based on context-specific rules
    fn sort_children(&self, children: &mut Vec<&TreeItem>) {
        match self.context {
            TreeViewContext::FilesAndScopes => {
                // Files first, then scopes alphabetically
                children.sort_by(|a, b| {
                    use super::TreeItemData;
                    match (&a.custom_data, &b.custom_data) {
                        (TreeItemData::FileScope { file_id: Some(_), scope_path: None, .. },
                         TreeItemData::FileScope { scope_path: Some(_), .. }) => {
                            std::cmp::Ordering::Less  // Files before scopes
                        },
                        (TreeItemData::FileScope { scope_path: Some(_), .. },
                         TreeItemData::FileScope { file_id: Some(_), scope_path: None, .. }) => {
                            std::cmp::Ordering::Greater  // Scopes after files
                        },
                        _ => a.label.cmp(&b.label)  // Alphabetical within same type
                    }
                });
            },
            
            TreeViewContext::LoadFiles => {
                // Directories first, then files, both alphabetically
                children.sort_by(|a, b| {
                    use super::TreeItemData;
                    match (&a.custom_data, &b.custom_data) {
                        (TreeItemData::FileSystem { is_directory: true, .. },
                         TreeItemData::FileSystem { is_directory: false, .. }) => {
                            std::cmp::Ordering::Less  // Directories before files
                        },
                        (TreeItemData::FileSystem { is_directory: false, .. },
                         TreeItemData::FileSystem { is_directory: true, .. }) => {
                            std::cmp::Ordering::Greater  // Files after directories
                        },
                        _ => a.label.cmp(&b.label)  // Alphabetical within same type
                    }
                });
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::reactive_tree_view::TreeItemData;
    
    fn create_test_item(key: &str, label: &str, parent_key: Option<String>) -> TreeItem {
        TreeItem {
            key: key.to_string(),
            parent_key,
            label: label.to_string(),
            icon: None,
            tooltip: None,
            expandable: false,
            expanded: false,
            selectable: true,
            selected: false,
            disabled: false,
            error: false,
            loading: false,
            custom_data: TreeItemData::Generic(std::collections::HashMap::new()),
        }
    }
    
    #[test]
    fn diff_empty_to_single_item() {
        let differ = TreeDiffer::new(TreeViewContext::FilesAndScopes);
        let old_items = vec![];
        let new_items = vec![create_test_item("item1", "Item 1", None)];
        
        let changes = differ.diff(&old_items, &new_items);
        
        assert_eq!(changes.len(), 1);
        match &changes[0] {
            TreeChange::Add { key, .. } => assert_eq!(key, "item1"),
            _ => panic!("Expected Add change"),
        }
    }
    
    #[test]
    fn diff_remove_item() {
        let differ = TreeDiffer::new(TreeViewContext::FilesAndScopes);
        let old_items = vec![create_test_item("item1", "Item 1", None)];
        let new_items = vec![];
        
        let changes = differ.diff(&old_items, &new_items);
        
        assert_eq!(changes.len(), 1);
        match &changes[0] {
            TreeChange::Remove { key } => assert_eq!(key, "item1"),
            _ => panic!("Expected Remove change"),
        }
    }
    
    #[test]
    fn diff_update_item() {
        let differ = TreeDiffer::new(TreeViewContext::FilesAndScopes);
        let old_items = vec![create_test_item("item1", "Old Label", None)];
        let new_items = vec![create_test_item("item1", "New Label", None)];
        
        let changes = differ.diff(&old_items, &new_items);
        
        assert_eq!(changes.len(), 1);
        match &changes[0] {
            TreeChange::Update { key, old_item, new_item } => {
                assert_eq!(key, "item1");
                assert_eq!(old_item.label, "Old Label");
                assert_eq!(new_item.label, "New Label");
            },
            _ => panic!("Expected Update change"),
        }
    }
    
    #[test]
    fn diff_move_item() {
        let differ = TreeDiffer::new(TreeViewContext::FilesAndScopes);
        let old_items = vec![create_test_item("item1", "Item 1", None)];
        let new_items = vec![create_test_item("item1", "Item 1", Some("parent1".to_string()))];
        
        let changes = differ.diff(&old_items, &new_items);
        
        assert_eq!(changes.len(), 1);
        match &changes[0] {
            TreeChange::Move { key, old_parent_key, new_parent_key, .. } => {
                assert_eq!(key, "item1");
                assert_eq!(old_parent_key, &None);
                assert_eq!(new_parent_key, &Some("parent1".to_string()));
            },
            _ => panic!("Expected Move change"),
        }
    }
    
    #[test]
    fn build_hierarchy() {
        let differ = TreeDiffer::new(TreeViewContext::FilesAndScopes);
        let items = vec![
            create_test_item("root1", "Root 1", None),
            create_test_item("child1", "Child 1", Some("root1".to_string())),
            create_test_item("child2", "Child 2", Some("root1".to_string())),
            create_test_item("root2", "Root 2", None),
        ];
        
        let (root_items, children_map) = differ.build_hierarchy(&items);
        
        assert_eq!(root_items.len(), 2);
        assert_eq!(children_map.get("root1").unwrap().len(), 2);
        assert!(children_map.get("root2").is_none());
    }
}