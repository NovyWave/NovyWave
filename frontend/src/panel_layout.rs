use moonzoon_novyui::tokens::color::{neutral_2, neutral_4, neutral_11, primary_3, primary_6, primary_7};
use moonzoon_novyui::*;
use zoon::*;

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
pub fn variables_name_vertical_divider(app_config: &crate::config::AppConfig) -> impl Element {
    use crate::dragging::{DividerType, is_divider_dragging, start_drag};

    let is_dragging_signal = is_divider_dragging(DividerType::VariablesNameColumn);

    El::new()
        .s(Width::exact(4))
        .s(Height::fill())
        .s(Background::new()
            .color_signal(is_dragging_signal.map_bool_signal(|| primary_7(), || primary_6())))
        .s(Cursor::new(CursorIcon::ColumnResize))
        .s(Padding::all(0))
        .on_pointer_down({
            let app_config = app_config.clone();
            move || {
                start_drag(DividerType::VariablesNameColumn, (0.0, 0.0), &app_config);
            }
        })
}

/// Vertical divider for variables value column
pub fn variables_value_vertical_divider(app_config: &crate::config::AppConfig) -> impl Element {
    use crate::dragging::{DividerType, is_divider_dragging, start_drag};

    let is_dragging_signal = is_divider_dragging(DividerType::VariablesValueColumn);

    El::new()
        .s(Width::exact(4))
        .s(Height::fill())
        .s(Background::new()
            .color_signal(is_dragging_signal.map_bool_signal(|| primary_7(), || primary_6())))
        .s(Cursor::new(CursorIcon::ColumnResize))
        .s(Padding::all(0))
        .on_pointer_down({
            let app_config = app_config.clone();
            move || {
                start_drag(DividerType::VariablesValueColumn, (0.0, 0.0), &app_config);
            }
        })
}

/// Vertical divider for files panel main section
pub fn files_panel_vertical_divider(app_config: &crate::config::AppConfig) -> impl Element {
    use crate::dragging::{DividerType, is_divider_dragging, start_drag};

    let is_dragging_signal = is_divider_dragging(DividerType::FilesPanelMain);

    El::new()
        .s(Width::exact(4))
        .s(Height::fill())
        .s(Background::new()
            .color_signal(is_dragging_signal.map_bool_signal(|| primary_7(), || primary_6())))
        .s(Cursor::new(CursorIcon::ColumnResize))
        .s(Padding::all(0))
        .on_pointer_down({
            let app_config = app_config.clone();
            move || {
                start_drag(DividerType::FilesPanelMain, (0.0, 0.0), &app_config);
            }
        })
}

/// Horizontal divider for files panel secondary section
pub fn files_panel_horizontal_divider(app_config: &crate::config::AppConfig) -> impl Element {
    use crate::dragging::{DividerType, is_divider_dragging, start_drag};

    let is_dragging_signal = is_divider_dragging(DividerType::FilesPanelSecondary);

    El::new()
        .s(Width::fill())
        .s(Height::exact(4))
        .s(Background::new()
            .color_signal(is_dragging_signal.map_bool_signal(|| primary_7(), || primary_6())))
        .s(Cursor::new(CursorIcon::RowResize))
        .on_pointer_down({
            let app_config = app_config.clone();
            move || {
                start_drag(DividerType::FilesPanelSecondary, (0.0, 0.0), &app_config);
            }
        })
}