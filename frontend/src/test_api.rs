use crate::selected_variables::SelectedVariables;
use crate::tracked_files::TrackedFiles;
use crate::visualizer::timeline::timeline_actor::WaveformTimeline;
use std::cell::RefCell;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use zoon::SendWrapper;

pub struct TestApiState {
    pub tracked_files: TrackedFiles,
    pub selected_variables: SelectedVariables,
    pub waveform_timeline: WaveformTimeline,
    pub connection: Arc<SendWrapper<zoon::Connection<shared::UpMsg, shared::DownMsg>>>,
}

thread_local! {
    static TEST_API_STATE: RefCell<Option<TestApiState>> = const { RefCell::new(None) };
}

pub fn store_test_api_state(
    tracked_files: TrackedFiles,
    selected_variables: SelectedVariables,
    waveform_timeline: WaveformTimeline,
    connection: Arc<SendWrapper<zoon::Connection<shared::UpMsg, shared::DownMsg>>>,
) {
    TEST_API_STATE.with(|cell| {
        *cell.borrow_mut() = Some(TestApiState {
            tracked_files,
            selected_variables,
            waveform_timeline,
            connection,
        });
    });
}

fn with_state<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&TestApiState) -> R,
{
    TEST_API_STATE.with(|cell| cell.borrow().as_ref().map(f))
}

#[wasm_bindgen]
pub fn expose_novywave_test_api() {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };

    let api = js_sys::Object::new();

    let get_timeline_state_closure =
        Closure::wrap(Box::new(get_timeline_state_impl) as Box<dyn Fn() -> JsValue>);
    js_sys::Reflect::set(
        &api,
        &"getTimelineState".into(),
        get_timeline_state_closure.as_ref().unchecked_ref(),
    )
    .ok();
    get_timeline_state_closure.forget();

    let get_cursor_values_closure =
        Closure::wrap(Box::new(get_cursor_values_impl) as Box<dyn Fn() -> JsValue>);
    js_sys::Reflect::set(
        &api,
        &"getCursorValues".into(),
        get_cursor_values_closure.as_ref().unchecked_ref(),
    )
    .ok();
    get_cursor_values_closure.forget();

    let get_selected_variables_closure =
        Closure::wrap(Box::new(get_selected_variables_impl) as Box<dyn Fn() -> JsValue>);
    js_sys::Reflect::set(
        &api,
        &"getSelectedVariables".into(),
        get_selected_variables_closure.as_ref().unchecked_ref(),
    )
    .ok();
    get_selected_variables_closure.forget();

    let get_loaded_files_closure =
        Closure::wrap(Box::new(get_loaded_files_impl) as Box<dyn Fn() -> JsValue>);
    js_sys::Reflect::set(
        &api,
        &"getLoadedFiles".into(),
        get_loaded_files_closure.as_ref().unchecked_ref(),
    )
    .ok();
    get_loaded_files_closure.forget();

    let select_workspace_closure =
        Closure::wrap(Box::new(select_workspace_impl) as Box<dyn Fn(String) -> bool>);
    js_sys::Reflect::set(
        &api,
        &"selectWorkspace".into(),
        select_workspace_closure.as_ref().unchecked_ref(),
    )
    .ok();
    select_workspace_closure.forget();

    js_sys::Reflect::set(&window, &"__novywave_test_api".into(), &api).ok();

    zoon::println!("[NovyWave] Test API exposed on window.__novywave_test_api");
}

fn get_timeline_state_impl() -> JsValue {
    with_state(|state| {
        let render_state = state.waveform_timeline.render_state_actor().get_cloned();

        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &"viewportStartPs".into(),
            &JsValue::from_f64(render_state.viewport_start.0 as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"viewportEndPs".into(),
            &JsValue::from_f64(render_state.viewport_end.0 as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"cursorPs".into(),
            &JsValue::from_f64(render_state.cursor.0 as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"zoomCenterPs".into(),
            &JsValue::from_f64(render_state.zoom_center.0 as f64),
        )
        .ok();

        obj.into()
    })
    .unwrap_or(JsValue::NULL)
}

fn get_cursor_values_impl() -> JsValue {
    with_state(|state| {
        let cursor_values = state.waveform_timeline.cursor_values_actor().get_cloned();
        let obj = js_sys::Object::new();

        for (key, value) in cursor_values.iter() {
            let value_str = format!("{:?}", value);
            js_sys::Reflect::set(&obj, &key.into(), &value_str.into()).ok();
        }

        obj.into()
    })
    .unwrap_or(JsValue::NULL)
}

fn get_selected_variables_impl() -> JsValue {
    with_state(|state| {
        let variables = state.selected_variables.variables_vec_actor.get_cloned();
        let arr = js_sys::Array::new();

        for var in variables.iter() {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"uniqueId".into(), &var.unique_id.clone().into()).ok();

            // Parse unique_id to get name and scope
            let (name, scope_path) = if let Some((_, scope, name)) = var.parse_unique_id() {
                (name, scope)
            } else {
                (String::new(), String::new())
            };

            js_sys::Reflect::set(&obj, &"name".into(), &name.into()).ok();

            // scopePath must be an array to match protocol.rs VariableInfo
            let scope_arr = js_sys::Array::new();
            if !scope_path.is_empty() {
                scope_arr.push(&scope_path.into());
            }
            js_sys::Reflect::set(&obj, &"scopePath".into(), &scope_arr).ok();

            let format_str = var
                .formatter
                .as_ref()
                .map(|f| format!("{:?}", f))
                .unwrap_or_else(|| "None".to_string());
            js_sys::Reflect::set(&obj, &"format".into(), &format_str.into()).ok();

            arr.push(&obj);
        }

        arr.into()
    })
    .unwrap_or(JsValue::NULL)
}

fn get_loaded_files_impl() -> JsValue {
    with_state(|state| {
        let files = state.tracked_files.get_current_files();
        let arr = js_sys::Array::new();

        for file in files.iter() {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"id".into(), &file.id.clone().into()).ok();
            js_sys::Reflect::set(&obj, &"path".into(), &file.path.clone().into()).ok();
            js_sys::Reflect::set(&obj, &"filename".into(), &file.filename.clone().into()).ok();
            js_sys::Reflect::set(&obj, &"status".into(), &format!("{:?}", file.state).into()).ok();
            js_sys::Reflect::set(&obj, &"smartLabel".into(), &file.smart_label.clone().into())
                .ok();

            arr.push(&obj);
        }

        arr.into()
    })
    .unwrap_or(JsValue::NULL)
}

fn select_workspace_impl(path: String) -> bool {
    zoon::println!("[Test API] selectWorkspace called with: {}", path);

    TEST_API_STATE.with(|cell| {
        if let Some(state) = cell.borrow().as_ref() {
            let connection = state.connection.clone();
            zoon::Task::start(async move {
                let msg = shared::UpMsg::SelectWorkspace { root: path };
                zoon::println!("[Test API] Sending SelectWorkspace message...");
                match connection.send_up_msg(msg).await {
                    Ok(_) => zoon::println!("[Test API] SelectWorkspace sent successfully"),
                    Err(e) => zoon::eprintln!("[Test API] Failed to send SelectWorkspace: {:?}", e),
                }
            });
            true
        } else {
            zoon::eprintln!("[Test API] selectWorkspace: state not initialized");
            false
        }
    })
}
