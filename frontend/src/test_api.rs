use crate::selected_variables::SelectedVariables;
use crate::tracked_files::TrackedFiles;
use crate::visualizer::timeline::timeline_actor::WaveformTimeline;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

pub struct TestApiState {
    pub tracked_files: TrackedFiles,
    pub selected_variables: SelectedVariables,
    pub waveform_timeline: WaveformTimeline,
}

thread_local! {
    static TEST_API_STATE: RefCell<Option<TestApiState>> = const { RefCell::new(None) };
}

pub fn store_test_api_state(
    tracked_files: TrackedFiles,
    selected_variables: SelectedVariables,
    waveform_timeline: WaveformTimeline,
) {
    TEST_API_STATE.with(|cell| {
        *cell.borrow_mut() = Some(TestApiState {
            tracked_files,
            selected_variables,
            waveform_timeline,
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
    expose_test_api_js();
}

#[wasm_bindgen(inline_js = r#"
export function expose_test_api_js() {
    if (typeof window === 'undefined') return;

    window.__novywave_test_api = {
        getTimelineState: function() {
            try {
                return window.__novywave_get_timeline_state();
            } catch (e) {
                console.error('[NovyWave Test API] getTimelineState error:', e);
                return null;
            }
        },
        getCursorValues: function() {
            try {
                return window.__novywave_get_cursor_values();
            } catch (e) {
                console.error('[NovyWave Test API] getCursorValues error:', e);
                return {};
            }
        },
        getSelectedVariables: function() {
            try {
                return window.__novywave_get_selected_variables();
            } catch (e) {
                console.error('[NovyWave Test API] getSelectedVariables error:', e);
                return [];
            }
        },
        getLoadedFiles: function() {
            try {
                return window.__novywave_get_loaded_files();
            } catch (e) {
                console.error('[NovyWave Test API] getLoadedFiles error:', e);
                return [];
            }
        }
    };

    console.log('[NovyWave] Test API exposed on window.__novywave_test_api');
}
"#)]
extern "C" {
    fn expose_test_api_js();
}

#[wasm_bindgen(js_name = "__novywave_get_timeline_state")]
pub fn get_timeline_state() -> JsValue {
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

#[wasm_bindgen(js_name = "__novywave_get_cursor_values")]
pub fn get_cursor_values() -> JsValue {
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

#[wasm_bindgen(js_name = "__novywave_get_selected_variables")]
pub fn get_selected_variables() -> JsValue {
    with_state(|state| {
        let variables = state.selected_variables.variables_vec_actor.get_cloned();
        let arr = js_sys::Array::new();

        for var in variables.iter() {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"uniqueId".into(), &var.unique_id.clone().into()).ok();

            if let Some(name) = var.variable_name() {
                js_sys::Reflect::set(&obj, &"name".into(), &name.into()).ok();
            }

            if let Some(scope) = var.scope_path() {
                js_sys::Reflect::set(&obj, &"scopePath".into(), &scope.into()).ok();
            }

            if let Some(ref fmt) = var.formatter {
                js_sys::Reflect::set(&obj, &"format".into(), &format!("{:?}", fmt).into()).ok();
            }

            arr.push(&obj);
        }

        arr.into()
    })
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "__novywave_get_loaded_files")]
pub fn get_loaded_files() -> JsValue {
    with_state(|state| {
        let files = state.tracked_files.get_current_files();
        let arr = js_sys::Array::new();

        for file in files.iter() {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"id".into(), &file.id.clone().into()).ok();
            js_sys::Reflect::set(&obj, &"path".into(), &file.path.clone().into()).ok();
            js_sys::Reflect::set(&obj, &"filename".into(), &file.filename.clone().into()).ok();
            js_sys::Reflect::set(&obj, &"status".into(), &format!("{:?}", file.state).into()).ok();
            js_sys::Reflect::set(&obj, &"smartLabel".into(), &file.smart_label.clone().into()).ok();

            arr.push(&obj);
        }

        arr.into()
    })
    .unwrap_or(JsValue::NULL)
}
