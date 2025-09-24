use moonzoon_novyui::tokens::color::{
    neutral_2, neutral_4, neutral_11, primary_3, primary_6, primary_7,
};
use moonzoon_novyui::*;
use zoon::*;
use zoon::{PointerEvent, RawPointerEvent};

/// Create a standard panel with header and content sections
pub fn create_panel(header_content: impl Element, content: impl Element) -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Width::growable())
        .s(Scrollbars::both())
        .s(Background::new().color_signal(neutral_2()))
        .update_raw_el(|raw_el| raw_el.style("scrollbar-width", "thin"))
        .s(Borders::all_signal(
            neutral_4().map(|color| Border::new().width(1).color(color)),
        ))
        .child(
            Column::new()
                .s(Height::fill())
                .s(Scrollbars::both())
                .update_raw_el(|raw_el| raw_el.style("scrollbar-width", "thin"))
                .item(
                    El::new()
                        .s(Padding::new().x(SPACING_12).y(SPACING_4))
                        .s(Background::new().color_signal(neutral_4()))
                        .s(Borders::new().bottom_signal(
                            neutral_4().map(|color| Border::new().width(1).color(color)),
                        ))
                        .s(Font::new()
                            .weight(FontWeight::SemiBold)
                            .size(14)
                            .color_signal(neutral_11()))
                        .child(header_content),
                )
                .item(
                    El::new()
                        .s(Height::fill())
                        .s(Width::fill())
                        .s(Scrollbars::both())
                        .update_raw_el(|raw_el| {
                            raw_el
                                .style("scrollbar-width", "thin")
                                .style("overflow-x", "auto")
                                .style("min-height", "0")
                                .style_signal(
                                    "scrollbar-color",
                                    primary_6()
                                        .map(|thumb| {
                                            primary_3()
                                                .map(move |track| format!("{} {}", thumb, track))
                                        })
                                        .flatten(),
                                )
                        })
                        .child(content),
                ),
        )
}

/// Vertical divider for variables name column
pub fn variables_name_vertical_divider(
    _app_config: &crate::config::AppConfig,
    dragging_system: crate::dragging::DraggingSystem,
) -> impl Element {
    use crate::dragging::{DividerType, start_drag};

    // Use static appearance for now - dragging state will be handled at application level
    let is_dragging_signal = zoon::always(false);

    El::new()
        .s(Width::exact(4))
        .s(Height::fill())
        .s(Background::new()
            .color_signal(is_dragging_signal.map_bool_signal(|| primary_7(), || primary_6())))
        .s(Cursor::new(CursorIcon::ColumnResize))
        .s(Padding::all(0))
        .on_pointer_down_event({
            let dragging_system = dragging_system.clone();
            move |event: PointerEvent| {
                let raw_pointer_down = match &event.raw_event {
                    RawPointerEvent::PointerDown(raw_event) => raw_event,
                    _ => return,
                };

                if raw_pointer_down.button() != events::MouseButton::Left {
                    return;
                }

                start_drag(
                    &dragging_system,
                    DividerType::VariablesNameColumn,
                    (event.x() as f32, event.y() as f32),
                );
            }
        })
}

/// Vertical divider for variables value column
pub fn variables_value_vertical_divider(
    _app_config: &crate::config::AppConfig,
    dragging_system: crate::dragging::DraggingSystem,
) -> impl Element {
    use crate::dragging::{DividerType, start_drag};

    // Use static appearance for now - dragging state will be handled at application level
    let is_dragging_signal = zoon::always(false);

    El::new()
        .s(Width::exact(4))
        .s(Height::fill())
        .s(Background::new()
            .color_signal(is_dragging_signal.map_bool_signal(|| primary_7(), || primary_6())))
        .s(Cursor::new(CursorIcon::ColumnResize))
        .s(Padding::all(0))
        .on_pointer_down_event({
            let dragging_system = dragging_system.clone();
            move |event: PointerEvent| {
                let raw_pointer_down = match &event.raw_event {
                    RawPointerEvent::PointerDown(raw_event) => raw_event,
                    _ => return,
                };

                if raw_pointer_down.button() != events::MouseButton::Left {
                    return;
                }

                start_drag(
                    &dragging_system,
                    DividerType::VariablesValueColumn,
                    (event.x() as f32, event.y() as f32),
                );
            }
        })
}

/// Vertical divider for files panel main section
pub fn files_panel_vertical_divider(
    _app_config: &crate::config::AppConfig,
    dragging_system: crate::dragging::DraggingSystem,
) -> impl Element {
    use crate::dragging::{DividerType, start_drag};

    // Use static appearance for now - dragging state will be handled at application level
    let is_dragging_signal = zoon::always(false);

    El::new()
        .s(Width::exact(4))
        .s(Height::fill())
        .s(Background::new()
            .color_signal(is_dragging_signal.map_bool_signal(|| primary_7(), || primary_6())))
        .s(Cursor::new(CursorIcon::ColumnResize))
        .s(Padding::all(0))
        .on_pointer_down_event({
            let dragging_system = dragging_system.clone();
            move |event: PointerEvent| {
                let raw_pointer_down = match &event.raw_event {
                    RawPointerEvent::PointerDown(raw_event) => raw_event,
                    _ => return,
                };

                if raw_pointer_down.button() != events::MouseButton::Left {
                    return;
                }

                start_drag(
                    &dragging_system,
                    DividerType::FilesPanelMain,
                    (event.x() as f32, event.y() as f32),
                );
            }
        })
}

/// Horizontal divider for files panel secondary section
pub fn files_panel_horizontal_divider(
    _app_config: &crate::config::AppConfig,
    dragging_system: crate::dragging::DraggingSystem,
) -> impl Element {
    use crate::dragging::{DividerType, start_drag};

    // Use static appearance for now - dragging state will be handled at application level
    let is_dragging_signal = zoon::always(false);

    El::new()
        .s(Width::fill())
        .s(Height::exact(4))
        .s(Background::new()
            .color_signal(is_dragging_signal.map_bool_signal(|| primary_7(), || primary_6())))
        .s(Cursor::new(CursorIcon::RowResize))
        .on_pointer_down_event({
            let dragging_system = dragging_system.clone();
            move |event: PointerEvent| {
                let raw_pointer_down = match &event.raw_event {
                    RawPointerEvent::PointerDown(raw_event) => raw_event,
                    _ => return,
                };

                if raw_pointer_down.button() != events::MouseButton::Left {
                    return;
                }

                start_drag(
                    &dragging_system,
                    DividerType::FilesPanelSecondary,
                    (event.x() as f32, event.y() as f32),
                );
            }
        })
}
