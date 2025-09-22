use crate::components::*;
use crate::tokens::*;
use indexmap::IndexSet;
use zoon::*;
// Force recompilation to test hover remove buttons

// Tree node data structure matching Vue TreeViewItemData interface
#[derive(Clone)]
pub struct TreeViewItemData {
    pub id: String,
    pub label: String,
    pub children: Option<Vec<TreeViewItemData>>,
    pub icon: Option<String>,
    pub disabled: Option<bool>,
    pub item_type: Option<TreeViewItemType>,
    pub has_expandable_content: Option<bool>,
    pub on_remove: Option<std::rc::Rc<dyn Fn(&str) + 'static>>,
    pub is_waveform_file: Option<bool>,
    pub tooltip: Option<String>, // Hover tooltip text (usually full file path)
    pub error_message: Option<String>, // Error message for problematic files
                                 // Removed styled_label field due to Zoon trait compatibility issues
}

#[derive(Debug, Clone, Copy)]
pub enum TreeViewItemType {
    Folder,
    File,
    FileError, // Files with errors (missing, corrupted, unsupported)
    Default,
}

#[derive(Debug, Clone, Copy)]
pub enum TreeViewSize {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy)]
pub enum TreeViewVariant {
    Basic,
    Bordered,
    Elevated,
}

impl TreeViewItemData {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: None,
            icon: None,
            disabled: None,
            item_type: None,
            has_expandable_content: None,
            on_remove: None,
            is_waveform_file: None,
            tooltip: None,
            error_message: None,
            // styled_label field removed
        }
    }

    pub fn with_children(mut self, children: Vec<TreeViewItemData>) -> Self {
        self.children = Some(children);
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }

    pub fn item_type(mut self, item_type: TreeViewItemType) -> Self {
        self.item_type = Some(item_type);
        self
    }

    pub fn has_expandable_content(mut self, has_expandable_content: bool) -> Self {
        self.has_expandable_content = Some(has_expandable_content);
        self
    }

    pub fn on_remove<F>(mut self, callback: F) -> Self
    where
        F: Fn(&str) + 'static,
    {
        self.on_remove = Some(std::rc::Rc::new(callback));
        self
    }

    pub fn is_waveform_file(mut self, is_waveform_file: bool) -> Self {
        self.is_waveform_file = Some(is_waveform_file);
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn error_message(mut self, error_message: impl Into<String>) -> Self {
        self.error_message = Some(error_message.into());
        self
    }

    // styled_label method removed due to Zoon trait compatibility issues

    pub fn has_children(&self) -> bool {
        // Use has_expandable_content flag if available, otherwise fall back to checking children array
        if let Some(has_expandable) = self.has_expandable_content {
            has_expandable
        } else {
            self.children
                .as_ref()
                .map_or(false, |children| !children.is_empty())
        }
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled.unwrap_or(false)
    }
}

// TreeView builder with signal-based state management
pub struct TreeViewBuilder {
    data: Vec<TreeViewItemData>,
    size: TreeViewSize,
    variant: TreeViewVariant,
    show_icons: bool,
    show_checkboxes: bool,
    show_checkboxes_on_scopes_only: bool,
    single_scope_selection: bool,
    disabled: bool,
    aria_label: Option<String>,
    default_expanded: Vec<String>,
    default_selected: Vec<String>,
    external_expanded: Option<Mutable<IndexSet<String>>>,
    external_selected: Option<Mutable<IndexSet<String>>>,
    external_selected_vec: Option<MutableVec<String>>,
}

impl TreeViewBuilder {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            size: TreeViewSize::Medium,
            variant: TreeViewVariant::Basic,
            show_icons: true,
            show_checkboxes: false,
            show_checkboxes_on_scopes_only: false,
            single_scope_selection: false,
            disabled: false,
            aria_label: None,
            default_expanded: Vec::new(),
            default_selected: Vec::new(),
            external_expanded: None,
            external_selected: None,
            external_selected_vec: None,
        }
    }

    pub fn data(mut self, data: Vec<TreeViewItemData>) -> Self {
        self.data = data;
        self
    }

    pub fn size(mut self, size: TreeViewSize) -> Self {
        self.size = size;
        self
    }

    pub fn variant(mut self, variant: TreeViewVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn show_icons(mut self, show_icons: bool) -> Self {
        self.show_icons = show_icons;
        self
    }

    pub fn show_checkboxes(mut self, show_checkboxes: bool) -> Self {
        self.show_checkboxes = show_checkboxes;
        self
    }

    pub fn show_checkboxes_on_scopes_only(mut self, show_checkboxes_on_scopes_only: bool) -> Self {
        self.show_checkboxes_on_scopes_only = show_checkboxes_on_scopes_only;
        self
    }

    pub fn single_scope_selection(mut self, single_scope_selection: bool) -> Self {
        self.single_scope_selection = single_scope_selection;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn aria_label(mut self, aria_label: impl Into<String>) -> Self {
        self.aria_label = Some(aria_label.into());
        self
    }

    pub fn default_expanded(mut self, expanded: Vec<String>) -> Self {
        self.default_expanded = expanded;
        self
    }

    pub fn default_selected(mut self, selected: Vec<String>) -> Self {
        self.default_selected = selected;
        self
    }

    pub fn external_expanded(mut self, expanded: Mutable<IndexSet<String>>) -> Self {
        self.external_expanded = Some(expanded);
        self
    }

    pub fn external_selected(mut self, selected: Mutable<IndexSet<String>>) -> Self {
        self.external_selected = Some(selected);
        self
    }

    pub fn external_selected_vec(mut self, selected: MutableVec<String>) -> Self {
        self.external_selected_vec = Some(selected);
        self
    }

    pub fn build(self) -> impl Element {
        // Use external state if provided, otherwise create internal state
        let external_expanded_ref = self.external_expanded.clone();
        let expanded_items = if let Some(external) = self.external_expanded {
            external
        } else {
            Mutable::new(IndexSet::from_iter(self.default_expanded.clone()))
        };

        let selected_items = if let Some(external) = self.external_selected {
            external
        } else {
            Mutable::new(IndexSet::from_iter(self.default_selected.clone()))
        };

        // Store external_vec reference separately for direct handling
        let external_selected_vec = self.external_selected_vec;

        let focused_item = Mutable::new(None::<String>);

        let data = self.data;
        let size = self.size;
        let variant = self.variant;
        let show_icons = self.show_icons;
        let show_checkboxes = self.show_checkboxes;
        let show_checkboxes_on_scopes_only = self.show_checkboxes_on_scopes_only;
        let single_scope_selection = self.single_scope_selection;
        let disabled = self.disabled;
        let aria_label = self.aria_label.unwrap_or_else(|| "Tree".to_string());

        // Create the tree container with proper styling
        let tree_container = Column::new()
            .update_raw_el(|raw_el| {
                raw_el
                    .style("width", "100%")
                    .style("min-width", "fit-content")
            })
            .s(Gap::new().y(SPACING_2))
            .items(
                data.into_iter()
                    .map({
                        let expanded_items = expanded_items.clone();
                        let selected_items = selected_items.clone();
                        let focused_item = focused_item.clone();
                        let external_expanded_ref = external_expanded_ref.clone();
                        let external_selected_vec = external_selected_vec.clone();
                        move |item| {
                            render_tree_item(
                                item,
                                0,
                                size,
                                variant,
                                show_icons,
                                show_checkboxes,
                                show_checkboxes_on_scopes_only,
                                single_scope_selection,
                                disabled,
                                expanded_items.clone(),
                                selected_items.clone(),
                                focused_item.clone(),
                                external_expanded_ref.clone(),
                                external_selected_vec.clone(),
                            )
                            .unify()
                        }
                    })
                    .collect::<Vec<_>>(),
            );

        // Apply variant-specific styling
        match variant {
            TreeViewVariant::Basic => tree_container.s(Background::new().color("transparent")),
            TreeViewVariant::Bordered => {
                tree_container
                    .s(Background::new().color_signal(theme().map(|t| match t {
                        Theme::Light => "oklch(100% 0 0)", // neutral_1 light
                        Theme::Dark => "oklch(10% 0 0)",   // neutral_1 dark
                    })))
                    .s(Borders::all_signal(theme().map(|t| match t {
                        Theme::Light => Border::new().width(1).color("oklch(85% 0.14 250)"), // neutral_4 light
                        Theme::Dark => Border::new().width(1).color("oklch(25% 0.14 250)"), // neutral_4 dark
                    })))
                    .s(RoundedCorners::all(4))
                    .s(Padding::all(SPACING_8))
            }
            TreeViewVariant::Elevated => {
                tree_container
                    .s(Background::new().color_signal(theme().map(|t| match t {
                        Theme::Light => "oklch(100% 0 0)", // neutral_1 light
                        Theme::Dark => "oklch(10% 0 0)",   // neutral_1 dark
                    })))
                    .s(Shadows::new([
                        Shadow::new().blur(3).y(1).color("oklch(65% 0.14 250)20"), // neutral_9 with alpha
                    ]))
                    .s(RoundedCorners::all(8))
                    .s(Padding::all(SPACING_8))
            }
        }
        .update_raw_el(move |raw_el| {
            raw_el
                .attr("role", "tree")
                .attr("aria-label", &aria_label)
                .attr("tabindex", "0") // Make tree focusable
        })
        // FUTURE: Add keyboard navigation (arrows, space, enter)
    }
}

// Helper function for stable selection state mutations - works with both IndexSet and MutableVec
fn handle_selection_change(
    item_id: &str,
    selected_items: &Mutable<IndexSet<String>>,
    single_scope_selection: bool,
) {
    let mut selected = selected_items.lock_mut();
    let is_selected = selected.contains(item_id);

    // Handle scope selection logic
    if item_id.starts_with("scope_") {
        // Special handling for scopes when single_scope_selection is enabled
        if single_scope_selection {
            if is_selected {
                // Deselect this scope
                selected.shift_remove(item_id);
            } else {
                // Clear all other scope selections and select this one (radio button behavior)
                selected.retain(|id| !id.starts_with("scope_"));
                selected.insert(item_id.to_string());
            }
        } else {
            // Regular multi-select behavior for scopes
            if is_selected {
                selected.shift_remove(item_id);
            } else {
                selected.insert(item_id.to_string());
            }
        }
    } else {
        // Regular checkbox behavior for non-scope items
        if is_selected {
            selected.shift_remove(item_id);
        } else {
            selected.insert(item_id.to_string());
        }
    }
}

// Helper function for MutableVec selection changes
fn handle_selection_change_vec(
    item_id: &str,
    selected_items_vec: &MutableVec<String>,
    single_scope_selection: bool,
) {
    // quiet
    let mut selected = selected_items_vec.lock_mut();

    // Handle scope selection logic
    if item_id.starts_with("scope_") {
        // Special handling for scopes when single_scope_selection is enabled
        if single_scope_selection {
            if let Some(pos) = selected.iter().position(|id| id == item_id) {
                // Deselect this scope
                selected.remove(pos);
            } else {
                // Clear all other scope selections and select this one (radio button behavior)
                selected.retain(|id| !id.starts_with("scope_"));
                selected.push_cloned(item_id.to_string());
            }
        } else {
            // Regular multi-select behavior for scopes
            if let Some(pos) = selected.iter().position(|id| id == item_id) {
                selected.remove(pos);
            } else {
                selected.push_cloned(item_id.to_string());
            }
        }
    } else {
        // Regular checkbox behavior for non-scope items
        if let Some(pos) = selected.iter().position(|id| id == item_id) {
            selected.remove(pos);
        } else {
            selected.push_cloned(item_id.to_string());
        }
    }

    let _snapshot: Vec<_> = selected.iter().cloned().collect();
}

// Render individual tree item with full functionality
fn render_tree_item(
    item: TreeViewItemData,
    level: u32,
    size: TreeViewSize,
    variant: TreeViewVariant,
    show_icons: bool,
    show_checkboxes: bool,
    show_checkboxes_on_scopes_only: bool,
    single_scope_selection: bool,
    tree_disabled: bool,
    expanded_items: Mutable<IndexSet<String>>,
    selected_items: Mutable<IndexSet<String>>,
    focused_item: Mutable<Option<String>>,
    external_expanded: Option<Mutable<IndexSet<String>>>,
    external_selected_vec: Option<MutableVec<String>>,
) -> impl Element {
    let item_id = item.id.clone();
    let has_children = item.has_children();
    let is_disabled = tree_disabled || item.is_disabled();

    // Clone values needed for closures before moving
    let item_id_for_remove = item_id.clone();
    let item_on_remove = item.on_remove.clone();
    let item_type = item.item_type;

    // Hover state for remove button
    // Hover state tracking for interactive tree item styling
    #[allow(unused_variables)]
    let (hovered, hovered_signal) = Mutable::new_and_signal(false);

    // Calculate indentation based on level
    let indent_width = level * 12; // 12px per level for compact hierarchy spacing

    // Size-dependent values
    // Vertical spacing configuration for consistent tree item height
    #[allow(unused_variables)]
    let (min_height, font_size, padding_y, expand_icon_size) = match size {
        TreeViewSize::Small => (24, FONT_SIZE_14, SPACING_2, 16),
        TreeViewSize::Medium => (26, FONT_SIZE_14, SPACING_2, 18),
        TreeViewSize::Large => (28, FONT_SIZE_14, SPACING_2, 20),
    };

    // Create the tree item row with compact structure - using El to avoid nested buttons
    let item_row = El::new()
        .s(Height::exact(min_height))
        .s(Width::fill())
        .s(Cursor::new(if is_disabled {
            CursorIcon::NotAllowed
        } else {
            CursorIcon::Pointer
        }))
        // Add tooltip support via HTML title attribute
        .update_raw_el({
            let tooltip = item.tooltip.clone();
            move |raw_el| {
                if let Some(tooltip_text) = &tooltip {
                    raw_el.attr("title", tooltip_text)
                } else {
                    raw_el
                }
            }
        })
        .on_hovered_change({
            let hovered = hovered.clone();
            move |is_hovered| {
                hovered.set(is_hovered);
            }
        })
        .child(
            Row::new()
                .s(Height::exact(min_height))
                .s(Width::fill())
                .s(Gap::new().x(SPACING_2))
                .s(Align::new().center_y())
        // Indentation spacer
        .item(
            El::new()
                .s(Width::exact(indent_width))
                .s(Height::exact(1))
        )
        // Expand/collapse button or placeholder
        .item(
            if has_children {
                Button::new()
                    .s(Width::exact(expand_icon_size))
                    .s(Height::exact(expand_icon_size))
                    .s(Padding::all(0))
                    .s(Background::new().color("transparent"))
                    .s(Borders::new())
                    .s(RoundedCorners::all(2))
                    // .s(Align::center())
                    .s(Cursor::new(if is_disabled {
                        CursorIcon::NotAllowed
                    } else {
                        CursorIcon::Pointer
                    }))
                    .label_signal({
                        let item_id = item_id.clone();
                        let external_expanded = external_expanded.clone();
                        let expanded_items = expanded_items.clone();

                        if let Some(external) = external_expanded {
                            external.signal_ref(move |expanded_set| expanded_set.contains(&item_id)).boxed()
                        } else {
                            expanded_items.signal_ref(move |expanded_set| expanded_set.contains(&item_id)).boxed()
                        }.map(move |is_expanded| {
                            IconBuilder::new(if is_expanded { IconName::ChevronDown } else { IconName::ChevronRight })
                                .size(IconSize::Small)
                                .color(if is_disabled {
                                    IconColor::Muted
                                } else {
                                    IconColor::Secondary
                                })
                                .build()
                        })
                    })
                    .on_press_event({
                        let item_id = item_id.clone();
                        let expanded_items = expanded_items.clone();
                        let external_expanded = external_expanded.clone();
                        move |event| {
                            // Prevent event from bubbling up to the row's click handler
                            event.pass_to_parent(false);

                            if !is_disabled {
                                // Use external expansion state if provided, otherwise use internal state
                                if let Some(external) = &external_expanded {
                                    let mut expanded = external.lock_mut();
                                    let was_expanded = expanded.contains(&item_id);
                                    if was_expanded {
                                        expanded.shift_remove(&item_id);
                                    } else {
                                        expanded.insert(item_id.clone());
                                    }
                                } else {
                                    let mut expanded = expanded_items.lock_mut();
                                    let was_expanded = expanded.contains(&item_id);
                                    if was_expanded {
                                        expanded.shift_remove(&item_id);
                                    } else {
                                        expanded.insert(item_id.clone());
                                    }
                                }
                            } else {
                            }
                        }
                    })
                    .unify()
            } else {
                El::new()
                    .s(Width::exact(expand_icon_size))
                    .s(Height::exact(expand_icon_size))
                    .unify()
            }
        )
        // Checkbox (if enabled) - stable element to prevent recreation
        .item({
            // Determine if checkbox should be shown (static decision)
            let should_show_checkbox = show_checkboxes && !is_disabled &&
                if show_checkboxes_on_scopes_only {
                    // Strict mode: show checkboxes ONLY for scopes
                    item_id.starts_with("scope_")
                } else {
                    // Original logic for backwards compatibility
                    if item_id.starts_with("scope_") {
                        // Scopes: YES checkboxes (e.g., "scope_simple_tb")
                        true
                    } else if matches!(item.item_type, Some(TreeViewItemType::Folder)) {
                        // Folders: NO checkboxes for scopes (they're handled above), but could be file picker dirs
                        false
                    } else if matches!(item.item_type, Some(TreeViewItemType::File)) {
                        // Files: different logic based on context
                        if item_id.starts_with("file_") && !item_id.starts_with("scope_") {
                            // Top-level waveform files: NO checkboxes (e.g., "file_71a2908980aee1d")
                            false
                        } else if item_id.starts_with("/") {
                            // File picker paths: use proper is_waveform_file field instead of extension checking
                            item.is_waveform_file.unwrap_or(false)
                        } else {
                            // Signals in Files & Scopes: YES checkboxes (e.g., "A", "B")
                            true
                        }
                    } else {
                        // Other types: NO checkboxes
                        false
                    }
                };

            if should_show_checkbox {
                // Create stable checkbox button that only updates visual state, not structure
                Button::new()
                    .s(Width::exact(20))
                    .s(Height::exact(20))
                    .s(Padding::all(0))
                    .s(Background::new().color("transparent"))
                    .s(Borders::new())
                    .s(Align::new().center_y())
                    .s(Cursor::new(CursorIcon::Pointer))
                    .label_signal(if let Some(ref vec_state) = external_selected_vec {
                        // Use MutableVec signal when provided
                        vec_state.signal_vec_cloned().to_signal_cloned().map({
                            let item_id = item_id.clone();
                            move |selected_vec| {
                                CheckboxBuilder::new()
                                    .size(CheckboxSize::Small)
                                    .checked(selected_vec.contains(&item_id))
                                    .build()
                            }
                        }).boxed_local()
                    } else {
                        // Use IndexSet signal as fallback
                        selected_items.signal_ref({
                            let item_id = item_id.clone();
                            move |selected| {
                                CheckboxBuilder::new()
                                    .size(CheckboxSize::Small)
                                    .checked(selected.contains(&item_id))
                                    .build()
                            }
                        }).boxed_local()
                    })
                    .on_press_event({
                        let item_id = item_id.clone();
                        let selected_items = selected_items.clone();
                        let external_selected_vec = external_selected_vec.clone();
                        move |event| {
                            // Prevent event from bubbling up to the row's click handler
                            event.pass_to_parent(false);

                            if !is_disabled {
                                // Use appropriate handler based on state type
                                if let Some(ref vec_state) = external_selected_vec {
                                    handle_selection_change_vec(&item_id, vec_state, single_scope_selection);
                                } else {
                                    handle_selection_change(&item_id, &selected_items, single_scope_selection);
                                }
                            } else {
                            }
                        }
                    })
                    .unify()
            } else {
                // Empty spacer when no checkbox
                El::new()
                    .s(Width::exact(0))
                    .s(Height::exact(20))
                    .unify()
            }
        })
        // Icon (if enabled)
        .item_signal(always(show_icons).map({
            let item = item.clone();
            let item_id = item_id.clone();
            move |show| {
                if show {
                    // Check if this is a Files & Scope item (no folder icons for these)
                    let is_files_and_scope_item = item_id.starts_with("file_") ||
                                                  item_id.starts_with("scope_") ||
                                                  (!item_id.starts_with("/") && matches!(item.item_type, Some(TreeViewItemType::File)));

                    let icon_name = if let Some(icon) = &item.icon {
                        icon_name_from_str(icon)
                    } else {
                        match item.item_type {
                            Some(TreeViewItemType::Folder) => {
                                if is_files_and_scope_item {
                                    // No folder icons for Files & Scope items
                                    return None;
                                } else if has_children {
                                    IconName::Folder
                                } else {
                                    IconName::Folder
                                }
                            }
                            Some(TreeViewItemType::File) => IconName::File,
                            Some(TreeViewItemType::FileError) => {
                                // Use error-specific icons based on the item's icon field if available
                                if let Some(icon) = &item.icon {
                                    icon_name_from_str(icon)
                                } else {
                                    IconName::CircleAlert // Default error icon
                                }
                            }
                            _ => {
                                if is_files_and_scope_item {
                                    // No folder icons for Files & Scope items
                                    return None;
                                } else if has_children {
                                    IconName::Folder
                                } else {
                                    IconName::File
                                }
                            }
                        }
                    };

                    Some(
                        IconBuilder::new(icon_name)
                            .size(match size {
                                TreeViewSize::Small => IconSize::Small,
                                TreeViewSize::Medium => IconSize::Medium,
                                TreeViewSize::Large => IconSize::Large,
                            })
                            .color(if is_disabled {
                                IconColor::Muted
                            } else {
                                // Special colors for different types
                                match item.item_type {
                                    Some(TreeViewItemType::Folder) => IconColor::Primary, // Primary color for folders
                                    Some(TreeViewItemType::File) => IconColor::Secondary,
                                    Some(TreeViewItemType::FileError) => IconColor::Error, // Error color for problematic files
                                    _ => {
                                        if has_children {
                                            IconColor::Primary
                                        } else {
                                            IconColor::Secondary
                                        }
                                    }
                                }
                            })
                            .no_center_align()
                            .build()
                            .unify()
                    )
                } else {
                    None
                }
            }
        }))
        // Label with proper click handling
        .item(
            Button::new()
                .s(Background::new().color("transparent"))
                .s(Borders::new())
                .s(Padding::new().x(0).y(0))
                .s(Cursor::new(if is_disabled {
                    CursorIcon::NotAllowed
                } else {
                    CursorIcon::Pointer
                }))
                .s(Align::new().left())
                .label(
                    Row::new()
                        .s(Align::new().center_y())
                        .s(Gap::new().x(SPACING_0))
                        .item(
                            El::new()
                                .s(Padding::new().x(SPACING_2))
                                .child({
                                    // Apply inline smart label styling if label contains '/' or timeline info
                                    if item.label.contains('/') && item.label.contains(" (") && item.label.contains("–") && item.label.ends_with(')') {
                                        // Parse labels with BOTH path prefix AND timeline info
                                        if let Some(last_slash) = item.label.rfind('/') {
                                            let prefix = &item.label[..=last_slash]; // Include trailing slash
                                            let rest = &item.label[last_slash + 1..]; // Everything after the slash

                                            if let Some(timeline_start) = rest.rfind(" (") {
                                                let filename = &rest[..timeline_start];
                                                let timeline_info = &rest[timeline_start..];

                                                // Create Paragraph with styled prefix + filename + timeline
                                                zoon::Paragraph::new()
                                                    .content(
                                                        El::new()
                                                            .s(Font::new().color_signal(crate::tokens::color::neutral_8()).no_wrap())
                                                            .child(prefix)
                                                    )
                                                    .content(
                                                        El::new()
                                                            .s(Font::new().color_signal(theme().map(|t| match t {
                                                                crate::tokens::theme::Theme::Light => "oklch(15% 0.14 250)", // neutral_11 light
                                                                crate::tokens::theme::Theme::Dark => "oklch(95% 0.14 250)", // neutral_11 dark
                                                            })).no_wrap())
                                                            .child(filename)
                                                    )
                                                    .content(
                                                        El::new()
                                                            .s(Font::new().color_signal(crate::tokens::color::primary_6()).no_wrap())
                                                            .child(timeline_info)
                                                    )
                                                    .unify()
                                            } else {
                                                // Fallback if timeline parsing fails
                                                Text::new(&item.label).unify()
                                            }
                                        } else {
                                            // Fallback if path parsing fails
                                            Text::new(&item.label).unify()
                                        }
                                    } else if item.label.contains('/') {
                                        // Parse smart label to separate prefix from filename (no timeline)
                                        if let Some(last_slash) = item.label.rfind('/') {
                                            let prefix = &item.label[..=last_slash]; // Include trailing slash
                                            let filename = &item.label[last_slash + 1..];

                                            // Create Paragraph with styled prefix and filename
                                            zoon::Paragraph::new()
                                                .content(
                                                    El::new()
                                                        .s(Font::new().color_signal(crate::tokens::color::neutral_8()).no_wrap())
                                                        .child(prefix)
                                                )
                                                .content(
                                                    El::new()
                                                        .s(Font::new().color_signal(theme().map(|t| match t {
                                                            crate::tokens::theme::Theme::Light => "oklch(15% 0.14 250)", // neutral_11 light
                                                            crate::tokens::theme::Theme::Dark => "oklch(95% 0.14 250)", // neutral_11 dark
                                                        })).no_wrap())
                                                        .child(filename)
                                                )
                                                .unify()
                                        } else {
                                            // Fallback to regular text if parsing fails
                                            Text::new(&item.label).unify()
                                        }
                                    } else if item.label.contains(" (") && item.label.contains("–") && item.label.ends_with(')') {
                                        // Parse timeline info to separate filename from time range
                                        if let Some(timeline_start) = item.label.rfind(" (") {
                                            let filename = &item.label[..timeline_start];
                                            let timeline_info = &item.label[timeline_start..];

                                            // Create Paragraph with filename and dimmed timeline info
                                            zoon::Paragraph::new()
                                                .content(
                                                    El::new()
                                                        .s(Font::new().color_signal(theme().map(|t| match t {
                                                            crate::tokens::theme::Theme::Light => "oklch(15% 0.14 250)", // neutral_11 light
                                                            crate::tokens::theme::Theme::Dark => "oklch(95% 0.14 250)", // neutral_11 dark
                                                        })).no_wrap())
                                                        .child(filename)
                                                )
                                                .content(
                                                    El::new()
                                                        .s(Font::new().color_signal(crate::tokens::color::primary_6()).no_wrap())
                                                        .child(timeline_info)
                                                )
                                                .unify()
                                        } else {
                                            // Fallback to regular text if parsing fails
                                            Text::new(&item.label).unify()
                                        }
                                    } else {
                                        // No prefix or timeline info, use regular text
                                        Text::new(&item.label).unify()
                                    }
                                })
                                .s({
                                    let mut font = Font::new()
                                        .size(font_size)
                                        .weight(FontWeight::Number(FONT_WEIGHT_4))
                                        .no_wrap();

                                    // Only apply color signal for non-styled labels (styled labels handle their own colors)
                                    if !item.label.contains('/') {
                                        let item_id_for_error_check = item_id.clone();
                                        let has_error_message = item.error_message.is_some();
                                        let is_file_error = matches!(item.item_type, Some(TreeViewItemType::FileError));
                                        font = font.color_signal(map_ref! {
                                            let theme = theme(),
                                            let is_selected = selected_items.signal_ref({
                                                let item_id = item_id.clone();
                                                move |selected| selected.contains(&item_id)
                                            }) => {
                                            if item_id_for_error_check == "access_denied" || has_error_message || is_file_error {
                                                match *theme {
                                                    Theme::Light => "oklch(55% 0.16 15)", // Error color light
                                                    Theme::Dark => "oklch(70% 0.16 15)", // Error color dark
                                                }
                                            } else if is_disabled {
                                                match *theme {
                                                    Theme::Light => "oklch(45% 0.14 250)", // neutral_5 light
                                                    Theme::Dark => "oklch(55% 0.14 250)", // neutral_5 dark
                                                }
                                            } else if *is_selected {
                                                match *theme {
                                                    Theme::Light => "oklch(55% 0.22 250)", // primary_7 light
                                                    Theme::Dark => "oklch(65% 0.22 250)", // primary_7 dark
                                                }
                                            } else {
                                                match *theme {
                                                    Theme::Light => "oklch(15% 0.14 250)", // neutral_9 light
                                                    Theme::Dark => "oklch(95% 0.14 250)", // neutral_11 dark
                                                }
                                            }}
                                        });
                                    }
                                    font
                                })
                        )
                        .item_signal(
                            map_ref! {
                                let is_removable_file = always(matches!(item_type, Some(TreeViewItemType::File | TreeViewItemType::FileError))) =>
                                if *is_removable_file && item_on_remove.is_some() {
                                    Some(
                                        Button::new()
                                            .s(Width::exact(16))
                                            .s(Height::exact(16))
                                            .s(Padding::all(0))
                                            .s(Background::new().color("transparent"))
                                            .s(RoundedCorners::all(2))
                                            .s(Borders::new())
                                            .s(Cursor::new(CursorIcon::Pointer))
                                            .s(Align::center())
                                            .label(
                                                IconBuilder::new(IconName::X)
                                                    .size(IconSize::Small)
                                                    .color(IconColor::Error)
                                                    .build()
                                            )
                                            .on_press_event({
                                                let item_id = item_id_for_remove.clone();
                                                let on_remove = item_on_remove.clone();
                                                move |event| {
                                                    event.pass_to_parent(false);
                                                    if let Some(callback) = &on_remove {
                                                        callback(&item_id);
                                                    }
                                                }
                                            })
                                            .unify()
                                    )
                                } else {
                                    None
                                }
                            }
                        )
                ) // Close Row::new()
                .on_press_event({
                    let item_id = item_id.clone();
                    let focused_item = focused_item.clone();
                    let selected_items = selected_items.clone();
                    let external_selected_vec = external_selected_vec.clone();
                    move |event| {

                        if !is_disabled {
                            // Always set focus when clicking a label
                            focused_item.set(Some(item_id.clone()));

                            // Handle selection logic for scope items (regardless of children) or leaf items with checkboxes
                            // and prevent bubbling only in that case
                            let should_handle_selection = show_checkboxes && (item_id.starts_with("scope_") || !has_children);

                            if should_handle_selection {
                                // Prevent event from bubbling up for selection handling
                                event.pass_to_parent(false);

                                // Use appropriate handler based on state type
                                if let Some(ref vec_state) = external_selected_vec {
                                    handle_selection_change_vec(&item_id, vec_state, single_scope_selection);
                                } else {
                                    handle_selection_change(&item_id, &selected_items, single_scope_selection);
                                }
                            } else {
                            }
                        } else {
                        }
                    }
                })
                .unify()
        ) // Close Row::new()
        ) // Close .label()
        // Click handler for entire row (excluding other interactive elements)
        .on_click({
            let item_id = item_id.clone();
            let focused_item = focused_item.clone();
            let expanded_items = expanded_items.clone();
            let external_expanded = external_expanded.clone();
            move || {

                if !is_disabled && has_children {
                    // Set focus when clicking row
                    focused_item.set(Some(item_id.clone()));

                    // Handle expansion/collapse for items with children
                    // Use external expansion state if provided, otherwise use internal state
                    if let Some(external) = &external_expanded {
                        let mut expanded = external.lock_mut();
                        let was_expanded = expanded.contains(&item_id);
                        if was_expanded {
                            expanded.shift_remove(&item_id);
                        } else {
                            expanded.insert(item_id.clone());
                        }
                    } else {
                        let mut expanded = expanded_items.lock_mut();
                        let was_expanded = expanded.contains(&item_id);
                        if was_expanded {
                            expanded.shift_remove(&item_id);
                        } else {
                            expanded.insert(item_id.clone());
                        }
                    }
                } else {
                }
            }
        })
        // Static background for now - eliminate ALL signals causing render loops
        .s(Background::new().color("transparent"))
        // Focus ring (simplified for now)
        .s(Outline::inner().width(0).color("transparent"))
        // ARIA attributes - reactive to actual state
        .update_raw_el({
            move |raw_el| {
                raw_el
                    .attr("role", "treeitem")
                    .attr("aria-level", &(level + 1).to_string())
            }
        })
        // FUTURE: Add dynamic ARIA attributes (aria-expanded, aria-selected)
        .update_raw_el(|raw_el| {
            let mut el = raw_el
                .attr("aria-selected", "false")
                .attr("tabindex", "-1");

            if has_children {
                el = el.attr("aria-expanded", "false");
            }

            el
        });

    // Create the base column with the item row
    let mut column = Column::new().s(Width::growable()).item(item_row);

    // Add children using item_signal for reactivity
    if has_children {
        column = column.item_signal({
            let item_id = item_id.clone();
            let item_children = item.children.clone();
            let expanded_items = expanded_items.clone();
            let external_expanded = external_expanded.clone();

            // Create signal based on expansion source
            let expansion_signal = if let Some(external) = external_expanded.clone() {
                external
                    .signal_ref(move |expanded_set| expanded_set.contains(&item_id))
                    .boxed()
            } else {
                expanded_items
                    .signal_ref(move |expanded_set| expanded_set.contains(&item_id))
                    .boxed()
            };

            expansion_signal.map(move |is_expanded| {
                if is_expanded {
                    Some(
                        Column::new()
                            .s(Width::growable())
                            .items(if let Some(children) = &item_children {
                                children
                                    .iter()
                                    .map({
                                        let expanded_items = expanded_items.clone();
                                        let selected_items = selected_items.clone();
                                        let focused_item = focused_item.clone();
                                        let external_expanded = external_expanded.clone();
                                        let external_selected_vec = external_selected_vec.clone();
                                        move |child| {
                                            render_tree_item(
                                                child.clone(),
                                                level + 1,
                                                size,
                                                variant,
                                                show_icons,
                                                show_checkboxes,
                                                show_checkboxes_on_scopes_only,
                                                single_scope_selection,
                                                tree_disabled,
                                                expanded_items.clone(),
                                                selected_items.clone(),
                                                focused_item.clone(),
                                                external_expanded.clone(),
                                                external_selected_vec.clone(),
                                            )
                                            .unify()
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            } else {
                                Vec::new()
                            })
                            .update_raw_el(|raw_el| raw_el.attr("role", "group"))
                            .into_element(),
                    )
                } else {
                    None
                }
            })
        });
    }

    column
}

// Helper function to convert string to IconName
fn icon_name_from_str(icon: &str) -> IconName {
    match icon {
        "folder" => IconName::Folder,
        "file" => IconName::File,
        "document" => IconName::File,
        "image" => IconName::Image,
        "video" => IconName::File,
        "music" => IconName::File,
        "archive" => IconName::File,
        "code" => IconName::File,
        "settings" => IconName::Settings,
        "user" => IconName::User,
        "users" => IconName::Users,
        "home" => IconName::File,
        "star" => IconName::Star,
        "heart" => IconName::Heart,
        "check" => IconName::Check,
        "x" => IconName::X,
        "plus" => IconName::Plus,
        "minus" => IconName::Minus,
        "edit" => IconName::File,
        "trash" => IconName::Trash,
        "download" => IconName::Download,
        "upload" => IconName::Upload,
        "search" => IconName::Search,
        "filter" => IconName::File,
        "sort" => IconName::File,
        "calendar" => IconName::Calendar,
        "clock" => IconName::Clock,
        "mail" => IconName::Mail,
        "phone" => IconName::Phone,
        "globe" => IconName::File,
        "lock" => IconName::Lock,
        "unlock" => IconName::Lock,
        "eye" => IconName::Eye,
        "eye-off" => IconName::EyeOff,
        "chevron-up" => IconName::ChevronUp,
        "chevron-down" => IconName::ChevronDown,
        "chevron-left" => IconName::ChevronLeft,
        "chevron-right" => IconName::ChevronRight,
        "arrow-up" => IconName::ArrowUp,
        "arrow-down" => IconName::ArrowDown,
        "arrow-left" => IconName::ArrowLeft,
        "arrow-right" => IconName::ArrowRight,
        // Error-specific icons for file states
        "triangle-alert" => IconName::TriangleAlert,
        "circle-alert" => IconName::CircleAlert,
        "circle-help" => IconName::CircleHelp,
        _ => IconName::File, // Default fallback
    }
}

// Convenience functions
pub fn tree_view() -> TreeViewBuilder {
    TreeViewBuilder::new()
}

pub fn tree_view_item(id: impl Into<String>, label: impl Into<String>) -> TreeViewItemData {
    TreeViewItemData::new(id, label)
}

// FUTURE: Add keyboard navigation support
