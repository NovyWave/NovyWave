use zoon::{*, futures_util::future::try_join_all};
use std::f32::consts::PI;
use std::mem;
use moonzoon_novyui::*;
use serde::{Serialize, Deserialize};

// Panel resizing state
static LEFT_PANEL_WIDTH: Lazy<Mutable<u32>> = Lazy::new(|| 470.into());
static FILES_PANEL_HEIGHT: Lazy<Mutable<u32>> = Lazy::new(|| 300.into());
static VERTICAL_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();
static HORIZONTAL_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();

// Dock state management - DEFAULT TO DOCKED MODE  
static IS_DOCKED_TO_BOTTOM: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(true));
static MAIN_AREA_HEIGHT: Lazy<Mutable<u32>> = Lazy::new(|| 350.into());

// File dialog state
static SHOW_FILE_DIALOG: Lazy<Mutable<bool>> = lazy::default();
static FILE_PATHS_INPUT: Lazy<Mutable<String>> = lazy::default();

// File loading progress state
static LOADING_FILES: Lazy<MutableVec<LoadingFile>> = lazy::default();
static IS_LOADING: Lazy<Mutable<bool>> = lazy::default();

// Loaded files hierarchy for TreeView
static LOADED_FILES: Lazy<MutableVec<WaveformFile>> = lazy::default();

#[derive(Clone, Debug)]
pub struct LoadingFile {
    pub file_id: String,
    pub filename: String,
    pub progress: f32,
    pub status: LoadingStatus,
}

#[derive(Clone, Debug)]
pub enum LoadingStatus {
    Starting,
    Parsing,
    Completed,
    Error(String),
}

// Backend message types
#[derive(Serialize, Deserialize, Debug)]
pub enum UpMsg {
    LoadWaveformFile(String),
    GetParsingProgress(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DownMsg {
    ParsingStarted { file_id: String, filename: String },
    ParsingProgress { file_id: String, progress: f32 },
    FileLoaded { file_id: String, hierarchy: FileHierarchy },
    ParsingError { file_id: String, error: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileHierarchy {
    pub files: Vec<WaveformFile>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WaveformFile {
    pub id: String,
    pub filename: String,
    pub format: FileFormat,
    pub signals: Vec<Signal>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FileFormat {
    VCD,
    FST,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Signal {
    pub id: String,
    pub name: String,
    pub signal_type: String,
    pub width: u32,
}

// Type alias for clarity
// Represents a collection of 2D objects for fast2d canvas
type ExampleObjects = Vec<fast2d::Object2d>;

fn show_file_paths_dialog() {
    SHOW_FILE_DIALOG.set(true);
    FILE_PATHS_INPUT.set_neq(String::new());
}

fn process_file_paths() {
    let input = FILE_PATHS_INPUT.get_cloned();
    let paths: Vec<String> = input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    zoon::println!("Selected file paths: {:?}", paths);
    
    if !paths.is_empty() {
        IS_LOADING.set(true);
    }
    
    for path in paths {
        zoon::println!("Loading file: {}", path);
        send_up_msg(UpMsg::LoadWaveformFile(path));
    }
    
    SHOW_FILE_DIALOG.set(false);
}

static CONNECTION: Lazy<Connection<UpMsg, DownMsg>> = Lazy::new(|| {
    Connection::new(|down_msg, _| {
        zoon::println!("Received DownMsg: {:?}", down_msg);
        match down_msg {
            DownMsg::ParsingStarted { file_id, filename } => {
                zoon::println!("Started parsing file: {} ({})", filename, file_id);
                
                // Add or update loading file
                let loading_file = LoadingFile {
                    file_id: file_id.clone(),
                    filename: filename.clone(),
                    progress: 0.0,
                    status: LoadingStatus::Starting,
                };
                
                LOADING_FILES.lock_mut().push_cloned(loading_file);
            }
            DownMsg::ParsingProgress { file_id, progress } => {
                zoon::println!("File {} progress: {}%", file_id, progress * 100.0);
                
                // Update progress for the file
                let current_files: Vec<LoadingFile> = LOADING_FILES.lock_ref().iter().cloned().collect();
                let updated_files: Vec<LoadingFile> = current_files.into_iter().map(|mut file| {
                    if file.file_id == file_id {
                        file.progress = progress;
                        file.status = LoadingStatus::Parsing;
                    }
                    file
                }).collect();
                LOADING_FILES.lock_mut().replace_cloned(updated_files);
            }
            DownMsg::FileLoaded { file_id, hierarchy } => {
                zoon::println!("File loaded: {} with {} files", file_id, hierarchy.files.len());
                
                // Add loaded files to the TreeView state
                for file in hierarchy.files {
                    zoon::println!("  - {}: {} signals", file.filename, file.signals.len());
                    LOADED_FILES.lock_mut().push_cloned(file);
                }
                
                // Mark file as completed
                let current_files: Vec<LoadingFile> = LOADING_FILES.lock_ref().iter().cloned().collect();
                let updated_files: Vec<LoadingFile> = current_files.into_iter().map(|mut file| {
                    if file.file_id == file_id {
                        file.progress = 1.0;
                        file.status = LoadingStatus::Completed;
                    }
                    file
                }).collect();
                LOADING_FILES.lock_mut().replace_cloned(updated_files);
                
                // Check if all files are completed
                check_loading_complete();
            }
            DownMsg::ParsingError { file_id, error } => {
                zoon::println!("Error parsing file {}: {}", file_id, error);
                
                // Mark file as error
                let current_files: Vec<LoadingFile> = LOADING_FILES.lock_ref().iter().cloned().collect();
                let updated_files: Vec<LoadingFile> = current_files.into_iter().map(|mut file| {
                    if file.file_id == file_id {
                        file.status = LoadingStatus::Error(error.clone());
                    }
                    file
                }).collect();
                LOADING_FILES.lock_mut().replace_cloned(updated_files);
                
                // Check if all files are completed
                check_loading_complete();
            }
        }
    })
});

fn check_loading_complete() {
    let loading_files = LOADING_FILES.lock_ref();
    let all_done = loading_files.iter().all(|f| {
        matches!(f.status, LoadingStatus::Completed | LoadingStatus::Error(_))
    });
    
    if all_done {
        IS_LOADING.set(false);
        // Clear completed files after a delay to show final state
        Task::start(async {
            Timer::sleep(2000).await;
            LOADING_FILES.lock_mut().clear();
        });
    }
}

fn load_files_button_with_progress(variant: ButtonVariant, size: ButtonSize, icon: Option<IconName>) -> impl Element {
    El::new()
        .child_signal(IS_LOADING.signal().map(move |is_loading| {
            let mut btn = button();
            
            if is_loading {
                btn = btn.label("Loading...")
                    .disabled(true);
                if let Some(icon) = icon {
                    btn = btn.left_icon(icon);
                }
            } else {
                btn = btn.label("Load Files")
                    .on_press(|| show_file_paths_dialog());
                if let Some(icon) = icon {
                    btn = btn.left_icon(icon);
                }
            }
            
            btn.variant(variant.clone())
                .size(size.clone())
                .build()
                .into_element()
        }))
}

fn load_files_dialog_button() -> impl Element {
    El::new()
        .child_signal(IS_LOADING.signal().map(|is_loading| {
            let mut btn = button();
            
            if is_loading {
                btn = btn.label("Loading...")
                    .disabled(true);
            } else {
                btn = btn.label("Load Files")
                    .on_press(|| process_file_paths());
            }
            
            btn.variant(ButtonVariant::Primary)
                .size(ButtonSize::Medium)
                .build()
                .into_element()
        }))
}

fn send_up_msg(up_msg: UpMsg) {
    Task::start(async move {
        let result = CONNECTION.send_up_msg(up_msg).await;
        if let Err(error) = result {
            zoon::println!("Failed to send message: {:?}", error);
        }
    });
}

/// Entry point: loads fonts and starts the app.
pub fn main() {
    Task::start(async {
        load_and_register_fonts().await;
        // Force the default "Docked to Right" state
        IS_DOCKED_TO_BOTTOM.set(false);
        
        start_app("app", root);
        CONNECTION.init_lazy();
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
                                .on_press(|| {})
                                .build()
                        )
                        .item(
                            button()
                                .label("Secondary")
                                .variant(ButtonVariant::Secondary)
                                .size(ButtonSize::Medium)
                                .on_press(|| {})
                                .build()
                        )
                        .item(
                            button()
                                .label("Outline")
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Medium)
                                .on_press(|| {})
                                .build()
                        )
                        .item(
                            button()
                                .label("Link")
                                .variant(ButtonVariant::Link)
                                .size(ButtonSize::Medium)
                                .on_press(|| {})
                                .build()
                        )
                        .item(
                            button()
                                .label("Destructive")
                                .variant(ButtonVariant::Destructive)
                                .size(ButtonSize::Medium)
                                .on_press(|| {})
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
                                .on_press(|| {})
                                .build()
                        )
                        .item(
                            button()
                                .label("Medium")
                                .size(ButtonSize::Medium)
                                .on_press(|| {})
                                .build()
                        )
                        .item(
                            button()
                                .label("Large")
                                .size(ButtonSize::Large)
                                .on_press(|| {})
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
                                .on_press(|| {})
                                .build()
                        )
                        .item(
                            button()
                                .label("Ghost")
                                .variant(ButtonVariant::Ghost)
                                .size(ButtonSize::Medium)
                                .on_press(|| {})
                                .build()
                        )
                        .item(
                            button()
                                .label("Disabled")
                                .disabled(true)
                                .size(ButtonSize::Medium)
                                .on_press(|| {})
                                .build()
                        )
                )
        )
}

fn file_paths_dialog() -> impl Element {
    El::new()
        .s(Background::new().color("rgba(0, 0, 0, 0.8)"))
        .s(Width::fill())
        .s(Height::fill())
        .s(Align::center())
        .child(
            El::new()
                .s(Background::new().color(hsluv!(220, 15, 15)))
                .s(RoundedCorners::all(8))
                .s(Borders::all(Border::new().width(2).color(hsluv!(220, 10, 30))))
                .s(Padding::all(24))
                .s(Width::exact(500))
                .child(
                    Column::new()
                        .s(Gap::new().y(16))
                        .item(
                            El::new()
                                .s(Font::new().size(18).weight(FontWeight::Bold).color(hsluv!(220, 10, 85)))
                                .child("Load Waveform Files")
                        )
                        .item(
                            El::new()
                                .s(Font::new().size(14).color(hsluv!(220, 10, 70)))
                                .child("Enter absolute file paths, separated by commas:")
                        )
                        .item(
                            input()
                                .placeholder("/path/to/file1.vcd, /path/to/file2.fst")
                                .on_change(|text| FILE_PATHS_INPUT.set_neq(text))
                                .size(InputSize::Medium)
                                .build()
                        )
                        .item(
                            Row::new()
                                .s(Gap::new().x(12))
                                .s(Align::new().right())
                                .item(
                                    button()
                                        .label("Cancel")
                                        .variant(ButtonVariant::Ghost)
                                        .size(ButtonSize::Medium)
                                        .on_press(|| SHOW_FILE_DIALOG.set(false))
                                        .build()
                                )
                                .item(
                                    load_files_dialog_button()
                                )
                        )
                )
        )
}

fn root() -> impl Element {
    Stack::new()
        .s(Height::screen())
        .s(Width::fill())
        .s(Background::new().color(hsluv!(220, 15, 8)))
        .layer(main_layout())
        .layer_signal(SHOW_FILE_DIALOG.signal().map_true(
            || file_paths_dialog()
        ))
}

// --- Waveform Viewer Layout ---

fn create_panel(header_content: impl Element, content: impl Element) -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Background::new().color(hsluv!(220, 15, 11)))
        .s(RoundedCorners::all(6))
        .s(Borders::all(Border::new().width(1).color(hsluv!(220, 10, 25))))
        .child(
            Column::new()
                .s(Height::fill())
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
                        .on_press(|| show_file_paths_dialog())
                        .build()
                )
        )
        .item(
            El::new()
                .s(Width::fill())
        )
}

fn main_layout() -> impl Element {
    let is_any_divider_dragging = map_ref! {
        let vertical = VERTICAL_DIVIDER_DRAGGING.signal(),
        let horizontal = HORIZONTAL_DIVIDER_DRAGGING.signal() =>
        *vertical || *horizontal
    };

    El::new()
        .s(Height::screen())
        .s(Width::fill())
        .text_content_selecting_signal(
            is_any_divider_dragging.map(|is_dragging| {
                if is_dragging {
                    TextContentSelecting::none()
                } else {
                    TextContentSelecting::auto()
                }
            })
        )
        .s(Cursor::with_signal(
            map_ref! {
                let vertical = VERTICAL_DIVIDER_DRAGGING.signal(),
                let horizontal = HORIZONTAL_DIVIDER_DRAGGING.signal() =>
                if *vertical {
                    Some(CursorIcon::ColumnResize)
                } else if *horizontal {
                    Some(CursorIcon::RowResize)
                } else {
                    None
                }
            }
        ))
        .on_pointer_up(|| {
            VERTICAL_DIVIDER_DRAGGING.set_neq(false);
            HORIZONTAL_DIVIDER_DRAGGING.set_neq(false);
        })
        .on_pointer_leave(|| {
            VERTICAL_DIVIDER_DRAGGING.set_neq(false);
            HORIZONTAL_DIVIDER_DRAGGING.set_neq(false);
        })
        .on_pointer_move_event(|event| {
            if VERTICAL_DIVIDER_DRAGGING.get() {
                LEFT_PANEL_WIDTH.update(|width| {
                    let new_width = width as i32 + event.movement_x();
                    u32::max(50, u32::try_from(new_width).unwrap_or(50))
                });
            } else if HORIZONTAL_DIVIDER_DRAGGING.get() {
                if IS_DOCKED_TO_BOTTOM.get() {
                    // In "Docked to Bottom" mode, horizontal divider controls main area height
                    MAIN_AREA_HEIGHT.update(|height| {
                        let new_height = height as i32 + event.movement_y();
                        u32::max(50, u32::try_from(new_height).unwrap_or(50))
                    });
                } else {
                    // In "Docked to Right" mode, horizontal divider controls files panel height
                    FILES_PANEL_HEIGHT.update(|height| {
                        let new_height = height as i32 + event.movement_y();
                        u32::max(50, u32::try_from(new_height).unwrap_or(50))
                    });
                }
            }
        })
        .child(docked_layout_wrapper())
}

// Wrapper function that switches between docked and undocked layouts
fn docked_layout_wrapper() -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Width::fill())
        .child_signal(IS_DOCKED_TO_BOTTOM.signal().map(|is_docked| {
            if is_docked {
                // Docked to Bottom layout
                El::new()
                    .s(Height::fill())
                    .child(
                        Column::new()
                            .s(Height::fill())
                            .s(Width::fill())
                            .item(
                                Row::new()
                                    .s(Height::exact_signal(MAIN_AREA_HEIGHT.signal()))
                                    .s(Width::fill())
                                    .item(files_panel_docked())
                                    .item(vertical_divider(VERTICAL_DIVIDER_DRAGGING.clone()))
                                    .item(variables_panel_docked())
                            )
                            .item(horizontal_divider(HORIZONTAL_DIVIDER_DRAGGING.clone()))
                            .item(selected_variables_with_waveform_panel())
                    )
            } else {
                // Docked to Right layout
                El::new()
                    .s(Height::fill())
                    .child(
                        Row::new()
                            .s(Height::fill())
                            .s(Width::fill())
                            .item(
                                El::new()
                                    .s(Width::exact_signal(LEFT_PANEL_WIDTH.signal()))
                                    .s(Height::fill())
                                    .child(
                                        Column::new()
                                            .s(Height::fill())
                                            .item(files_panel_with_height())
                                            .item(horizontal_divider(HORIZONTAL_DIVIDER_DRAGGING.clone()))
                                            .item(variables_panel_with_fill())
                                    )
                            )
                            .item(vertical_divider(VERTICAL_DIVIDER_DRAGGING.clone()))
                            .item(
                                El::new()
                                    .s(Width::fill())
                                    .s(Height::fill())
                                    .child(selected_variables_with_waveform_panel())
                            )
                    )
            }
        }))
}

// Docked layout: Top area (Files & Scopes | Variables) + Bottom area (Selected Variables)
fn docked_layout() -> impl Element {
    Column::new()
        .s(Height::fill())
        .s(Width::fill())
        .item(
            Row::new()
                .s(Height::exact_signal(MAIN_AREA_HEIGHT.signal()))
                .s(Width::fill())
                .item(files_panel_docked())
                .item(vertical_divider(VERTICAL_DIVIDER_DRAGGING.clone()))
                .item(variables_panel_docked())
        )
        .item(horizontal_divider(HORIZONTAL_DIVIDER_DRAGGING.clone()))
        .item(selected_variables_with_waveform_panel())
}

// Undocked layout: (Files & Scopes + Variables) | Selected Variables
fn undocked_layout() -> impl Element {
    Row::new()
        .s(Height::fill())
        .s(Width::fill())
        .item(
            Column::new()
                .s(Width::exact_signal(LEFT_PANEL_WIDTH.signal()))
                .s(Height::fill())
                .item(files_panel_with_height())
                .item(horizontal_divider(HORIZONTAL_DIVIDER_DRAGGING.clone()))
                .item(variables_panel_with_fill())
        )
        .item(vertical_divider(VERTICAL_DIVIDER_DRAGGING.clone()))
        .item(selected_variables_with_waveform_panel())
}

// Helper functions for different panel configurations

fn files_panel_with_width() -> impl Element {
    El::new()
        .s(Width::exact_signal(LEFT_PANEL_WIDTH.signal()))
        .s(Height::fill())
        .child(files_panel())
}

fn files_panel_with_height() -> impl Element {
    El::new()
        .s(Height::exact_signal(FILES_PANEL_HEIGHT.signal()))
        .s(Width::fill())
        .child(files_panel())
}

fn variables_panel_with_fill() -> impl Element {
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .child(variables_panel())
}

// Docked mode specific panels with proper sizing
fn files_panel_docked() -> impl Element {
    El::new()
        .s(Width::exact_signal(LEFT_PANEL_WIDTH.signal()))  // Use draggable width in docked mode too
        .s(Height::fill())
        .child(files_panel())
}

fn variables_panel_docked() -> impl Element {
    El::new()
        .s(Width::fill())  // Variables takes remaining space
        .s(Height::fill())
        .child(variables_panel())
}

fn remove_all_button() -> impl Element {
    button()
        .label("Remove All")
        .left_icon(IconName::X)
        .variant(ButtonVariant::DestructiveGhost)
        .size(ButtonSize::Small)
        .on_press(|| {
            LOADED_FILES.lock_mut().clear();
            zoon::println!("Cleared all loaded files");
        })
        .build()
}

fn dock_toggle_button() -> impl Element {
    El::new()
        .child_signal(IS_DOCKED_TO_BOTTOM.signal().map(|is_docked| {
            button()
                .label(if is_docked { "Dock to Right" } else { "Dock to Bottom" })
                .left_icon_element(|| {
                    El::new()
                        .child_signal(IS_DOCKED_TO_BOTTOM.signal().map(|is_docked| {
                            let icon_el = icon(IconName::ArrowDownToLine).size(IconSize::Small).build();
                            if is_docked {
                                El::new()
                                    .s(Transform::new().rotate(-90))
                                    .child(icon_el)
                                    .into_element()
                            } else {
                                El::new().child(icon_el).into_element()
                            }
                        }))
                        .unify()
                })
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Small)
                .on_press(|| {
                    IS_DOCKED_TO_BOTTOM.update(|is_docked| !is_docked);
                })
                .align(Align::center())
                .build()
                .into_element()
        }))
}

fn convert_files_to_tree_data(files: &[WaveformFile]) -> Vec<TreeViewItemData> {
    files.iter().map(|file| {
        let format_label = match file.format {
            FileFormat::VCD => "VCD",
            FileFormat::FST => "FST",
        };
        
        // Group signals by their path components (if they have hierarchy)
        let mut signal_groups: std::collections::HashMap<String, Vec<&Signal>> = std::collections::HashMap::new();
        
        for signal in &file.signals {
            // For now, just put all signals at the root level
            // Later we can parse signal names for hierarchy (e.g., "cpu.core.alu" -> nested structure)
            signal_groups.entry("signals".to_string()).or_default().push(signal);
        }
        
        let mut children = vec![];
        
        // Add a summary item
        children.push(
            TreeViewItemData::new(
                format!("{}_info", file.id),
                format!("{} - {} signals", format_label, file.signals.len())
            )
            .item_type(TreeViewItemType::File)
        );
        
        // Add signal groups
        for (group_name, signals) in signal_groups {
            if signals.len() > 10 {
                // If too many signals, create a folder
                children.push(
                    TreeViewItemData::new(
                        format!("{}_{}", file.id, group_name),
                        format!("Signals ({} items)", signals.len())
                    )
                    .item_type(TreeViewItemType::Folder)
                    .with_children(
                        signals.iter().take(20).map(|signal| {
                            TreeViewItemData::new(
                                format!("signal_{}", signal.id),
                                format!("{} [{}:0]", signal.name, signal.width - 1)
                            )
                            .item_type(TreeViewItemType::File)
                        }).collect()
                    )
                );
            } else {
                // Add signals directly
                for signal in signals {
                    children.push(
                        TreeViewItemData::new(
                            format!("signal_{}", signal.id),
                            format!("{} [{}:0]", signal.name, signal.width - 1)
                        )
                        .item_type(TreeViewItemType::File)
                    );
                }
            }
        }
        
        TreeViewItemData::new(file.id.clone(), file.filename.clone())
            .item_type(TreeViewItemType::File)
            .with_children(children)
    }).collect()
}

fn files_panel() -> impl Element {
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
                            .child("Files & Scopes")
                    )
                    .item(
                        El::new()
                            .s(Width::fill())
                    )
                    .item(
                        load_files_button_with_progress(
                            ButtonVariant::Secondary,
                            ButtonSize::Small,
                            Some(IconName::Folder)
                        )
                    )
                    .item(
                        El::new()
                            .s(Width::fill())
                    )
                    .item(
                        remove_all_button()
                    ),
                Column::new()
                    .s(Gap::new().y(4))
                    .s(Padding::all(12))
                    .s(Height::fill())  // Make the column fill available height
                    .item(
                        El::new()
                            .s(Height::fill())
                            .child_signal(
                                LOADED_FILES.signal_vec_cloned()
                                    .to_signal_map(|files| {
                                        let tree_data = convert_files_to_tree_data(&files);
                                        
                                        if tree_data.is_empty() {
                                            // Show placeholder when no files loaded
                                            El::new()
                                                .s(Padding::all(20))
                                                .s(Font::new().color(hsluv!(0, 0, 50)).italic())
                                                .child("No files loaded. Click 'Load Files' to add waveform files.")
                                                .unify()
                                        } else {
                                            // Show TreeView with loaded files
                                            let expanded_ids: Vec<String> = files.iter()
                                                .map(|f| f.id.clone())
                                                .collect();
                                            
                                            tree_view()
                                                .data(tree_data)
                                                .size(TreeViewSize::Medium)
                                                .variant(TreeViewVariant::Basic)
                                                .show_icons(true)
                                                .show_checkboxes(false)
                                                .default_expanded(expanded_ids)
                                                .build()
                                                .unify()
                                        }
                                    })
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
                    .s(Height::fill())  // Make the column fill available height
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

fn vertical_divider(is_dragging: Mutable<bool>) -> impl Element {
    El::new()
        .s(Width::exact(4))  // Back to original 4px width
        .s(Height::fill())
        .s(Background::new().color_signal(
            is_dragging.signal().map_bool(
                || hsluv!(220, 100, 75), // Brighter blue when dragging
                || hsluv!(220, 85, 60)   // Default blue matching Figma exactly
            )
        ))
        .s(Cursor::new(CursorIcon::ColumnResize))
        .s(Padding::all(0))  // Ensure no padding interferes
        .on_pointer_down(move || is_dragging.set_neq(true))
}

fn horizontal_divider(is_dragging: Mutable<bool>) -> impl Element {
    El::new()
        .s(Width::fill())
        .s(Height::exact(4))
        .s(Background::new().color_signal(
            is_dragging.signal().map_bool(
                || hsluv!(220, 100, 75), // Brighter blue when dragging
                || hsluv!(220, 85, 60)   // Default blue matching Figma exactly
            )
        ))
        .s(Cursor::new(CursorIcon::RowResize))
        .on_pointer_down(move || is_dragging.set_neq(true))
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
                        dock_toggle_button()
                    )
                    .item(
                        El::new()
                            .s(Width::fill())
                    )
                    .item(
                        remove_all_button()
                    ),
                // 3-column table layout: Variable Name | Value | Waveform
                El::new()
                    .s(Height::fill())
                    .child(
                        Column::new()
                            .s(Gap::new().y(0))
                            .s(Padding::all(8))
                            .s(Height::fill())  // Make the column fill available height
                            .item(
                                // Timeline header
                        Row::new()
                            .s(Gap::new().x(0))
                            .s(Align::new().center_y())
                            .s(Padding::new().y(4))
                            .item(
                                // Variable Name column header
                                El::new()
                                    .s(Width::exact(250))
                                    .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                    .child("Variable")
                            )
                            .item(
                                // Value column header  
                                El::new()
                                    .s(Width::exact(60))
                                    .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                    .child("Value")
                            )
                            .item(
                                // Timeline markers for waveform column
                                Row::new()
                                    .s(Width::fill())
                                    .s(Gap::new().x(40))
                                    .s(Padding::new().x(10))
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("0s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("10s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("20s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("30s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("40s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("50s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 60)).size(12))
                                            .child("60s")
                                    )
                            )
                    )
                    .items((0..8).map(|i| {
                        let var_names = [
                            "LsuPlugin_logic_bus_rsp_payload_error",
                            "LsuPlugin_logic_bus_rsp_payload_data",
                            "io_writes_0_payload_data", 
                            "logic_logic_onDebugCd_dmiStat_value_string",
                            "LsuPlugin_logic_bus_rsp_payload_error",
                            "LsuPlugin_logic_bus_rsp_payload_data",
                            "io_writes_0_payload_data",
                            "clk"
                        ];
                        
                        let values = ["0", "14x2106624", "0", "success", "0", "14x2106624", "0", "1"];
                        
                        // Each row: Variable Name | Value | Waveform
                        Row::new()
                            .s(Gap::new().x(0))
                            .s(Align::new().center_y())
                            .s(Padding::new().y(1))
                            .item(
                                // Variable Name column (250px width)
                                Row::new()
                                    .s(Width::exact(250))
                                    .s(Gap::new().x(8))
                                    .s(Align::new().center_y())
                                    .item("⋮⋮")
                                    .item(
                                        El::new()
                                            .s(Font::new().color(hsluv!(220, 10, 85)).size(13))
                                            .child(var_names[i as usize])
                                    )
                                    .item("❌")
                            )
                            .item(
                                // Value column (60px width)
                                El::new()
                                    .s(Width::exact(60))
                                    .s(Font::new().color(hsluv!(220, 10, 75)).size(13))
                                    .child(values[i as usize])
                            )
                            .item(
                                // Waveform column (fills remaining width)
                                Row::new()
                                    .s(Width::fill())
                                    .s(Height::exact(20))
                                    .s(Gap::new().x(1))
                                    .s(Padding::new().x(10))
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
                            )
                    }))
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
                        dock_toggle_button()
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
                                    .on_press(|| {})
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
                                    .on_press(|| {})
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
                            .on_press(|| {})
                            .build()
                    )
                    .item(
                        button()
                            .label("Zoom Out")
                            .left_icon(IconName::ZoomOut)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Small)
                            .on_press(|| {})
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
