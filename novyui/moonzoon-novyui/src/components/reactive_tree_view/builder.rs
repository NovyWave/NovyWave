use zoon::*;
use std::marker::PhantomData;
use super::{
    ReactiveTreeView, 
    TreeViewContext, 
    SelectionMode, 
    UpdateStrategy,
    DataSource,
    TreeItemRenderer,
};

/// Builder for configuring ReactiveTreeView with fluent API
pub struct ReactiveTreeViewBuilder<T> {
    context: Option<TreeViewContext>,
    data_source: Option<DataSource<T>>,
    item_renderer: Option<TreeItemRenderer<T>>,
    selection_mode: Option<SelectionMode>,
    update_strategy: Option<UpdateStrategy>,
    external_expanded: Option<Box<dyn Signal<Item = std::collections::HashSet<String>> + Unpin>>,
    external_selected: Option<Box<dyn Signal<Item = Vec<String>> + Unpin>>,
    single_selection: bool,
    show_remove_buttons: bool,
    empty_state_message: Option<String>,
    custom_styles: Vec<String>,
    phantom: PhantomData<T>,
}

impl<T: 'static + Clone> ReactiveTreeViewBuilder<T> {
    pub fn new() -> Self {
        Self {
            context: None,
            data_source: None,
            item_renderer: None,
            selection_mode: None,
            update_strategy: None,
            external_expanded: None,
            external_selected: None,
            single_selection: false,
            show_remove_buttons: false,
            empty_state_message: None,
            custom_styles: Vec::new(),
            phantom: PhantomData,
        }
    }
    
    /// Set the tree view context (determines default behaviors)
    pub fn context(mut self, context: TreeViewContext) -> Self {
        self.context = Some(context);
        self
    }
    
    /// Set the data source for tree items
    pub fn data_source(mut self, source: DataSource<T>) -> Self {
        self.data_source = Some(source);
        self
    }
    
    /// Set the item renderer for converting data to visual representation
    pub fn item_renderer(mut self, renderer: TreeItemRenderer<T>) -> Self {
        self.item_renderer = Some(renderer);
        self
    }
    
    /// Set selection mode (overrides context default)
    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = Some(mode);
        self
    }
    
    /// Set update strategy (overrides context default)
    pub fn update_strategy(mut self, strategy: UpdateStrategy) -> Self {
        self.update_strategy = Some(strategy);
        self
    }
    
    /// Set external expanded state signal
    pub fn external_expanded<S>(mut self, signal: S) -> Self
    where S: Signal<Item = std::collections::HashSet<String>> + Unpin + 'static
    {
        self.external_expanded = Some(Box::new(signal));
        self
    }
    
    /// Set external selected state signal
    pub fn external_selected<S>(mut self, signal: S) -> Self  
    where S: Signal<Item = Vec<String>> + Unpin + 'static
    {
        self.external_selected = Some(Box::new(signal));
        self
    }
    
    /// Enable single selection mode (legacy TreeView compatibility)
    pub fn single_selection(mut self, enabled: bool) -> Self {
        self.single_selection = enabled;
        if enabled {
            self.selection_mode = Some(SelectionMode::Single);
        }
        self
    }
    
    /// Show remove buttons on items  
    pub fn show_remove_buttons(mut self, show: bool) -> Self {
        self.show_remove_buttons = show;
        self
    }
    
    /// Set message to display when tree is empty
    pub fn empty_state_message<S: Into<String>>(mut self, message: S) -> Self {
        self.empty_state_message = Some(message.into());
        self
    }
    
    /// Add custom CSS class
    pub fn custom_class<S: Into<String>>(mut self, class: S) -> Self {
        self.custom_styles.push(class.into());
        self
    }
    
    /// Build the ReactiveTreeView component
    pub fn build(self) -> ReactiveTreeView<T> {
        let context = self.context.unwrap_or(TreeViewContext::FilesAndScopes);
        
        ReactiveTreeView::new(
            context,
            self.data_source.expect("data_source is required"),
            self.item_renderer.expect("item_renderer is required"),
            ReactiveTreeViewConfig {
                selection_mode: self.selection_mode.unwrap_or_else(|| context.default_selection_mode()),
                update_strategy: self.update_strategy.unwrap_or_else(|| context.default_update_strategy()),
                external_expanded: self.external_expanded,
                external_selected: self.external_selected,
                single_selection: self.single_selection,
                show_remove_buttons: self.show_remove_buttons,
                empty_state_message: self.empty_state_message,
                custom_styles: self.custom_styles,
            }
        )
    }
}

/// Configuration parameters for ReactiveTreeView
pub struct ReactiveTreeViewConfig {
    pub selection_mode: SelectionMode,
    pub update_strategy: UpdateStrategy,
    pub external_expanded: Option<Box<dyn Signal<Item = std::collections::HashSet<String>> + Unpin>>,
    pub external_selected: Option<Box<dyn Signal<Item = Vec<String>> + Unpin>>,
    pub single_selection: bool,
    pub show_remove_buttons: bool,
    pub empty_state_message: Option<String>,
    pub custom_styles: Vec<String>,
}

// Convenience functions for common configurations

impl<T: 'static + Clone> ReactiveTreeViewBuilder<T> {
    /// Configure for Files & Scopes panel usage
    pub fn for_files_and_scopes(self) -> Self {
        self.context(TreeViewContext::FilesAndScopes)
            .selection_mode(SelectionMode::Single)
            .single_selection(true)
            .show_remove_buttons(true)
            .empty_state_message("Click 'Load Files' to add waveform files.")
    }
    
    /// Configure for Load Files dialog usage
    pub fn for_load_files(self) -> Self {
        self.context(TreeViewContext::LoadFiles)
            .selection_mode(SelectionMode::Multiple)
            .show_remove_buttons(false)
            .empty_state_message("Browse to select waveform files (.vcd, .fst)")
    }
}

impl<T: 'static + Clone> Default for ReactiveTreeViewBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::reactive_tree_view::{TreeItemRenderer, DataSource};
    
    #[test]
    fn builder_defaults() {
        let builder = ReactiveTreeViewBuilder::<String>::new();
        
        assert!(builder.context.is_none());
        assert!(builder.data_source.is_none());
        assert!(builder.selection_mode.is_none());
        assert!(!builder.single_selection);
        assert!(!builder.show_remove_buttons);
    }
    
    #[test]  
    fn builder_chaining() {
        let builder = ReactiveTreeViewBuilder::<String>::new()
            .context(TreeViewContext::FilesAndScopes)
            .selection_mode(SelectionMode::Multiple)
            .single_selection(true)
            .show_remove_buttons(true)
            .empty_state_message("Test message")
            .custom_class("test-class");
            
        assert_eq!(builder.context, Some(TreeViewContext::FilesAndScopes));
        assert_eq!(builder.selection_mode, Some(SelectionMode::Multiple));
        assert!(builder.single_selection);
        assert!(builder.show_remove_buttons);
        assert_eq!(builder.empty_state_message, Some("Test message".to_string()));
        assert_eq!(builder.custom_styles, vec!["test-class"]);
    }
    
    #[test]
    fn convenience_configurations() {
        let files_builder = ReactiveTreeViewBuilder::<String>::new()
            .for_files_and_scopes();
            
        assert_eq!(files_builder.context, Some(TreeViewContext::FilesAndScopes));
        assert_eq!(files_builder.selection_mode, Some(SelectionMode::Single));
        assert!(files_builder.single_selection);
        assert!(files_builder.show_remove_buttons);
        
        let load_files_builder = ReactiveTreeViewBuilder::<String>::new()
            .for_load_files();
            
        assert_eq!(load_files_builder.context, Some(TreeViewContext::LoadFiles));
        assert_eq!(load_files_builder.selection_mode, Some(SelectionMode::Multiple));
        assert!(!load_files_builder.single_selection);
        assert!(!load_files_builder.show_remove_buttons);
    }
}