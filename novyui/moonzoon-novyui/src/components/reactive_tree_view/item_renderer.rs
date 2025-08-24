use zoon::*;
// Temporarily comment out icon integration until compilation works
// use crate::components::icon::{Icon, IconName};
use super::{TreeItem, TreeItemData, TreeViewContext};

/// Flexible tree item renderer that can adapt to different contexts
pub struct TreeItemRenderer<T> {
    key_fn: Box<dyn Fn(&T) -> String>,
    parent_key_fn: Box<dyn Fn(&T, TreeViewContext) -> Option<String>>,
    render_fn: Box<dyn Fn(&T, TreeViewContext) -> TreeItemBuilder>,
}

impl<T> TreeItemRenderer<T> {
    pub fn new() -> TreeItemRendererBuilder<T> {
        TreeItemRendererBuilder::new()
    }
    
    /// Get unique key for item (used for diffing)
    pub fn get_key(&self, item: &T) -> String {
        (self.key_fn)(item)
    }
    
    /// Get parent key for hierarchy
    pub fn get_parent_key(&self, item: &T, context: TreeViewContext) -> Option<String> {
        (self.parent_key_fn)(item, context)
    }
    
    /// Render item to TreeItemBuilder
    pub fn render(&self, item: &T, context: TreeViewContext) -> TreeItemBuilder {
        (self.render_fn)(item, context)
    }
}

/// Builder for TreeItemRenderer configuration
pub struct TreeItemRendererBuilder<T> {
    key_fn: Option<Box<dyn Fn(&T) -> String>>,
    parent_key_fn: Option<Box<dyn Fn(&T, TreeViewContext) -> Option<String>>>,
    render_fn: Option<Box<dyn Fn(&T, TreeViewContext) -> TreeItemBuilder>>,
}

impl<T> TreeItemRendererBuilder<T> {
    pub fn new() -> Self {
        Self {
            key_fn: None,
            parent_key_fn: None,
            render_fn: None,
        }
    }
    
    /// Set the key extraction function
    pub fn key_fn<F>(mut self, f: F) -> Self
    where F: Fn(&T) -> String + 'static
    {
        self.key_fn = Some(Box::new(f));
        self
    }
    
    /// Set the parent key extraction function
    pub fn parent_key_fn<F>(mut self, f: F) -> Self
    where F: Fn(&T, TreeViewContext) -> Option<String> + 'static
    {
        self.parent_key_fn = Some(Box::new(f));
        self
    }
    
    /// Set the rendering function
    pub fn render_fn<F>(mut self, f: F) -> Self
    where F: Fn(&T, TreeViewContext) -> TreeItemBuilder + 'static
    {
        self.render_fn = Some(Box::new(f));
        self
    }
    
    /// Build the renderer (consumes builder)
    pub fn build(self) -> TreeItemRenderer<T> {
        TreeItemRenderer {
            key_fn: self.key_fn.expect("key_fn is required"),
            parent_key_fn: self.parent_key_fn.expect("parent_key_fn is required"),
            render_fn: self.render_fn.expect("render_fn is required"),
        }
    }
}

/// Builder for individual tree item visual representation
/// Provides fluent API for configuring tree item appearance
pub struct TreeItemBuilder {
    label: String,
    icon: Option<String>, // Simplified to string for now
    tooltip: Option<String>,
    expandable: bool,
    expanded: bool,
    selectable: bool,
    selected: bool,
    disabled: bool,
    error: bool,
    loading: bool,
    custom_classes: Vec<String>,
    on_click: Option<Box<dyn Fn()>>,
    on_expand: Option<Box<dyn Fn()>>,
    on_select: Option<Box<dyn Fn()>>,
}

impl TreeItemBuilder {
    pub fn new() -> Self {
        Self {
            label: String::new(),
            icon: None,
            tooltip: None,
            expandable: false,
            expanded: false,
            selectable: true,
            selected: false,
            disabled: false,
            error: false,
            loading: false,
            custom_classes: Vec::new(),
            on_click: None,
            on_expand: None,
            on_select: None,
        }
    }
    
    /// Set the display label
    pub fn label<S: Into<String>>(mut self, label: S) -> Self {
        self.label = label.into();
        self
    }
    
    /// Set the icon
    pub fn icon(mut self, icon: String) -> Self {
        self.icon = Some(icon);
        self
    }
    
    /// Set tooltip text
    pub fn tooltip<S: Into<String>>(mut self, tooltip: S) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
    
    /// Set if item can be expanded (has children)
    pub fn expandable(mut self, expandable: bool) -> Self {
        self.expandable = expandable;
        self
    }
    
    /// Set if item is currently expanded
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }
    
    /// Set if item can be selected
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }
    
    /// Set if item is currently selected
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
    
    /// Set if item is disabled
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    
    /// Set if item has error state
    pub fn error(mut self, error: bool) -> Self {
        self.error = error;
        self
    }
    
    /// Set if item is in loading state
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
    
    /// Add custom CSS class
    pub fn custom_class<S: Into<String>>(mut self, class: S) -> Self {
        self.custom_classes.push(class.into());
        self
    }
    
    /// Set click handler
    pub fn on_click<F>(mut self, f: F) -> Self
    where F: Fn() + 'static
    {
        self.on_click = Some(Box::new(f));
        self
    }
    
    /// Set expand handler
    pub fn on_expand<F>(mut self, f: F) -> Self  
    where F: Fn() + 'static
    {
        self.on_expand = Some(Box::new(f));
        self
    }
    
    /// Set selection handler
    pub fn on_select<F>(mut self, f: F) -> Self
    where F: Fn() + 'static
    {
        self.on_select = Some(Box::new(f));
        self
    }
    
    // Getter methods for private fields
    pub fn get_label(&self) -> &str {
        &self.label
    }
    
    pub fn get_icon(&self) -> &Option<String> {
        &self.icon
    }
    
    pub fn get_tooltip(&self) -> &Option<String> {
        &self.tooltip
    }
    
    pub fn is_expandable(&self) -> bool {
        self.expandable
    }
    
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }
    
    pub fn is_selectable(&self) -> bool {
        self.selectable
    }
    
    pub fn is_selected(&self) -> bool {
        self.selected
    }
    
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
    
    pub fn has_error(&self) -> bool {
        self.error
    }
    
    pub fn is_loading(&self) -> bool {
        self.loading
    }
    
    /// Convert to Zoon Element for rendering
    pub fn build(self) -> impl Element {
        // Build all components first
        let chevron = if self.expandable {
            let chevron_text = if self.expanded { "▼" } else { "▶" };
            El::new()
                .s(Width::exact(16))
                .s(Font::new().size(12))
                .child(Text::new(chevron_text))
                .into_element()
        } else {
            El::new().s(Width::exact(16)).child(Text::new("")).into_element()
        };
        
        let icon_element = if let Some(icon) = &self.icon {
            El::new()
                .s(Width::exact(16))
                .s(Font::new().size(12))
                .child(Text::new(icon))
                .into_element()
        } else {
            El::new().s(Width::exact(0)).child(Text::new("")).into_element()
        };
        
        let label_element = {
            let mut label_el = El::new().child(Text::new(&self.label));
            if self.disabled {
                label_el = label_el.update_raw_el(|el| el.class("disabled-text"));
            }
            label_el.into_element()
        };
        
        // Create container with all items at once
        let mut container = Row::new()
            .s(Gap::new().x(8))
            .s(Align::new().center_y())
            .s(Padding::new().x(8).y(4))
            .item(chevron)
            .item(icon_element)
            .item(label_element);
            
        // Apply custom classes
        for class in self.custom_classes {
            container = container.update_raw_el(move |el| el.class(&class));
        }
        
        // Apply state classes
        if self.selected {
            container = container.update_raw_el(|el| el.class("selected"));
        }
        if self.disabled {
            container = container.update_raw_el(|el| el.class("disabled"));
        }
        if self.error {
            container = container.update_raw_el(|el| el.class("error"));
        }
        if self.loading {
            container = container.update_raw_el(|el| el.class("loading"));
        }
        
        // Apply click handler if selectable
        if self.selectable && !self.disabled {
            // Combine both handlers into single click handler
            if let Some(on_click) = self.on_click {
                if let Some(on_select) = self.on_select {
                    container = container.on_click(move || {
                        on_click();
                        on_select();
                    });
                } else {
                    container = container.on_click(move || on_click());
                }
            } else if let Some(on_select) = self.on_select {
                container = container.on_click(move || on_select());
            }
        }
        
        // Apply tooltip
        if let Some(tooltip) = self.tooltip {
            container = container.update_raw_el(move |el| el.attr("title", &tooltip));
        }
        
        container
    }
}

impl Default for TreeItemBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn tree_item_builder_defaults() {
        let builder = TreeItemBuilder::new();
        assert_eq!(builder.label, "");
        assert_eq!(builder.icon, None);
        assert!(!builder.expandable);
        assert!(builder.selectable);
        assert!(!builder.selected);
    }
    
    #[test]
    fn tree_item_builder_chaining() {
        let builder = TreeItemBuilder::new()
            .label("Test Item")
            .icon("📄".to_string())
            .tooltip("Test tooltip")
            .expandable(true)
            .expanded(false)
            .selectable(true);
            
        assert_eq!(builder.label, "Test Item");
        assert_eq!(builder.icon, Some("📄".to_string()));
        assert_eq!(builder.tooltip, Some("Test tooltip".to_string()));
        assert!(builder.expandable);
        assert!(!builder.expanded);
        assert!(builder.selectable);
    }
}