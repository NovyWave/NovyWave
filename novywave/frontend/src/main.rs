use zoon::{*, futures_util::future::try_join_all};
use std::f32::consts::PI;
use std::mem;

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

fn root() -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Width::fill())
        .s(Scrollbars::both())
        .s(Background::new().color(color!("Black")))
        .child(
            Column::new()
                .s(Gap::both(10))
                .s(Scrollbars::both())
                .s(Padding::all(10))
                .items(examples().map(panel_with_canvas))
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
