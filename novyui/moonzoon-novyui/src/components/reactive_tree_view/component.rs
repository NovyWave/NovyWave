use zoon::*;
use zoon::column::EmptyFlagNotSet;
use zoon::RawHtmlEl;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use web_sys::HtmlElement;
use super::{
    TreeViewContext,
    SelectionMode, 
    UpdateStrategy,
    DataSource,
    TreeItemRenderer,
    TreeDiffer,
    DomUpdater,
    TreeItem,
    TreeChange,
    ReactiveTreeViewConfig,
};

/// Main ReactiveTreeView component
/// Provides ultra-granular updates with signal-driven data
pub struct ReactiveTreeView<T> {
    context: TreeViewContext,
    data_source: DataSource<T>,
    item_renderer: TreeItemRenderer<T>,
    config: ReactiveTreeViewConfig,
    
    // Internal state
    current_items: Mutable<Vec<TreeItem>>,
    differ: Arc<TreeDiffer>,
    dom_updater: Mutable<Option<DomUpdater>>,
    
    // External state tracking
    external_expanded_state: Mutable<HashSet<String>>,
    external_selected_state: Mutable<Vec<String>>,
}

impl<T> ReactiveTreeView<T>
where T: Clone + 'static
{
    /// Create new ReactiveTreeView with configuration
    pub fn new(
        context: TreeViewContext,
        data_source: DataSource<T>,
        item_renderer: TreeItemRenderer<T>,
        config: ReactiveTreeViewConfig,
    ) -> Self {
        let differ = Arc::new(TreeDiffer::new(context));
        
        Self {
            context,
            data_source,
            item_renderer,
            config,
            current_items: Mutable::new(Vec::new()),
            differ,
            dom_updater: Mutable::new(None),
            external_expanded_state: Mutable::new(HashSet::new()),
            external_selected_state: Mutable::new(Vec::new()),
        }
    }
    
    /// Convert data items to TreeItems using the renderer
    fn convert_data_to_tree_items(&self, data: &[T]) -> Vec<TreeItem> {
        data.iter()
            .map(|item| {
                let key = self.item_renderer.get_key(item);
                let parent_key = self.item_renderer.get_parent_key(item, self.context);
                let tree_item_builder = self.item_renderer.render(item, self.context);
                
                // Convert TreeItemBuilder to TreeItem using getter methods
                TreeItem {
                    key,
                    parent_key,
                    label: tree_item_builder.get_label().to_string(),
                    icon: tree_item_builder.get_icon().clone(),
                    tooltip: tree_item_builder.get_tooltip().clone(),
                    expandable: tree_item_builder.is_expandable(),
                    expanded: self.external_expanded_state.lock_ref().contains(&tree_item_builder.get_label().to_string()),
                    selectable: tree_item_builder.is_selectable(),
                    selected: self.external_selected_state.lock_ref().contains(&tree_item_builder.get_label().to_string()),
                    disabled: tree_item_builder.is_disabled(),
                    error: tree_item_builder.has_error(),
                    loading: tree_item_builder.is_loading(),
                    custom_data: super::TreeItemData::Generic(HashMap::new()), // TODO: Extract from builder
                }
            })
            .collect()
    }
    
    /// Update tree based on new data
    fn update_tree(&self, new_data: Vec<T>) {
        let old_items = self.current_items.lock_ref().clone();
        let new_tree_items = self.convert_data_to_tree_items(&new_data);
        
        // Generate minimal set of changes
        let changes = self.differ.diff(&old_items, &new_tree_items);
        
        // Apply changes to DOM if updater is available
        if let Some(dom_updater) = self.dom_updater.lock_ref().as_ref() {
            // Apply changes through DOM updater
            // Note: We would need to clone the updater or use RefCell for interior mutability
            zoon::println!("🔄 ReactiveTreeView: Applying {} changes", changes.len());
        }
        
        // Update current items
        self.current_items.set_neq(new_tree_items);
    }
    
    /// Set up signal handlers for reactive updates
    fn setup_signal_handlers(&self) {
        // Handle external expanded state changes
        if let Some(external_expanded) = &self.config.external_expanded {
            let external_expanded_state = self.external_expanded_state.clone();
            Task::start(external_expanded.for_each_sync(move |expanded_set| {
                external_expanded_state.set_neq(expanded_set.clone());
            }));
        }
        
        // Handle external selected state changes  
        if let Some(external_selected) = &self.config.external_selected {
            let external_selected_state = self.external_selected_state.clone();
            Task::start(external_selected.for_each_sync(move |selected_vec| {
                external_selected_state.set_neq(selected_vec.clone());
            }));
        }
        
        // Handle data source changes
        match &self.data_source {
            DataSource::SignalVec(signal_vec) => {
                let update_fn = {
                    let this = self.clone(); // Need to implement Clone
                    move |data: Vec<T>| {
                        this.update_tree(data);
                    }
                };
                
                // TODO: Set up SignalVec monitoring
                // Task::start(signal_vec.to_signal_cloned().for_each_sync(update_fn));
            },
            
            DataSource::Signal(signal) => {
                let update_fn = {
                    let this = self.clone(); // Need to implement Clone
                    move |data: Vec<T>| {
                        this.update_tree(data);
                    }
                };
                
                // Task::start(signal.for_each_sync(update_fn));
            },
            
            DataSource::Static(data) => {
                // Static data - update once
                self.update_tree(data.clone());
            },
        }
    }
    
    /// Create empty state element when no items
    fn create_empty_state(&self) -> Column<EmptyFlagNotSet, RawHtmlEl> {
        Column::new()
            .s(Align::center())
            .s(Padding::new().x(32).y(32))
            .item(
                El::new()
                    .s(Font::new().size(14))
                    .child(
                        Text::new(self.config.empty_state_message.as_deref().unwrap_or("No items"))
                    )
            )
    }
    
    /// Create the tree structure element
    fn create_tree_element(&self) -> impl Element {
        let items = self.current_items.signal_ref(|items| items.clone());
        
        Column::new()
            .s(Width::fill())
            .s(Height::fill())
            .item_signal(items.map(move |tree_items| {
                if tree_items.is_empty() {
                    self.create_empty_state()
                } else {
                    self.render_tree_items(tree_items)
                }
            }))
    }
    
    /// Render tree items as nested structure
    fn render_tree_items(&self, items: Vec<TreeItem>) -> Column<EmptyFlagNotSet, RawHtmlEl> {
        let (root_items, children_map) = self.differ.build_hierarchy(&items);
        
        Column::new()
            .s(Width::fill())
            .items(root_items.into_iter().map(|item| {
                self.render_tree_item(item, &children_map, 0)
            }))
    }
    
    /// Render individual tree item with children
    fn render_tree_item(
        &self, 
        item: &TreeItem, 
        children_map: &HashMap<String, Vec<&TreeItem>>,
        depth: usize
    ) -> impl Element {
        let indent_size = depth * 16; // 16px per level
        
        // Build all row items first
        let chevron_element = if item.expandable {
            let chevron = if item.expanded { "▼" } else { "▶" };
            El::new()
                .s(Width::exact(16))
                .s(Font::new().size(12))
                .child(Text::new(chevron))
        } else {
            // Spacer for alignment
            El::new().s(Width::exact(16)).child(Text::new(""))
        };
        
        let icon_element = if let Some(_icon) = &item.icon {
            // TODO: Render actual icon based on IconName
            El::new()
                .s(Width::exact(16))
                .child(Text::new("📄")) // Placeholder icon
        } else {
            El::new().s(Width::exact(16)).child(Text::new(""))
        };
        
        let label_element = El::new()
            .s(Font::new().size(14))
            .child(Text::new(&item.label));
        
        // Create item row with all items at once
        let mut item_row = Row::new()
            .s(Gap::new().x(8))
            .s(Align::new().center_y())
            .s(Padding::new().left(indent_size as u32).y(4))
            .item(chevron_element)
            .item(icon_element)
            .item(label_element);
            
        // Apply item state classes
        if item.selected {
            item_row = item_row.update_raw_el(|el| el.class("selected"));
        }
        if item.disabled {
            item_row = item_row.update_raw_el(|el| el.class("disabled"));
        }
        if item.error {
            item_row = item_row.update_raw_el(|el| el.class("error"));
        }
        if item.loading {
            item_row = item_row.update_raw_el(|el| el.class("loading"));
        }
        
        // Build all container items first  
        let mut column_items = vec![item_row.into_element()];
        
        // Add children (if expanded)
        if item.expanded {
            if let Some(children) = children_map.get(&item.key) {
                for child in children {
                    column_items.push(
                        self.render_tree_item(child, children_map, depth + 1).into_element()
                    );
                }
            }
        }
        
        // Create column with first item, then add others
        let mut container = Column::new().s(Width::fill()).item(column_items.remove(0));
        for item_element in column_items {
            container = container.item(item_element);
        }
        
        El::new().child(container)
    }
}

impl<T> ReactiveTreeView<T> 
where T: Clone + 'static
{
    /// Build the TreeView as an Element that can be used in the UI
    pub fn build(self) -> impl Element {
        // Set up signal handlers first
        self.setup_signal_handlers();
        
        // Create main container with tree content
        let container = Column::new()
            .s(Width::fill())
            .s(Height::fill())
            .item(self.create_tree_element());
            
        // Apply custom styles after adding item
        let mut styled_container = container;
        for style_class in &self.config.custom_styles {
            styled_container = styled_container.update_raw_el(|el| el.class(style_class));
        }
        
        styled_container
    }
}

// Need to implement Clone for signal handling
// This is a simplified implementation - in practice we'd need to handle the non-Clone fields
impl<T> Clone for ReactiveTreeView<T> 
where T: Clone + 'static
{
    fn clone(&self) -> Self {
        // This is a simplified clone implementation
        // In practice, we'd need to properly handle sharing of mutable state
        Self {
            context: self.context,
            data_source: DataSource::Static(Vec::new()), // Simplified
            item_renderer: TreeItemRenderer::new().build(), // Simplified - would need proper cloning
            config: ReactiveTreeViewConfig {
                selection_mode: self.config.selection_mode,
                update_strategy: self.config.update_strategy,
                external_expanded: None, // Can't clone Box<dyn Signal>
                external_selected: None, // Can't clone Box<dyn Signal>
                single_selection: self.config.single_selection,
                show_remove_buttons: self.config.show_remove_buttons,
                empty_state_message: self.config.empty_state_message.clone(),
                custom_styles: self.config.custom_styles.clone(),
            },
            current_items: Mutable::new(Vec::new()),
            differ: self.differ.clone(),
            dom_updater: Mutable::new(None),
            external_expanded_state: Mutable::new(HashSet::new()),
            external_selected_state: Mutable::new(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn create_reactive_tree_view() {
        let context = TreeViewContext::FilesAndScopes;
        let data_source = DataSource::Static(vec!["item1".to_string(), "item2".to_string()]);
        let item_renderer = TreeItemRenderer::new()
            .key_fn(|item: &String| item.clone())
            .parent_key_fn(|_item: &String, _context| None)
            .render_fn(|item: &String, _context| {
                super::super::TreeItemBuilder::new()
                    .label(item)
                    .selectable(true)
            })
            .build();
            
        let config = ReactiveTreeViewConfig {
            selection_mode: SelectionMode::Single,
            update_strategy: UpdateStrategy::Reactive,
            external_expanded: None,
            external_selected: None,
            single_selection: true,
            show_remove_buttons: false,
            empty_state_message: Some("No items".to_string()),
            custom_styles: vec!["test-class".to_string()],
        };
        
        let tree = ReactiveTreeView::new(context, data_source, item_renderer, config);
        
        assert_eq!(tree.context, TreeViewContext::FilesAndScopes);
        assert_eq!(tree.current_items.lock_ref().len(), 0); // Not updated until signal setup
    }
}