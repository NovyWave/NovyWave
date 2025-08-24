use zoon::*;
use web_sys::{Element as DomElement, HtmlElement};
use std::collections::HashMap;
use super::{TreeChange, TreeItem, TreeItemBuilder};

/// Manages minimal DOM updates for ReactiveTreeView
/// Maintains a registry of DOM elements and applies targeted changes
pub struct DomUpdater {
    /// Registry of DOM elements by item key
    element_registry: HashMap<String, HtmlElement>,
    
    /// Container element that holds the tree
    container: HtmlElement,
    
    /// Hierarchy tracking for efficient insertions
    hierarchy: HashMap<String, Vec<String>>, // parent_key -> [child_keys]
}

impl DomUpdater {
    /// Create new DOM updater with container element
    pub fn new(container: HtmlElement) -> Self {
        Self {
            element_registry: HashMap::new(),
            container,
            hierarchy: HashMap::new(),
        }
    }
    
    /// Apply a batch of changes to the DOM
    /// Changes are expected to be pre-sorted in optimal order
    pub fn apply_changes(&mut self, changes: Vec<TreeChange>) {
        for change in changes {
            match change {
                TreeChange::Remove { key } => {
                    self.remove_item(&key);
                },
                
                TreeChange::Move { key, old_parent_key, new_parent_key, new_index } => {
                    self.move_item(&key, old_parent_key.as_deref(), new_parent_key.as_deref(), new_index);
                },
                
                TreeChange::Update { key, new_item, .. } => {
                    self.update_item(&key, &new_item);
                },
                
                TreeChange::Add { key, item, parent_key, index } => {
                    self.add_item(&key, &item, parent_key.as_deref(), index);
                },
            }
        }
    }
    
    /// Add new item to DOM
    fn add_item(&mut self, key: &str, item: &TreeItem, parent_key: Option<&str>, index: Option<usize>) {
        // Create DOM element for the item
        let item_element = self.create_item_element(item);
        
        // Find parent container
        let parent_element = if let Some(parent_key) = parent_key {
            // Find parent item's children container
            self.element_registry.get(parent_key)
                .and_then(|parent| self.find_children_container(parent))
                .unwrap_or_else(|| self.container.clone())
        } else {
            self.container.clone()
        };
        
        // Insert at specified index or append
        if let Some(index) = index {
            if let Some(next_sibling) = parent_element.children().item(index as u32) {
                parent_element.insert_before(&item_element, Some(&next_sibling))
                    .expect("Failed to insert DOM element");
            } else {
                parent_element.append_child(&item_element)
                    .expect("Failed to append DOM element");
            }
        } else {
            parent_element.append_child(&item_element)
                .expect("Failed to append DOM element");
        }
        
        // Register element
        self.element_registry.insert(key.to_string(), item_element);
        
        // Update hierarchy tracking
        if let Some(parent_key) = parent_key {
            self.hierarchy
                .entry(parent_key.to_string())
                .or_insert_with(Vec::new)
                .push(key.to_string());
        }
    }
    
    /// Remove item from DOM
    fn remove_item(&mut self, key: &str) {
        if let Some(element) = self.element_registry.remove(key) {
            // Remove from DOM
            if let Some(parent) = element.parent_element() {
                parent.remove_child(&element)
                    .expect("Failed to remove DOM element");
            }
            
            // Remove from hierarchy tracking
            self.hierarchy.retain(|_, children| {
                children.retain(|child_key| child_key != key);
                !children.is_empty()
            });
            
            // Remove any children recursively
            if let Some(child_keys) = self.hierarchy.remove(key) {
                for child_key in child_keys {
                    self.remove_item(&child_key);
                }
            }
        }
    }
    
    /// Update existing item in DOM
    fn update_item(&mut self, key: &str, new_item: &TreeItem) {
        if let Some(element) = self.element_registry.get(key) {
            // Update the content without recreating the entire element
            self.update_item_content(element, new_item);
        }
    }
    
    /// Move item to different parent location
    fn move_item(&mut self, key: &str, old_parent_key: Option<&str>, new_parent_key: Option<&str>, new_index: Option<usize>) {
        if let Some(element) = self.element_registry.get(key).cloned() {
            // Remove from old parent in hierarchy tracking
            if let Some(old_parent_key) = old_parent_key {
                if let Some(old_siblings) = self.hierarchy.get_mut(old_parent_key) {
                    old_siblings.retain(|child_key| child_key != key);
                }
            }
            
            // Find new parent container
            let new_parent_element = if let Some(new_parent_key) = new_parent_key {
                self.element_registry.get(new_parent_key)
                    .and_then(|parent| self.find_children_container(parent))
                    .unwrap_or_else(|| self.container.clone())
            } else {
                self.container.clone()
            };
            
            // Move element to new location
            if let Some(index) = new_index {
                if let Some(next_sibling) = new_parent_element.children().item(index as u32) {
                    new_parent_element.insert_before(&element, Some(&next_sibling))
                        .expect("Failed to move DOM element");
                } else {
                    new_parent_element.append_child(&element)
                        .expect("Failed to move DOM element");
                }
            } else {
                new_parent_element.append_child(&element)
                    .expect("Failed to move DOM element");
            }
            
            // Update hierarchy tracking
            if let Some(new_parent_key) = new_parent_key {
                self.hierarchy
                    .entry(new_parent_key.to_string())
                    .or_insert_with(Vec::new)
                    .push(key.to_string());
            }
        }
    }
    
    /// Create DOM element for tree item
    fn create_item_element(&self, item: &TreeItem) -> HtmlElement {
        // For now, create a basic structure
        // This will be replaced with proper TreeItemBuilder integration
        let _item_builder = TreeItemBuilder::new()
            .label(&item.label)
            .expandable(item.expandable)
            .expanded(item.expanded)
            .selectable(item.selectable)
            .selected(item.selected)
            .disabled(item.disabled)
            .error(item.error)
            .loading(item.loading);
            
        // Convert TreeItemBuilder to DOM element
        // TODO: Implement proper conversion from Zoon Element to HtmlElement
        self.create_placeholder_element(&item.label)
    }
    
    /// Create placeholder DOM element (temporary implementation)
    fn create_placeholder_element(&self, label: &str) -> HtmlElement {
        let document = web_sys::window().unwrap().document().unwrap();
        let div = document.create_element("div").unwrap()
            .dyn_into::<HtmlElement>().unwrap();
        
        div.set_inner_text(label);
        div.set_class_name("tree-item");
        
        // Add basic styling
        div.style().set_property("padding", "4px 8px").unwrap();
        div.style().set_property("cursor", "pointer").unwrap();
        div.style().set_property("user-select", "none").unwrap();
        
        div
    }
    
    /// Update content of existing DOM element
    fn update_item_content(&self, element: &HtmlElement, new_item: &TreeItem) {
        // Update text content
        element.set_inner_text(&new_item.label);
        
        // Update CSS classes based on state
        let mut class_list = vec!["tree-item".to_string()];
        
        if new_item.selected {
            class_list.push("selected".to_string());
        }
        if new_item.disabled {
            class_list.push("disabled".to_string());
        }
        if new_item.error {
            class_list.push("error".to_string());
        }
        if new_item.loading {
            class_list.push("loading".to_string());
        }
        
        element.set_class_name(&class_list.join(" "));
        
        // Update tooltip
        if let Some(tooltip) = &new_item.tooltip {
            element.set_title(tooltip);
        } else {
            element.remove_attribute("title").ok();
        }
    }
    
    /// Find children container within a tree item element
    fn find_children_container(&self, parent_element: &HtmlElement) -> Option<HtmlElement> {
        // Look for a container element that holds children
        // This depends on the TreeItemBuilder structure
        
        // For now, return the parent element itself
        // TODO: Implement proper children container detection
        Some(parent_element.clone())
    }
    
    /// Clear all items from the tree
    pub fn clear(&mut self) {
        // Remove all child elements
        while let Some(child) = self.container.first_element_child() {
            self.container.remove_child(&child)
                .expect("Failed to remove child element");
        }
        
        // Clear registries
        self.element_registry.clear();
        self.hierarchy.clear();
    }
    
    /// Get current number of managed elements
    pub fn element_count(&self) -> usize {
        self.element_registry.len()
    }
    
    /// Check if element exists in registry
    pub fn has_element(&self, key: &str) -> bool {
        self.element_registry.contains_key(key)
    }
}

/// Helper trait for converting Zoon Elements to DOM elements
/// This will need to be implemented to properly integrate with Zoon
trait ToDomElement {
    fn to_dom_element(&self) -> HtmlElement;
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    
    wasm_bindgen_test_configure!(run_in_browser);
    
    #[wasm_bindgen_test]
    fn create_dom_updater() {
        let document = web_sys::window().unwrap().document().unwrap();
        let container = document.create_element("div").unwrap()
            .dyn_into::<HtmlElement>().unwrap();
            
        let updater = DomUpdater::new(container);
        
        assert_eq!(updater.element_count(), 0);
    }
    
    #[wasm_bindgen_test]
    fn add_remove_element() {
        let document = web_sys::window().unwrap().document().unwrap();
        let container = document.create_element("div").unwrap()
            .dyn_into::<HtmlElement>().unwrap();
            
        let mut updater = DomUpdater::new(container.clone());
        
        let item = TreeItem {
            key: "test-item".to_string(),
            parent_key: None,
            label: "Test Item".to_string(),
            icon: None,
            tooltip: None,
            expandable: false,
            expanded: false,
            selectable: true,
            selected: false,
            disabled: false,
            error: false,
            loading: false,
            custom_data: super::super::TreeItemData::Generic(std::collections::HashMap::new()),
        };
        
        // Add item
        let changes = vec![TreeChange::Add {
            key: "test-item".to_string(),
            item: item.clone(),
            parent_key: None,
            index: None,
        }];
        
        updater.apply_changes(changes);
        
        assert_eq!(updater.element_count(), 1);
        assert!(updater.has_element("test-item"));
        assert_eq!(container.children().length(), 1);
        
        // Remove item
        let changes = vec![TreeChange::Remove {
            key: "test-item".to_string(),
        }];
        
        updater.apply_changes(changes);
        
        assert_eq!(updater.element_count(), 0);
        assert!(!updater.has_element("test-item"));
        assert_eq!(container.children().length(), 0);
    }
}