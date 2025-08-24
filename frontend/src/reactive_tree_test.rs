// ReactiveTreeView testing integration - SIMPLIFIED FOR INITIAL TESTING
// Simple placeholder while we fix compilation issues

use zoon::*;
use shared::TrackedFile;
// Temporarily commented out complex imports
// use novyui::{reactive_tree_view, TreeViewContext, DataSource, TreeItemRenderer};

/// Simple ReactiveTreeView prototype - demonstrates efficient signal-based updates
pub fn create_test_reactive_tree_view() -> impl Element {
    use crate::state::TRACKED_FILES;
    
    Column::new()
        .s(Width::fill())
        .s(Height::fill())
        .s(Padding::all(8))
        .item(
            El::new()
                .s(Font::new().size(12).weight(FontWeight::Bold))
                .child(Text::new("🚀 ReactiveTreeView [Working Prototype]"))
        )
        .item(
            El::new()
                .s(Font::new().size(10))
                .child(Text::new("Signal-based efficient updates"))
        )
        .item(
            // Dynamic list of tracked files - updates automatically
            Column::new()
                .s(Gap::new().y(4))
                .s(Padding::new().top(12))
                .items_signal_vec(
                    crate::state::stable_tracked_files_signal_vec().map(|file| {
                        create_simple_tree_item(file.smart_label.clone(), file.path.clone())
                    })
                )
        )
}

/// Creates a simple tree item (file entry)
fn create_simple_tree_item(label: String, path: String) -> impl Element {
    zoon::println!("🔨 [ReactiveTreeView] RENDERING item: {}", label); // Debug logging
    
    Row::new()
        .s(Gap::new().x(8))
        .s(Padding::new().y(2).x(8))
        .s(RoundedCorners::all(4))
        .item(
            El::new()
                .s(Font::new().size(10))
                .child(Text::new("📄"))
        )
        .item(
            Column::new()
                .item(
                    El::new()
                        .s(Font::new().size(11).weight(FontWeight::Medium))
                        .child(Text::new(&label))
                )
                .item(
                    El::new()
                        .s(Font::new().size(9))
                        .child(Text::new(&path))
                )
        )
}

/// Simple debug element showing TRACKED_FILES count
pub fn create_debug_info() -> impl Element {
    use crate::state::TRACKED_FILES;
    
    Row::new()
        .s(Gap::new().x(16))
        .s(Padding::all(8))
        .item(
            El::new()
                .s(Font::new().size(12))
                .child_signal(TRACKED_FILES.signal_vec_cloned().len().map(|count| {
                    Text::new(&format!("📊 TRACKED_FILES count: {}", count))
                }))
        )
}