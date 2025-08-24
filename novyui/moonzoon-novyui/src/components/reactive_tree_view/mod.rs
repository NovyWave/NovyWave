// ReactiveTreeView - Ultra-granular signal-friendly TreeView component
//
// Provides efficient, reactive tree rendering with minimal DOM updates.
// Supports multiple contexts (Files & Scopes, Load Files Dialog) through
// flexible data sources and rendering strategies.

mod builder;
mod component;
mod context;
mod data_source;
mod differ;
mod dom_updater;
mod item_renderer;

// Public API exports
pub use builder::ReactiveTreeViewBuilder;
pub use component::ReactiveTreeView;
pub use context::{TreeViewContext, SelectionMode};
pub use data_source::{DataSource, TreeItem, TreeItemData};
pub use differ::TreeChange;
pub use item_renderer::{TreeItemRenderer, TreeItemBuilder};

// Re-export for builder pattern entry point
pub fn reactive_tree_view<T>() -> ReactiveTreeViewBuilder<T> 
where T: Clone + 'static
{
    ReactiveTreeViewBuilder::new()
}