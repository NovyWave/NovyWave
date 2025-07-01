use zoon::{*, futures_util::future::try_join_all};
use std::f32::consts::PI;
use std::mem;
use moonzoon_novyui::*;

// Type alias for clarity
// Represents a collection of 2D objects for fast2d canvas
type ExampleObjects = Vec<fast2d::Object2d>;

/// Entry point: loads fonts and starts the app.
pub fn main() {
    Task::start(async {
        load_and_register_fonts().await;
        start_app("app", root);
    });
}

/// Loads and registers required fonts asynchronously.
async fn load_and_register_fonts() {
    let fonts = try_join_all([
        fast2d::fetch_file("/_api/public/fonts/FiraCode-Regular.ttf"),
        fast2d::fetch_file("/_api/public/fonts/Inter-Regular.ttf"),
        fast2d::fetch_file("/_api/public/fonts/Inter-Bold.ttf"),
        fast2d::fetch_file("/_api/public/fonts/Inter-BoldItalic.ttf"),
    ]).await.unwrap_throw();
    fast2d::register_fonts(fonts).unwrap_throw();
}

/// Returns an array of example object collections.
fn examples() -> [ExampleObjects; 3] {
    [
        example_rectangle(),
        example_face(),
        example_sine_wave(),
    ]
}

// --- Example objects ---

fn example_rectangle() -> ExampleObjects {
    vec![
        // Main rectangle shape
        fast2d::Rectangle::new()
            .position(50., 50.)
            .size(200., 150.)
            .color(50, 0, 100, 1.0)
            .into(),
        // Label for the rectangle
        fast2d::Text::new()
            .text("Simple Rectangle")
            .position(10., 50.)
            .size(350., 120.)
            .color(255, 255, 255, 0.2)
            .font_size(60.)
            .family(fast2d::Family::name("Fira Code"))
            .into(),
    ]
}

fn example_face() -> ExampleObjects {
    vec![
        // Face background (head)
        fast2d::Circle::new()
            .center(175., 205.)
            .radius(100.)
            .color(0, 128, 0, 1.0)
            .into(),
        // Left eye white
        fast2d::Circle::new()
            .center(135., 175.)
            .radius(15.)
            .color(255, 255, 255, 1.0)
            .inner_border(2., 0, 0, 0, 1.0)
            .into(),
        // Left eye pupil
        fast2d::Circle::new()
            .center(135., 175.)
            .radius(7.)
            .color(0, 0, 0, 1.0)
            .into(),
        // Right eye white
        fast2d::Circle::new()
            .center(215., 175.)
            .radius(15.)
            .color(255, 255, 255, 1.0)
            .inner_border(2., 0, 0, 0, 1.0)
            .into(),
        // Right eye pupil
        fast2d::Circle::new()
            .center(215., 175.)
            .radius(7.)
            .color(0, 0, 0, 1.0)
            .into(),
        // Mouth
        fast2d::Rectangle::new()
            .position(115., 100.)
            .size(120., 20.)
            .color(0, 0, 0, 0.0)
            .rounded_corners(3., 3., 3., 3.)
            .inner_border(3., 139, 0, 0, 1.0)
            .into(),
        // Hat brim
        fast2d::Rectangle::new()
            .position(115., 100.)
            .size(120., 20.)
            .color(0, 0, 0, 0.0)
            .rounded_corners(3., 3., 3., 3.)
            .inner_border(3., 139, 0, 0, 1.0)
            .into(),
        // Hat crown
            fast2d::Rectangle::new()
            .position(135., 60.)
            .size(80., 45.)
            .color(0, 0, 0, 0.0)
            .rounded_corners(15., 15., 0., 0.)
            .inner_border(5., 255, 165, 0, 1.0)
            .into(),
        // Smile
        fast2d::Line::new()
            .points(&[
                (140., 245.),
                (155., 260.),
                (175., 265.),
                (195., 260.),
                (210., 245.),
            ])
            .color(0, 0, 0, 1.0)
            .width(5.)
            .into(),
        // Label for the face
        fast2d::Text::new()
            .text("Face Example")
            .position(10., 10.)
            .size(150., 50.)
            .color(255, 255, 255, 1.0)
            .font_size(20.)
            .family(fast2d::Family::name("Inter"))
            .into(),
        // Label for the hat
        fast2d::Text::new()
            .text("With a ")
            .position(180., 10.)
            .size(70., 50.)
            .color(255, 255, 0, 1.0)
            .font_size(20.)
            .family(fast2d::Family::name("Inter"))
            .italic(false)
            .weight(fast2d::FontWeight::Bold)
            .into(),
        // Label for the hat (continued)
        fast2d::Text::new()
            .text("hat")
            .position(250., 10.)
            .size(50., 50.)
            .color(139, 0, 0, 1.0)
            .font_size(20.)
            .family(fast2d::Family::name("Inter"))
            .italic(true)
            .weight(fast2d::FontWeight::Bold)
            .into(),
    ]
}

fn example_sine_wave() -> ExampleObjects {
    let mut points = Vec::new();
    let amplitude = 50.;
    let frequency = 0.01;
    let y_offset = 150.;
    let steps = 100;
    for i in 0..=steps {
        let x = (i as f32 / steps as f32) * 350.;
        let y = y_offset + amplitude * (x * frequency * 2. * PI).sin();
        points.push((x, y));
    }
    vec![
        // Sine wave line
        fast2d::Line::new()
            .points(&points)
            .color(0, 255, 255, 1.0)
            .width(3.)
            .into(),
        // Label for the sine wave
        fast2d::Text::new()
            .text("Sine Wave Example")
            .position(10., 10.)
            .size(300., 50.)
            .color(255, 255, 255, 0.8)
            .font_size(20.)
            .family(fast2d::Family::name("Fira Code"))
            .into(),
    ]
}

// --- UI Layout ---

fn novyui_buttons_demo() -> impl Element {
    El::new()
        .s(Background::new().color(hsluv!(0, 0, 10)))
        .s(RoundedCorners::all(15))
        .s(Padding::all(20))
        .s(Width::fill().max(650))
        .s(Align::center())
        .child(
            Column::new()
                .s(Gap::new().y(20))
                .item(
                    El::new()
                        .s(Font::new().color(color!("White")).size(24).weight(FontWeight::Bold))
                        .child("NovyUI Buttons Demo")
                )
                .item(
                    Row::new()
                        .s(Gap::new().x(10))
                        .item(
                            button()
                                .label("Primary")
                                .variant(ButtonVariant::Primary)
                                .size(ButtonSize::Medium)
                                .on_press(|| zoon::println!("Primary button clicked!"))
                                .build()
                        )
                        .item(
                            button()
                                .label("Secondary")
                                .variant(ButtonVariant::Secondary)
                                .size(ButtonSize::Medium)
                                .on_press(|| zoon::println!("Secondary button clicked!"))
                                .build()
                        )
                        .item(
                            button()
                                .label("Outline")
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Medium)
                                .on_press(|| zoon::println!("Outline button clicked!"))
                                .build()
                        )
                        .item(
                            button()
                                .label("Link")
                                .variant(ButtonVariant::Link)
                                .size(ButtonSize::Medium)
                                .on_press(|| zoon::println!("Link button clicked!"))
                                .build()
                        )
                        .item(
                            button()
                                .label("Destructive")
                                .variant(ButtonVariant::Destructive)
                                .size(ButtonSize::Medium)
                                .on_press(|| zoon::println!("Destructive button clicked!"))
                                .build()
                        )
                )
                .item(
                    Row::new()
                        .s(Gap::new().x(10))
                        .item(
                            button()
                                .label("Small")
                                .size(ButtonSize::Small)
                                .on_press(|| zoon::println!("Small button clicked!"))
                                .build()
                        )
                        .item(
                            button()
                                .label("Medium")
                                .size(ButtonSize::Medium)
                                .on_press(|| zoon::println!("Medium button clicked!"))
                                .build()
                        )
                        .item(
                            button()
                                .label("Large")
                                .size(ButtonSize::Large)
                                .on_press(|| zoon::println!("Large button clicked!"))
                                .build()
                        )
                )
                .item(
                    Row::new()
                        .s(Gap::new().x(10))
                        .item(
                            button()
                                .label("Icon Check")
                                .left_icon(IconName::Check)
                                .size(ButtonSize::Medium)
                                .on_press(|| zoon::println!("Button with icon clicked!"))
                                .build()
                        )
                        .item(
                            button()
                                .label("Ghost")
                                .variant(ButtonVariant::Ghost)
                                .size(ButtonSize::Medium)
                                .on_press(|| zoon::println!("Ghost button clicked!"))
                                .build()
                        )
                        .item(
                            button()
                                .label("Disabled")
                                .disabled(true)
                                .size(ButtonSize::Medium)
                                .on_press(|| zoon::println!("This won't print"))
                                .build()
                        )
                )
        )
}

fn root() -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Width::fill())
        .s(Background::new().color(hsluv!(220, 15, 8)))
        .child(main_layout())
}

// --- Waveform Viewer Layout ---

fn create_panel(header_content: impl Element, content: impl Element) -> impl Element {
    El::new()
        .s(Background::new().color(hsluv!(220, 15, 11)))
        .s(RoundedCorners::all(6))
        .s(Borders::all(Border::new().width(1).color(hsluv!(220, 10, 25))))
        .child(
            Column::new()
                .item(
                    El::new()
                        .s(Padding::new().x(12).y(8))
                        .s(Background::new().color(hsluv!(220, 15, 13)))
                        .s(Borders::new().bottom(Border::new().width(1).color(hsluv!(220, 10, 25))))
                        .s(RoundedCorners::new().top(6))
                        .s(Font::new().weight(FontWeight::SemiBold).size(14).color(hsluv!(220, 5, 80)))
                        .child(header_content)
                )
                .item(content)
        )
}

fn app_header() -> impl Element {
    Row::new()
        .s(Height::exact(40))
        .s(Width::fill())
        .s(Background::new().color(hsluv!(220, 15, 12)))
        .s(Borders::new().bottom(Border::new().width(1).color(hsluv!(220, 15, 20))))
        .s(Padding::new().x(16).y(8))
        .item(
            Row::new()
                .s(Gap::new().x(8))
                .s(Align::center())
                .item(
                    button()
                        .label("📁 Load files")
                        .variant(ButtonVariant::Secondary)
                        .size(ButtonSize::Small)
                        .on_press(|| zoon::println!("Load files clicked"))
                        .build()
                )
        )
        .item(
            El::new()
                .s(Width::fill())
        )
}

fn main_layout() -> impl Element {
    Row::new()
        .s(Height::fill())
        .s(Width::fill())
        .s(Gap::new().x(1))
        .item(
            Column::new()
                .s(Width::exact(470))
                .s(Height::fill())
                .s(Gap::new().y(1))
                .item(files_panel())
                .item(variables_panel())
        )
        .item(selected_variables_with_waveform_panel())
}

fn files_panel() -> impl Element {
    El::new()
        .s(Height::fill().min(250).max(350))
        .child(
            create_panel(
                Row::new()
                    .s(Gap::new().x(8))
                    .s(Align::new().center_y())
                    .item(
                        El::new()
                            .s(Font::new().no_wrap())
                            .child("Files & Scopes")
                    )
                    .item(
                        El::new()
                            .s(Width::fill())
                    )
                    .item(
                        button()
                            .label("Load Files")
                            .left_icon(IconName::Folder)
                            .variant(ButtonVariant::Secondary)
                            .size(ButtonSize::Small)
                            .on_press(|| zoon::println!("Load Files clicked"))
                            .align(Align::center())
                            .build()
                    )
                    .item(
                        El::new()
                            .s(Width::fill())
                    )
                    .item(
                        button()
                            .label("Remove All")
                            .left_icon(IconName::X)
                            .variant(ButtonVariant::Destructive)
                            .size(ButtonSize::Small)
                            .on_press(|| zoon::println!("Remove All clicked"))
                            .build()
                    ),
                Column::new()
                    .s(Gap::new().y(4))
                    .s(Padding::all(12))
                    .item(
                        // Tree structure matching Figma
                        Column::new()
                            .s(Gap::new().y(2))
                            .item(
                                Row::new()
                                    .s(Gap::new().x(4))
                                    .item("▼")
                                    .item("📄")
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 85)).size(13))
                                            .child("wave_21.fst")
                                    )
                            )
                            .item(
                                Column::new()
                                    .s(Padding::new().left(20))
                                    .s(Gap::new().y(2))
                                    .item(
                                        Row::new()
                                            .s(Gap::new().x(4))
                                            .item("▼")
                                            .item("📁")
                                            .item(
                                                El::new()
                                                    .s(Font::new().color(hsluv!(220, 10, 85)).size(13))
                                                    .child("VexRiscv")
                                            )
                                    )
                                    .item(
                                        Column::new()
                                            .s(Padding::new().left(20))
                                            .s(Gap::new().y(2))
                                            .item(
                                                Row::new()
                                                    .s(Gap::new().x(4))
                                                    .item("📄")
                                                    .item(
                                                        El::new()
                                                            .s(Font::new().color(hsluv!(220, 10, 75)).size(13))
                                                            .child("EntitledRiscvHazardDebugCd_dmDirect_logic")
                                                    )
                                            )
                                            .item(
                                                Row::new()
                                                    .s(Gap::new().x(4))
                                                    .item("📄")
                                                    .item(
                                                        El::new()
                                                            .s(Font::new().color(hsluv!(220, 10, 75)).size(13))
                                                            .child("inputArea_target_buffercc")
                                                    )
                                            )
                                            .item(
                                                Row::new()
                                                    .s(Gap::new().x(4))
                                                    .item("📄")
                                                    .item(
                                                        El::new()
                                                            .s(Font::new().color(hsluv!(220, 10, 75)).size(13))
                                                            .child("bufferCC_4")
                                                    )
                                            )
                                    )
                            )
                            .item(
                                Row::new()
                                    .s(Gap::new().x(4))
                                    .item("+")
                                    .item("📄")
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 85)).size(13))
                                            .child("simple.vcd")
                                    )
                            )
                    )
            )
        )
}

fn variables_panel() -> impl Element {
    El::new()
        .s(Height::fill())
        .child(
            create_panel(
                Row::new()
                    .s(Gap::new().x(8))
                    .s(Align::new().center_y())
                    .item(
                        El::new()
                            .s(Font::new().no_wrap())
                            .child("Variables")
                    )
                    .item(
                        El::new()
                            .s(Width::fill())
                    )
                    .item(
                        input()
                            .placeholder("variable_name")
                            .left_icon(IconName::Search)
                            .size(InputSize::Small)
                            .build()
                    ),
                Column::new()
                    .s(Gap::new().y(6))
                    .s(Padding::all(12))
                    .item(
                        Column::new()
                            .s(Gap::new().y(4))
                            .item(
                                Row::new()
                                    .s(Gap::new().x(8))
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 85)).size(13))
                                            .child("io_bus_cmd_valid")
                                    )
                                    .item(
                                        badge("Wire 1-bit Input")
                                            .variant(BadgeVariant::Primary)
                                            .build()
                                    )
                            )
                            .item(
                                Row::new()
                                    .s(Gap::new().x(8))
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 85)).size(13))
                                            .child("io_bus_cmd_ready")
                                    )
                                    .item(
                                        badge("Wire 1-bit Output")
                                            .variant(BadgeVariant::Success)
                                            .build()
                                    )
                            )
                            .item(
                                Row::new()
                                    .s(Gap::new().x(8))
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 85)).size(13))
                                            .child("io_jtag_data")
                                    )
                                    .item(
                                        badge("Wire 1-bit Output")
                                            .variant(BadgeVariant::Success)
                                            .build()
                                    )
                            )
                            .item(
                                Row::new()
                                    .s(Gap::new().x(8))
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 85)).size(13))
                                            .child("clk")
                                    )
                                    .item(
                                        badge("Wire 1-bit Output")
                                            .variant(BadgeVariant::Success)
                                            .build()
                                    )
                            )
                    )
            )
        )
}

fn selected_variables_with_waveform_panel() -> impl Element {
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .child(
            create_panel(
                Row::new()
                    .s(Gap::new().x(8))
                    .s(Align::new().center_y())
                    .item(
                        El::new()
                            .s(Font::new().no_wrap())
                            .child("Selected Variables")
                    )
                    .item(
                        El::new()
                            .s(Width::fill())
                    )
                    .item(
                        button()
                            .label("Dock to Bottom")
                            .left_icon(IconName::ArrowDownToLine)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Small)
                            .on_press(|| zoon::println!("Dock to Bottom clicked"))
                            .align(Align::center())
                            .build()
                    )
                    .item(
                        El::new()
                            .s(Width::fill())
                    )
                    .item(
                        button()
                            .label("Remove All")
                            .left_icon(IconName::X)
                            .variant(ButtonVariant::Destructive)
                            .size(ButtonSize::Small)
                            .on_press(|| zoon::println!("Remove All clicked"))
                            .build()
                    ),
                Column::new()
                    .s(Gap::new().y(0))
                    .item(
                        // Selected variables list
                        Column::new()
                            .s(Gap::new().y(2))
                            .s(Padding::all(8))
                            .item(
                                Row::new()
                                    .s(Gap::new().x(8))
                                    .s(Align::new().center_y())
                                    .s(Padding::new().y(2))
                                    .item("⋮⋮")
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 85)).size(13))
                                            .child("LsuPlugin_logic_bus_rsp_payload_error")
                                    )
                                    .item("0")
                                    .item("❌")
                                    .item(
                                        El::new()
                                            .s(Width::fill())
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 70)).size(12))
                                            .child("14x2106624")
                                    )
                            )
                            // Add more variable rows...
                            .items((0..8).map(|i| {
                                let var_names = [
                                    "LsuPlugin_logic_bus_rsp_payload_data",
                                    "io_writes_0_payload_data", 
                                    "LsuPlugin_logic_bus_rsp_payload_data",
                                    "logic_logic_onDebugCd_dmiStat_value_string",
                                    "LsuPlugin_logic_bus_rsp_payload_error",
                                    "LsuPlugin_logic_bus_rsp_payload_data",
                                    "io_writes_0_payload_data",
                                    "final_var"
                                ];
                                
                                Row::new()
                                    .s(Gap::new().x(8))
                                    .s(Align::new().center_y())
                                    .s(Padding::new().y(2))
                                    .item("⋮⋮")
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 85)).size(13))
                                            .child(var_names[i as usize])
                                    )
                                    .item("0")
                                    .item("❌")
                                    .item(
                                        El::new()
                                            .s(Width::fill())
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 70)).size(12))
                                            .child("14x2106624")
                                    )
                            }))
                    )
                    .item(
                        // Waveform display area
                        Column::new()
                            .s(Gap::new().y(4))
                            .s(Padding::all(8))
                            .s(Background::new().color(hsluv!(220, 15, 8)))
                            .item(
                                // Timeline
                                Row::new()
                                    .s(Gap::new().x(40))
                                    .s(Padding::new().x(50))
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("0 s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("10 s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("20 s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("30 s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("40 s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("50 s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("60 s")
                                    )
                            )
                            .item(
                                // Waveform visualization with blue rectangles
                                El::new()
                                    .s(Height::exact(200))
                                    .s(Width::fill())
                                    .s(Background::new().color(hsluv!(220, 15, 6)))
                                    .s(RoundedCorners::all(4))
                                    .s(Padding::all(8))
                                    .child(
                                        Column::new()
                                            .s(Gap::new().y(1))
                                            .items((0..9).map(|i| {
                                                Row::new()
                                                    .s(Gap::new().x(1))
                                                    .s(Height::exact(20))
                                                    .s(Width::fill())
                                                    .items((0..12).map(|j| {
                                                        El::new()
                                                            .s(Width::fill())
                                                            .s(Height::exact(18))
                                                            .s(Background::new().color(
                                                                if (i + j) % 3 == 0 {
                                                                    hsluv!(220, 80, 55) // Bright blue
                                                                } else if (i + j) % 2 == 0 {
                                                                    hsluv!(220, 60, 45) // Medium blue  
                                                                } else {
                                                                    hsluv!(220, 15, 8) // Dark background
                                                                }
                                                            ))
                                                            .s(RoundedCorners::all(2))
                                                    }))
                                            }))
                                    )
                            )
                            .item(
                                // Bottom controls
                                Row::new()
                                    .s(Gap::new().x(10))
                                    .s(Align::center())
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 70)).size(12))
                                            .child("W")
                                    )
                                    .item("@")
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 70)).size(12))
                                            .child("100")
                                    )
                                    .item("%")
                                    .item("⬛")
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 70)).size(12))
                                            .child("S")
                                    )
                                    .item(
                                        El::new()
                                            .s(Width::fill())
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 70)).size(12))
                                            .child("A")
                                    )
                                    .item("◀")
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 70)).size(12))
                                            .child("55")
                                    )
                                    .item("/")
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 70)).size(12))
                                            .child("88 s")
                                    )
                                    .item("▶")
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 70)).size(12))
                                            .child("D")
                                    )
                            )
                    )
            )
        )
}

fn selected_panel() -> impl Element {
    El::new()
        .s(Height::fill())
        .child(
            create_panel(
                Row::new()
                    .s(Gap::new().x(10))
                    .item(
                        Text::new("Selected Variables")
                    )
                    .item(
                        button()
                            .label("Dock to Bottom")
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Small)
                            .on_press(|| zoon::println!("Dock to Bottom clicked"))
                            .build()
                    ),
                Column::new()
                    .s(Gap::new().y(8))
                    .s(Padding::all(16))
                    .item(
                        Row::new()
                            .s(Gap::new().x(8))
                            .s(Align::new().center_y())
                            .item("⋮⋮")
                            .item(
                                El::new()
                                    .s(Font::new().color(hsluv!(0, 0, 80)).size(14))
                                    .child("clock")
                            )
                            .item(
                                button()
                                    .label("×")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Small)
                                    .on_press(|| zoon::println!("Remove clock"))
                                    .build()
                            )
                    )
                    .item(
                        Row::new()
                            .s(Gap::new().x(8))
                            .s(Align::new().center_y())
                            .item("⋮⋮")
                            .item(
                                El::new()
                                    .s(Font::new().color(hsluv!(0, 0, 80)).size(14))
                                    .child("reset")
                            )
                            .item(
                                button()
                                    .label("×")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Small)
                                    .on_press(|| zoon::println!("Remove reset"))
                                    .build()
                            )
                    )
            )
        )
}

fn waveform_panel() -> impl Element {
    El::new()
        .s(Width::fill().min(500))
        .s(Height::fill())
        .child(
            create_panel(
                Row::new()
                    .s(Gap::new().x(10))
                    .item(
                        Text::new("Waveform")
                    )
                    .item(
                        button()
                            .label("Zoom In")
                            .left_icon(IconName::ZoomIn)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Small)
                            .on_press(|| zoon::println!("Zoom In clicked"))
                            .build()
                    )
                    .item(
                        button()
                            .label("Zoom Out")
                            .left_icon(IconName::ZoomOut)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Small)
                            .on_press(|| zoon::println!("Zoom Out clicked"))
                            .build()
                    ),
                Column::new()
                    .s(Gap::new().y(16))
                    .s(Padding::all(16))
                    .item(
                        Row::new()
                            .s(Gap::new().x(20))
                            .item("0s")
                            .item("10s")
                            .item("20s")
                            .item("30s")
                            .item("40s")
                            .item("50s")
                    )
                    .item(
                        El::new()
                            .s(Background::new().color(hsluv!(0, 0, 15)))
                            .s(Height::exact(200))
                            .s(Width::fill())
                            .s(Align::center())
                            .s(RoundedCorners::all(4))
                            .child(
                                El::new()
                                    .s(Font::new().color(hsluv!(0, 0, 50)).size(16))
                                    .child("Waveform display area")
                            )
                    )
            )
        )
}

fn panel_with_canvas(example_objects: ExampleObjects) -> impl Element {
    El::new()
        .s(Align::center())
        .s(Clip::both())
        .s(Borders::all(Border::new().color(color!("Gray"))))
        .s(Width::fill().max(650))
        .s(Height::exact(350))
        .child_signal(canvas_with_example(example_objects).into_signal_option())
}

async fn canvas_with_example(mut example_objects: ExampleObjects) -> impl Element {
    let mut zoon_canvas = Canvas::new()
        .width(0)
        .height(0)
        .s(Width::fill())
        .s(Height::fill());

    let dom_canvas = zoon_canvas.raw_el_mut().dom_element();
    let mut canvas_wrapper = fast2d::CanvasWrapper::new_with_canvas(dom_canvas).await;
    canvas_wrapper.update_objects(move |objects| {
        mem::swap(objects, &mut example_objects)
    });

    zoon_canvas.update_raw_el(move |raw_el| {
        raw_el.on_resize(move |width, height| {
            canvas_wrapper.resized(width, height);
        })
    })
}
