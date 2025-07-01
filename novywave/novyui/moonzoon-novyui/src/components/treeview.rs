use zoon::*;
use crate::tokens::*;
use crate::components::*;
use std::collections::HashSet;

// Tree node data structure matching Vue TreeViewItemData interface
#[derive(Debug, Clone)]
pub struct TreeViewItemData {
    pub id: String,
    pub label: String,
    pub children: Option<Vec<TreeViewItemData>>,
    pub icon: Option<String>,
    pub disabled: Option<bool>,
    pub item_type: Option<TreeViewItemType>,
}

#[derive(Debug, Clone, Copy)]
pub enum TreeViewItemType {
    Folder,
    File,
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

    pub fn has_children(&self) -> bool {
        self.children.as_ref().map_or(false, |children| !children.is_empty())
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
    disabled: bool,
    aria_label: Option<String>,
    default_expanded: Vec<String>,
    default_selected: Vec<String>,
    external_expanded: Option<Mutable<HashSet<String>>>,
    external_selected: Option<Mutable<HashSet<String>>>,
}

impl TreeViewBuilder {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            size: TreeViewSize::Medium,
            variant: TreeViewVariant::Basic,
            show_icons: true,
            show_checkboxes: false,
            disabled: false,
            aria_label: None,
            default_expanded: Vec::new(),
            default_selected: Vec::new(),
            external_expanded: None,
            external_selected: None,
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

    pub fn external_expanded(mut self, expanded: Mutable<HashSet<String>>) -> Self {
        self.external_expanded = Some(expanded);
        self
    }

    pub fn external_selected(mut self, selected: Mutable<HashSet<String>>) -> Self {
        self.external_selected = Some(selected);
        self
    }

    pub fn build(self) -> impl Element {
        // Use external state if provided, otherwise create internal state
        let expanded_items = if let Some(external) = self.external_expanded {
            external
        } else {
            Mutable::new(HashSet::from_iter(self.default_expanded.clone()))
        };

        let selected_items = if let Some(external) = self.external_selected {
            external
        } else {
            Mutable::new(HashSet::from_iter(self.default_selected.clone()))
        };

        let focused_item = Mutable::new(None::<String>);

        let data = self.data;
        let size = self.size;
        let variant = self.variant;
        let show_icons = self.show_icons;
        let show_checkboxes = self.show_checkboxes;
        let disabled = self.disabled;
        let aria_label = self.aria_label.unwrap_or_else(|| "Tree".to_string());

        // Create the tree container with proper styling
        let tree_container = Column::new()
            .s(Width::fill())
            .s(Gap::new().y(SPACING_2))
            .items_signal_vec(
                always(data.clone()).map({
                    let expanded_items = expanded_items.clone();
                    let selected_items = selected_items.clone();
                    let focused_item = focused_item.clone();
                    move |items| {
                        let expanded_items = expanded_items.clone();
                        let selected_items = selected_items.clone();
                        let focused_item = focused_item.clone();
                        items.into_iter().map({
                            let expanded_items = expanded_items.clone();
                            let selected_items = selected_items.clone();
                            let focused_item = focused_item.clone();
                            move |item| {
                                render_tree_item(
                                    item,
                                    0,
                                    size,
                                    variant,
                                    show_icons,
                                    show_checkboxes,
                                    disabled,
                                    expanded_items.clone(),
                                    selected_items.clone(),
                                    focused_item.clone(),
                                ).unify()
                            }
                        }).collect::<Vec<_>>()
                    }
                }).to_signal_vec()
            );

        // Apply variant-specific styling
        match variant {
            TreeViewVariant::Basic => {
                tree_container
                    .s(Background::new().color_signal(theme().map(|t| match t {
                        Theme::Light => "oklch(100% 0 0)", // neutral_1 light
                        Theme::Dark => "oklch(10% 0 0)", // neutral_1 dark
                    })))
            }
            TreeViewVariant::Bordered => {
                tree_container
                    .s(Background::new().color_signal(theme().map(|t| match t {
                        Theme::Light => "oklch(100% 0 0)", // neutral_1 light
                        Theme::Dark => "oklch(10% 0 0)", // neutral_1 dark
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
                        Theme::Dark => "oklch(10% 0 0)", // neutral_1 dark
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
        // TODO: Add keyboard navigation later
    }
}

// Render individual tree item with full functionality
fn render_tree_item(
    item: TreeViewItemData,
    level: u32,
    size: TreeViewSize,
    variant: TreeViewVariant,
    show_icons: bool,
    show_checkboxes: bool,
    tree_disabled: bool,
    expanded_items: Mutable<HashSet<String>>,
    selected_items: Mutable<HashSet<String>>,
    focused_item: Mutable<Option<String>>,
) -> impl Element {
    let item_id = item.id.clone();
    let has_children = item.has_children();
    let is_disabled = tree_disabled || item.is_disabled();

    // Calculate indentation based on level
    let indent_width = level * 20; // 20px per level like Vue version

    // Size-dependent values
    let (min_height, font_size, padding_y, expand_icon_size) = match size {
        TreeViewSize::Small => (28, FONT_SIZE_14, SPACING_4, 16),
        TreeViewSize::Medium => (32, FONT_SIZE_16, SPACING_4, 18),
        TreeViewSize::Large => (40, FONT_SIZE_16, SPACING_6, 20),
    };

    // Create the tree item row with proper structure:
    // - Outer container: Width::fill() for full-width highlighting
    // - Inner row: Align::left() for proper content alignment
    let item_row = El::new()
        .s(Width::fill())
        .s(Height::exact(min_height))
        .s(Padding::new().y(padding_y))
        .s(RoundedCorners::all(4))
        .s(Cursor::new(if is_disabled {
            CursorIcon::NotAllowed
        } else {
            CursorIcon::Pointer
        }))
        .child(
            Row::new()
                .s(Align::new().left()) // Left-align content within the row
                .s(Gap::new().x(SPACING_4))
                // Indentation
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
                    .s(Cursor::new(if is_disabled {
                        CursorIcon::NotAllowed
                    } else {
                        CursorIcon::Pointer
                    }))
                    .label_signal(
                        expanded_items.signal_ref({
                            let item_id = item_id.clone();
                            move |expanded| {
                                if expanded.contains(&item_id) {
                                    IconBuilder::new(IconName::ChevronDown)
                                        .size(IconSize::Small)
                                        .color(if is_disabled {
                                            IconColor::Muted
                                        } else {
                                            IconColor::Secondary
                                        })
                                        .build()
                                } else {
                                    IconBuilder::new(IconName::ChevronRight)
                                        .size(IconSize::Small)
                                        .color(if is_disabled {
                                            IconColor::Muted
                                        } else {
                                            IconColor::Secondary
                                        })
                                        .build()
                                }
                            }
                        })
                    )
                    .on_press_event({
                        let item_id = item_id.clone();
                        let expanded_items = expanded_items.clone();
                        move |event| {
                            // Prevent event from bubbling up to the row's click handler
                            event.pass_to_parent(false);

                            if !is_disabled {
                                let mut expanded = expanded_items.lock_mut();
                                if expanded.contains(&item_id) {
                                    expanded.remove(&item_id);
                                } else {
                                    expanded.insert(item_id.clone());
                                }
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
        // Checkbox (if enabled) - properly connected to selection state
        .item_signal(
            selected_items.signal_ref({
                let item_id = item_id.clone();
                move |selected| selected.contains(&item_id)
            }).map(move |is_selected| {
                if show_checkboxes {
                    Some(
                        CheckboxBuilder::new()
                            .size(match size {
                                TreeViewSize::Small => CheckboxSize::Small,
                                TreeViewSize::Medium => CheckboxSize::Medium,
                                TreeViewSize::Large => CheckboxSize::Large,
                            })
                            .checked(is_selected)
                            .build()
                            .unify()
                    )
                } else {
                    None
                }
            })
        )
        // Icon (if enabled)
        .item_signal(always(show_icons).map({
            let item = item.clone();
            move |show| {
                if show {
                    let icon_name = if let Some(icon) = &item.icon {
                        icon_name_from_str(icon)
                    } else {
                        match item.item_type {
                            Some(TreeViewItemType::Folder) => {
                                if has_children {
                                    IconName::Folder
                                } else {
                                    IconName::Folder
                                }
                            }
                            Some(TreeViewItemType::File) => IconName::File,
                            _ => {
                                if has_children {
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
                                    _ => {
                                        if has_children {
                                            IconColor::Primary
                                        } else {
                                            IconColor::Secondary
                                        }
                                    }
                                }
                            })
                            .build()
                            .unify()
                    )
                } else {
                    None
                }
            }
        }))
                // Label
                .item(
                    El::new()
                        .child(Text::new(&item.label))
                        .s(Font::new()
                            .size(font_size)
                            .weight(FontWeight::Number(FONT_WEIGHT_4))
                            .color_signal(
                                map_ref! {
                                    let theme = theme(),
                                    let is_selected = selected_items.signal_ref({
                                        let item_id = item_id.clone();
                                        move |selected| selected.contains(&item_id)
                                    }),
                                    let is_focused = focused_item.signal_ref({
                                        let item_id = item_id.clone();
                                        move |focused| focused.as_ref() == Some(&item_id)
                                    }) =>
                                    if is_disabled {
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
                                    }
                                }
                            )
                        )
                        .s(Align::new().left()) // Consistent left alignment
                )
        )
        // Background and interaction styling
        .s(Background::new().color_signal(
            map_ref! {
                let theme = theme(),
                let is_selected = selected_items.signal_ref({
                    let item_id = item_id.clone();
                    move |selected| selected.contains(&item_id)
                }),
                let is_focused = focused_item.signal_ref({
                    let item_id = item_id.clone();
                    move |focused| focused.as_ref() == Some(&item_id)
                }) =>
                // Only show selection background when checkboxes are enabled
                if show_checkboxes && *is_selected {
                    match *theme {
                        Theme::Light => "oklch(92% 0.045 255)", // neutral_3 light - much more subtle
                        Theme::Dark => "oklch(30% 0.045 255)", // neutral_3 dark - much more subtle
                    }
                } else if *is_focused {
                    match *theme {
                        Theme::Light => "oklch(97% 0.025 255)", // neutral_2 light - subtle
                        Theme::Dark => "oklch(25% 0.045 255)", // neutral_3 dark - more visible
                    }
                } else {
                    "transparent"
                }
            }
        ))
        // Focus ring (simplified for now)
        .s(Outline::inner().width(0).color("transparent"))
        // Click handler - improved interaction logic
        .on_click({
            let item_id = item_id.clone();
            let focused_item = focused_item.clone();
            let selected_items = selected_items.clone();
            let expanded_items = expanded_items.clone();
            move || {
                if !is_disabled {
                    // Always set focus when clicking a row
                    focused_item.set(Some(item_id.clone()));

                    // Handle interaction based on mode and item type
                    if show_checkboxes {
                        // In checkbox mode: row clicks ONLY toggle selection
                        let mut selected = selected_items.lock_mut();
                        if selected.contains(&item_id) {
                            selected.remove(&item_id);
                        } else {
                            selected.insert(item_id.clone());
                        }
                        // Note: expand/collapse is handled separately by the expand button
                    } else {
                        // In normal mode (no checkboxes):
                        // - Row clicks ONLY expand/collapse if it has children
                        // - No selection state is maintained
                        if has_children {
                            let mut expanded = expanded_items.lock_mut();
                            if expanded.contains(&item_id) {
                                expanded.remove(&item_id);
                            } else {
                                expanded.insert(item_id.clone());
                            }
                        }
                    }
                }
            }
        })
        // ARIA attributes - reactive to actual state
        .update_raw_el({
            let item_id = item_id.clone();
            let expanded_items = expanded_items.clone();
            let selected_items = selected_items.clone();
            let focused_item = focused_item.clone();
            move |raw_el| {
                raw_el
                    .attr("role", "treeitem")
                    .attr("aria-level", &(level + 1).to_string())
            }
        })
        // TODO: Add dynamic ARIA attributes later
        .update_raw_el(|raw_el| {
            let mut el = raw_el
                .attr("aria-selected", "false")
                .attr("tabindex", "-1");

            if has_children {
                el = el.attr("aria-expanded", "false");
            }

            el
        });

    // Create children container if item has children and is expanded
    let children_container = if has_children {
        Some(
            Column::new()
                .s(Width::fill())
                .items_signal_vec(
                    expanded_items.signal_ref({
                        let item_id = item_id.clone();
                        let item = item.clone();
                        move |expanded| {
                            if expanded.contains(&item_id) {
                                if let Some(children) = &item.children {
                                    children.clone()
                                } else {
                                    Vec::new()
                                }
                            } else {
                                Vec::new()
                            }
                        }
                    }).map({
                        let expanded_items = expanded_items.clone();
                        let selected_items = selected_items.clone();
                        let focused_item = focused_item.clone();
                        move |children| {
                            let expanded_items = expanded_items.clone();
                            let selected_items = selected_items.clone();
                            let focused_item = focused_item.clone();
                            children.into_iter().map({
                                let expanded_items = expanded_items.clone();
                                let selected_items = selected_items.clone();
                                let focused_item = focused_item.clone();
                                move |child| {
                                    render_tree_item(
                                        child,
                                        level + 1,
                                        size,
                                        variant,
                                        show_icons,
                                        show_checkboxes,
                                        tree_disabled,
                                        expanded_items.clone(),
                                        selected_items.clone(),
                                        focused_item.clone(),
                                    ).unify()
                                }
                            }).collect::<Vec<_>>()
                        }
                    }).to_signal_vec()
                )
                .update_raw_el(|raw_el| {
                    raw_el.attr("role", "group")
                })
        )
    } else {
        None
    };

    // Combine item row and children
    if let Some(children) = children_container {
        Column::new()
            .s(Width::fill())
            .item(item_row)
            .item(children)
    } else {
        Column::new()
            .s(Width::fill())
            .item(item_row)
    }
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

// TODO: Add keyboard navigation later
