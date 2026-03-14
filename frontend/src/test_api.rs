use crate::dragging::DraggingSystem;
use crate::selected_variables::SelectedVariables;
use crate::tracked_files::TrackedFiles;
use crate::visualizer::timeline::TimePs;
use crate::visualizer::timeline::timeline_actor::WaveformTimeline;
use shared::AnalogLimits;
use std::cell::RefCell;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, closure::Closure};
use zoon::SendWrapper;

pub struct TestApiState {
    pub tracked_files: TrackedFiles,
    pub selected_variables: SelectedVariables,
    pub waveform_timeline: WaveformTimeline,
    pub dragging_system: DraggingSystem,
    pub app_config: crate::config::AppConfig,
    pub connection: Arc<SendWrapper<zoon::Connection<shared::UpMsg, shared::DownMsg>>>,
}

thread_local! {
    static TEST_API_STATE: RefCell<Option<TestApiState>> = const { RefCell::new(None) };
}

pub fn store_test_api_state(
    tracked_files: TrackedFiles,
    selected_variables: SelectedVariables,
    waveform_timeline: WaveformTimeline,
    dragging_system: DraggingSystem,
    app_config: crate::config::AppConfig,
    connection: Arc<SendWrapper<zoon::Connection<shared::UpMsg, shared::DownMsg>>>,
) {
    TEST_API_STATE.with(|cell| {
        *cell.borrow_mut() = Some(TestApiState {
            tracked_files,
            selected_variables,
            waveform_timeline,
            dragging_system,
            app_config,
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

    let get_visible_rows_closure =
        Closure::wrap(Box::new(get_visible_rows_impl) as Box<dyn Fn() -> JsValue>);
    js_sys::Reflect::set(
        &api,
        &"getVisibleRows".into(),
        get_visible_rows_closure.as_ref().unchecked_ref(),
    )
    .ok();
    get_visible_rows_closure.forget();

    let get_markers_closure = Closure::wrap(Box::new(get_markers_impl) as Box<dyn Fn() -> JsValue>);
    js_sys::Reflect::set(
        &api,
        &"getMarkers".into(),
        get_markers_closure.as_ref().unchecked_ref(),
    )
    .ok();
    get_markers_closure.forget();

    let get_file_picker_roots_closure =
        Closure::wrap(Box::new(get_file_picker_roots_impl) as Box<dyn Fn() -> JsValue>);
    js_sys::Reflect::set(
        &api,
        &"getFilePickerRoots".into(),
        get_file_picker_roots_closure.as_ref().unchecked_ref(),
    )
    .ok();
    get_file_picker_roots_closure.forget();

    let get_config_debug_closure =
        Closure::wrap(Box::new(get_config_debug_impl) as Box<dyn Fn() -> JsValue>);
    js_sys::Reflect::set(
        &api,
        &"getConfigDebug".into(),
        get_config_debug_closure.as_ref().unchecked_ref(),
    )
    .ok();
    get_config_debug_closure.forget();

    let get_perf_counters_closure =
        Closure::wrap(Box::new(get_perf_counters_impl) as Box<dyn Fn() -> JsValue>);
    js_sys::Reflect::set(
        &api,
        &"getPerfCounters".into(),
        get_perf_counters_closure.as_ref().unchecked_ref(),
    )
    .ok();
    get_perf_counters_closure.forget();

    let reset_perf_counters_closure =
        Closure::wrap(Box::new(reset_perf_counters_impl) as Box<dyn Fn() -> bool>);
    js_sys::Reflect::set(
        &api,
        &"resetPerfCounters".into(),
        reset_perf_counters_closure.as_ref().unchecked_ref(),
    )
    .ok();
    reset_perf_counters_closure.forget();

    let start_frame_sampler_closure =
        Closure::wrap(Box::new(start_frame_sampler_impl) as Box<dyn Fn() -> bool>);
    js_sys::Reflect::set(
        &api,
        &"startFrameSampler".into(),
        start_frame_sampler_closure.as_ref().unchecked_ref(),
    )
    .ok();
    start_frame_sampler_closure.forget();

    let stop_frame_sampler_closure =
        Closure::wrap(Box::new(stop_frame_sampler_impl) as Box<dyn Fn() -> JsValue>);
    js_sys::Reflect::set(
        &api,
        &"stopFrameSampler".into(),
        stop_frame_sampler_closure.as_ref().unchecked_ref(),
    )
    .ok();
    stop_frame_sampler_closure.forget();

    let save_config_now_closure =
        Closure::wrap(Box::new(save_config_now_impl) as Box<dyn Fn() -> bool>);
    js_sys::Reflect::set(
        &api,
        &"saveConfigNow".into(),
        save_config_now_closure.as_ref().unchecked_ref(),
    )
    .ok();
    save_config_now_closure.forget();

    let select_workspace_closure =
        Closure::wrap(Box::new(select_workspace_impl) as Box<dyn Fn(String) -> bool>);
    js_sys::Reflect::set(
        &api,
        &"selectWorkspace".into(),
        select_workspace_closure.as_ref().unchecked_ref(),
    )
    .ok();
    select_workspace_closure.forget();

    let set_cursor_ps_closure =
        Closure::wrap(Box::new(set_cursor_ps_impl) as Box<dyn Fn(f64) -> bool>);
    js_sys::Reflect::set(
        &api,
        &"setCursorPs".into(),
        set_cursor_ps_closure.as_ref().unchecked_ref(),
    )
    .ok();
    set_cursor_ps_closure.forget();

    let set_pointer_hover_closure =
        Closure::wrap(Box::new(set_pointer_hover_impl) as Box<dyn Fn(f64, f64) -> bool>);
    js_sys::Reflect::set(
        &api,
        &"setPointerHover".into(),
        set_pointer_hover_closure.as_ref().unchecked_ref(),
    )
    .ok();
    set_pointer_hover_closure.forget();

    let clear_pointer_hover_closure =
        Closure::wrap(Box::new(clear_pointer_hover_impl) as Box<dyn Fn() -> bool>);
    js_sys::Reflect::set(
        &api,
        &"clearPointerHover".into(),
        clear_pointer_hover_closure.as_ref().unchecked_ref(),
    )
    .ok();
    clear_pointer_hover_closure.forget();

    let add_marker_closure =
        Closure::wrap(Box::new(add_marker_impl) as Box<dyn Fn(String) -> bool>);
    js_sys::Reflect::set(
        &api,
        &"addMarker".into(),
        add_marker_closure.as_ref().unchecked_ref(),
    )
    .ok();
    add_marker_closure.forget();

    let remove_marker_closure =
        Closure::wrap(Box::new(remove_marker_impl) as Box<dyn Fn(f64) -> bool>);
    js_sys::Reflect::set(
        &api,
        &"removeMarker".into(),
        remove_marker_closure.as_ref().unchecked_ref(),
    )
    .ok();
    remove_marker_closure.forget();

    let rename_marker_closure =
        Closure::wrap(Box::new(rename_marker_impl) as Box<dyn Fn(f64, String) -> bool>);
    js_sys::Reflect::set(
        &api,
        &"renameMarker".into(),
        rename_marker_closure.as_ref().unchecked_ref(),
    )
    .ok();
    rename_marker_closure.forget();

    let jump_to_marker_closure =
        Closure::wrap(Box::new(jump_to_marker_impl) as Box<dyn Fn(f64) -> bool>);
    js_sys::Reflect::set(
        &api,
        &"jumpToMarker".into(),
        jump_to_marker_closure.as_ref().unchecked_ref(),
    )
    .ok();
    jump_to_marker_closure.forget();

    let set_row_height_closure =
        Closure::wrap(Box::new(set_row_height_impl) as Box<dyn Fn(String, f64) -> bool>);
    js_sys::Reflect::set(
        &api,
        &"setRowHeight".into(),
        set_row_height_closure.as_ref().unchecked_ref(),
    )
    .ok();
    set_row_height_closure.forget();

    let start_row_resize_closure =
        Closure::wrap(Box::new(start_row_resize_impl) as Box<dyn Fn(String) -> bool>);
    js_sys::Reflect::set(
        &api,
        &"startRowResize".into(),
        start_row_resize_closure.as_ref().unchecked_ref(),
    )
    .ok();
    start_row_resize_closure.forget();

    let move_active_drag_closure =
        Closure::wrap(Box::new(move_active_drag_impl) as Box<dyn Fn(f64) -> bool>);
    js_sys::Reflect::set(
        &api,
        &"moveActiveDrag".into(),
        move_active_drag_closure.as_ref().unchecked_ref(),
    )
    .ok();
    move_active_drag_closure.forget();

    let end_active_drag_closure =
        Closure::wrap(Box::new(end_active_drag_impl) as Box<dyn Fn() -> bool>);
    js_sys::Reflect::set(
        &api,
        &"endActiveDrag".into(),
        end_active_drag_closure.as_ref().unchecked_ref(),
    )
    .ok();
    end_active_drag_closure.forget();

    let set_variable_format_closure =
        Closure::wrap(Box::new(set_variable_format_impl) as Box<dyn Fn(String, String) -> bool>);
    js_sys::Reflect::set(
        &api,
        &"setVariableFormat".into(),
        set_variable_format_closure.as_ref().unchecked_ref(),
    )
    .ok();
    set_variable_format_closure.forget();

    let set_analog_limits_closure = Closure::wrap(
        Box::new(set_analog_limits_impl) as Box<dyn Fn(String, bool, f64, f64) -> bool>
    );
    js_sys::Reflect::set(
        &api,
        &"setAnalogLimits".into(),
        set_analog_limits_closure.as_ref().unchecked_ref(),
    )
    .ok();
    set_analog_limits_closure.forget();

    let create_group_closure =
        Closure::wrap(Box::new(create_group_impl) as Box<dyn Fn(String, JsValue) -> bool>);
    js_sys::Reflect::set(
        &api,
        &"createGroup".into(),
        create_group_closure.as_ref().unchecked_ref(),
    )
    .ok();
    create_group_closure.forget();

    let rename_group_closure =
        Closure::wrap(Box::new(rename_group_impl) as Box<dyn Fn(f64, String) -> bool>);
    js_sys::Reflect::set(
        &api,
        &"renameGroup".into(),
        rename_group_closure.as_ref().unchecked_ref(),
    )
    .ok();
    rename_group_closure.forget();

    let toggle_group_collapse_closure =
        Closure::wrap(Box::new(toggle_group_collapse_impl) as Box<dyn Fn(f64) -> bool>);
    js_sys::Reflect::set(
        &api,
        &"toggleGroupCollapse".into(),
        toggle_group_collapse_closure.as_ref().unchecked_ref(),
    )
    .ok();
    toggle_group_collapse_closure.forget();

    let delete_group_closure =
        Closure::wrap(Box::new(delete_group_impl) as Box<dyn Fn(f64) -> bool>);
    js_sys::Reflect::set(
        &api,
        &"deleteGroup".into(),
        delete_group_closure.as_ref().unchecked_ref(),
    )
    .ok();
    delete_group_closure.forget();

    js_sys::Reflect::set(&window, &"__novywave_test_api".into(), &api).ok();

    zoon::println!("[NovyWave] Test API exposed on window.__novywave_test_api");
}

fn get_timeline_state_impl() -> JsValue {
    with_state(|state| {
        let render_state = state.waveform_timeline.render_state_actor().get_cloned();
        let debug_metrics = state.waveform_timeline.debug_metrics_actor().get_cloned();

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
        js_sys::Reflect::set(
            &obj,
            &"canvasWidthPx".into(),
            &JsValue::from_f64(render_state.canvas_width_px as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"canvasHeightPx".into(),
            &JsValue::from_f64(render_state.canvas_height_px as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"renderRowsLen".into(),
            &JsValue::from_f64(render_state.rows.len() as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"renderVariablesLen".into(),
            &JsValue::from_f64(render_state.variables.len() as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"renderCount".into(),
            &JsValue::from_f64(debug_metrics.render_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"fullRenderCount".into(),
            &JsValue::from_f64(debug_metrics.full_render_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"layoutRenderCount".into(),
            &JsValue::from_f64(debug_metrics.layout_render_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"requestSendCount".into(),
            &JsValue::from_f64(debug_metrics.request_send_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"requestDedupedCount".into(),
            &JsValue::from_f64(debug_metrics.request_deduped_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"startupInitialQuerySendCount".into(),
            &JsValue::from_f64(debug_metrics.startup_initial_query_send_count as f64),
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
            js_sys::Reflect::set(
                &obj,
                &"signalType".into(),
                &var.signal_type.clone().unwrap_or_default().into(),
            )
            .ok();
            js_sys::Reflect::set(
                &obj,
                &"rowHeight".into(),
                &JsValue::from_f64(var.row_height.unwrap_or(30) as f64),
            )
            .ok();
            if let Some(limits) = &var.analog_limits {
                let limits_obj = js_sys::Object::new();
                js_sys::Reflect::set(
                    &limits_obj,
                    &"auto".into(),
                    &JsValue::from_bool(limits.auto),
                )
                .ok();
                js_sys::Reflect::set(&limits_obj, &"min".into(), &JsValue::from_f64(limits.min))
                    .ok();
                js_sys::Reflect::set(&limits_obj, &"max".into(), &JsValue::from_f64(limits.max))
                    .ok();
                js_sys::Reflect::set(&obj, &"analogLimits".into(), &limits_obj).ok();
            } else {
                js_sys::Reflect::set(&obj, &"analogLimits".into(), &JsValue::NULL).ok();
            }

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
            js_sys::Reflect::set(&obj, &"smartLabel".into(), &file.smart_label.clone().into()).ok();

            arr.push(&obj);
        }

        arr.into()
    })
    .unwrap_or(JsValue::NULL)
}

fn get_visible_rows_impl() -> JsValue {
    with_state(|state| {
        let rows = state.selected_variables.visible_items.get_cloned();
        let arr = js_sys::Array::new();

        for row in rows {
            let obj = js_sys::Object::new();
            match row {
                crate::selected_variables::SelectedVariableOrGroup::GroupHeader {
                    index,
                    name,
                    collapsed,
                    member_count,
                } => {
                    js_sys::Reflect::set(&obj, &"kind".into(), &"group".into()).ok();
                    js_sys::Reflect::set(&obj, &"index".into(), &JsValue::from_f64(index as f64))
                        .ok();
                    js_sys::Reflect::set(&obj, &"name".into(), &name.into()).ok();
                    js_sys::Reflect::set(&obj, &"collapsed".into(), &JsValue::from_bool(collapsed))
                        .ok();
                    js_sys::Reflect::set(
                        &obj,
                        &"memberCount".into(),
                        &JsValue::from_f64(member_count as f64),
                    )
                    .ok();
                    js_sys::Reflect::set(&obj, &"rowHeight".into(), &JsValue::from_f64(30.0)).ok();
                }
                crate::selected_variables::SelectedVariableOrGroup::Variable(variable) => {
                    let row_height = state
                        .selected_variables
                        .live_row_height(&variable.unique_id);
                    js_sys::Reflect::set(&obj, &"kind".into(), &"variable".into()).ok();
                    js_sys::Reflect::set(
                        &obj,
                        &"uniqueId".into(),
                        &variable.unique_id.clone().into(),
                    )
                    .ok();
                    js_sys::Reflect::set(
                        &obj,
                        &"name".into(),
                        &variable.variable_name().unwrap_or_default().into(),
                    )
                    .ok();
                    js_sys::Reflect::set(
                        &obj,
                        &"rowHeight".into(),
                        &JsValue::from_f64(row_height as f64),
                    )
                    .ok();
                }
            }
            arr.push(&obj);
        }

        arr.into()
    })
    .unwrap_or(JsValue::NULL)
}

fn get_markers_impl() -> JsValue {
    with_state(|state| {
        let mut markers = state.waveform_timeline.markers_snapshot.get_cloned();
        markers.sort_by_key(|marker| marker.time_ps);
        let arr = js_sys::Array::new();

        for (index, marker) in markers.iter().enumerate() {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"index".into(), &JsValue::from_f64(index as f64)).ok();
            js_sys::Reflect::set(&obj, &"name".into(), &marker.name.clone().into()).ok();
            js_sys::Reflect::set(
                &obj,
                &"timePs".into(),
                &JsValue::from_f64(marker.time_ps as f64),
            )
            .ok();
            arr.push(&obj);
        }

        arr.into()
    })
    .unwrap_or(JsValue::NULL)
}

fn get_file_picker_roots_impl() -> JsValue {
    with_state(|state| {
        let roots = state
            .app_config
            .file_picker_domain
            .platform_roots
            .get_cloned()
            .unwrap_or_default();
        let expanded = state.app_config.file_picker_domain.get_expanded_snapshot();
        let obj = js_sys::Object::new();
        let roots_arr = js_sys::Array::new();
        let expanded_arr = js_sys::Array::new();

        for root in roots {
            let root_obj = js_sys::Object::new();
            js_sys::Reflect::set(&root_obj, &"path".into(), &root.path.into()).ok();
            js_sys::Reflect::set(&root_obj, &"label".into(), &root.label.into()).ok();
            js_sys::Reflect::set(
                &root_obj,
                &"quickAccess".into(),
                &JsValue::from_bool(root.is_quick_access),
            )
            .ok();
            roots_arr.push(&root_obj);
        }

        for path in expanded {
            expanded_arr.push(&JsValue::from_str(&path));
        }

        js_sys::Reflect::set(&obj, &"roots".into(), &roots_arr).ok();
        js_sys::Reflect::set(&obj, &"expanded".into(), &expanded_arr).ok();
        obj.into()
    })
    .unwrap_or(JsValue::NULL)
}

fn get_config_debug_impl() -> JsValue {
    with_state(|state| {
        let obj = js_sys::Object::new();
        let debug_metrics = state.app_config.debug_metrics.get_cloned();
        js_sys::Reflect::set(
            &obj,
            &"configLoaded".into(),
            &JsValue::from_bool(state.app_config.is_config_loaded()),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"serverReady".into(),
            &JsValue::from_bool(crate::platform::server_is_ready()),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"selectedVariablesSnapshotLen".into(),
            &JsValue::from_f64(
                state
                    .app_config
                    .selected_variables_snapshot
                    .get_cloned()
                    .len() as f64,
            ),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"markersConfigLen".into(),
            &JsValue::from_f64(state.app_config.markers_config.get_cloned().len() as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"signalGroupsConfigLen".into(),
            &JsValue::from_f64(state.app_config.signal_groups_config.get_cloned().len() as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"expandedDirectoriesLen".into(),
            &JsValue::from_f64(
                state
                    .app_config
                    .file_picker_domain
                    .get_expanded_snapshot()
                    .len() as f64,
            ),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"theme".into(),
            &format!("{:?}", state.app_config.theme.get()).into(),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"dockMode".into(),
            &format!("{:?}", state.app_config.dock_mode.get_cloned()).into(),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"tooltipEnabled".into(),
            &JsValue::from_bool(state.waveform_timeline.tooltip_visibility_handle().get()),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"saveSendCount".into(),
            &JsValue::from_f64(debug_metrics.save_send_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"saveDedupedCount".into(),
            &JsValue::from_f64(debug_metrics.save_deduped_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"startupPlatformRootsRequestCount".into(),
            &JsValue::from_f64(debug_metrics.startup_platform_roots_request_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"startupBrowseRequestCount".into(),
            &JsValue::from_f64(debug_metrics.startup_browse_request_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"restorePhase".into(),
            &format!("{:?}", state.app_config.restore_phase.get_cloned()).into(),
        )
        .ok();
        obj.into()
    })
    .unwrap_or(JsValue::NULL)
}

fn get_perf_counters_impl() -> JsValue {
    with_state(|state| {
        let obj = js_sys::Object::new();
        let timeline = state.waveform_timeline.debug_metrics_actor().get_cloned();
        let config = state.app_config.debug_metrics.get_cloned();
        let dragging = state.dragging_system.debug_metrics_actor().get_cloned();

        js_sys::Reflect::set(
            &obj,
            &"dragUpdateCount".into(),
            &JsValue::from_f64(dragging.drag_update_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"appliedRowResizeCount".into(),
            &JsValue::from_f64(dragging.applied_row_resize_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"renderCount".into(),
            &JsValue::from_f64(timeline.render_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"fullRenderCount".into(),
            &JsValue::from_f64(timeline.full_render_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"layoutRenderCount".into(),
            &JsValue::from_f64(timeline.layout_render_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"requestSendCount".into(),
            &JsValue::from_f64(timeline.request_send_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"requestDedupedCount".into(),
            &JsValue::from_f64(timeline.request_deduped_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"saveSendCount".into(),
            &JsValue::from_f64(config.save_send_count as f64),
        )
        .ok();
        js_sys::Reflect::set(
            &obj,
            &"saveDedupedCount".into(),
            &JsValue::from_f64(config.save_deduped_count as f64),
        )
        .ok();
        obj.into()
    })
    .unwrap_or(JsValue::NULL)
}

fn reset_perf_counters_impl() -> bool {
    with_state(|state| {
        state.dragging_system.reset_debug_metrics();
        state
            .waveform_timeline
            .debug_metrics_actor()
            .set(crate::visualizer::timeline::timeline_actor::TimelineDebugMetrics::default());
        state
            .app_config
            .debug_metrics
            .set(crate::config::ConfigDebugMetrics::default());
        true
    })
    .unwrap_or(false)
}

fn start_frame_sampler_impl() -> bool {
    js_sys::eval(
        r#"
        (() => {
            const state = window.__novywave_frame_sampler || (window.__novywave_frame_sampler = {
                active: false,
                deltasMs: [],
                lastTimestampMs: null,
                rafId: 0,
            });
            state.active = true;
            state.deltasMs = [];
            state.lastTimestampMs = null;
            const step = (timestampMs) => {
                if (!state.active) return;
                if (state.lastTimestampMs !== null) {
                    state.deltasMs.push(timestampMs - state.lastTimestampMs);
                }
                state.lastTimestampMs = timestampMs;
                state.rafId = window.requestAnimationFrame(step);
            };
            state.rafId = window.requestAnimationFrame(step);
            return true;
        })()
        "#,
    )
    .map(|value| value.as_bool().unwrap_or(false))
    .unwrap_or(false)
}

fn stop_frame_sampler_impl() -> JsValue {
    js_sys::eval(
        r#"
        (() => {
            const state = window.__novywave_frame_sampler || {
                active: false,
                deltasMs: [],
                lastTimestampMs: null,
                rafId: 0,
            };
            state.active = false;
            if (state.rafId) {
                window.cancelAnimationFrame(state.rafId);
                state.rafId = 0;
            }
            const deltas = [...state.deltasMs].sort((a, b) => a - b);
            const percentile = (p) => {
                if (deltas.length === 0) return 0;
                const index = Math.min(
                    deltas.length - 1,
                    Math.round((deltas.length - 1) * Math.max(0, Math.min(1, p)))
                );
                return deltas[index];
            };
            const result = {
                sampleCount: deltas.length,
                p50Ms: percentile(0.5),
                p95Ms: percentile(0.95),
                maxMs: deltas.length === 0 ? 0 : deltas[deltas.length - 1],
                over16_7Count: deltas.filter((delta) => delta > 16.7).length,
                over33Count: deltas.filter((delta) => delta > 33).length,
            };
            state.deltasMs = [];
            state.lastTimestampMs = null;
            window.__novywave_frame_sampler = state;
            return result;
        })()
        "#,
    )
    .unwrap_or(JsValue::NULL)
}

fn save_config_now_impl() -> bool {
    TEST_API_STATE.with(|cell| {
        let binding = cell.borrow();
        let Some(state) = binding.as_ref() else {
            return false;
        };

        let Some(shared_config) = state
            .app_config
            .compose_current_shared_config(&state.tracked_files, &state.selected_variables)
        else {
            return false;
        };

        let connection = state.connection.clone();
        zoon::Task::start(async move {
            if let Err(error) = connection
                .send_up_msg(shared::UpMsg::SaveConfig(shared_config))
                .await
            {
                zoon::eprintln!("[Test API] saveConfigNow failed: {:?}", error);
            }
        });

        true
    })
}

fn select_workspace_impl(path: String) -> bool {
    TEST_API_STATE.with(|cell| {
        if let Some(state) = cell.borrow().as_ref() {
            let connection = state.connection.clone();
            zoon::Task::start(async move {
                let msg = shared::UpMsg::SelectWorkspace { root: path };
                match connection.send_up_msg(msg).await {
                    Ok(_) => {}
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

fn set_cursor_ps_impl(time_ps: f64) -> bool {
    if !time_ps.is_finite() || time_ps < 0.0 {
        return false;
    }

    with_state(|state| {
        state
            .waveform_timeline
            .set_cursor_clamped(TimePs::from_picoseconds(time_ps.round() as u64));
        true
    })
    .unwrap_or(false)
}

fn set_pointer_hover_impl(normalized_x: f64, normalized_y: f64) -> bool {
    if !normalized_x.is_finite() || !normalized_y.is_finite() {
        return false;
    }

    with_state(|state| {
        state.waveform_timeline.set_pointer_hover(Some(
            crate::visualizer::timeline::timeline_actor::TimelinePointerHover {
                normalized_x: normalized_x.clamp(0.0, 1.0),
                normalized_y: normalized_y.clamp(0.0, 1.0),
            },
        ));
        true
    })
    .unwrap_or(false)
}

fn clear_pointer_hover_impl() -> bool {
    with_state(|state| {
        state.waveform_timeline.set_pointer_hover(None);
        true
    })
    .unwrap_or(false)
}

fn add_marker_impl(name: String) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }

    with_state(|state| {
        state.waveform_timeline.add_marker(trimmed.to_string());
        state
            .app_config
            .markers_config
            .set(state.waveform_timeline.markers_as_config());
        state.app_config.request_save();
        true
    })
    .unwrap_or(false)
}

fn remove_marker_impl(index: f64) -> bool {
    let Some(index) = f64_to_usize(index) else {
        return false;
    };

    with_state(|state| {
        state.waveform_timeline.remove_marker(index);
        state
            .app_config
            .markers_config
            .set(state.waveform_timeline.markers_as_config());
        state.app_config.request_save();
        true
    })
    .unwrap_or(false)
}

fn rename_marker_impl(index: f64, name: String) -> bool {
    let Some(index) = f64_to_usize(index) else {
        return false;
    };
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }

    with_state(|state| {
        state
            .waveform_timeline
            .rename_marker(index, trimmed.to_string());
        state
            .app_config
            .markers_config
            .set(state.waveform_timeline.markers_as_config());
        state.app_config.request_save();
        true
    })
    .unwrap_or(false)
}

fn jump_to_marker_impl(index: f64) -> bool {
    let Some(index) = f64_to_usize(index) else {
        return false;
    };

    with_state(|state| {
        state.waveform_timeline.jump_to_marker(index);
        true
    })
    .unwrap_or(false)
}

fn start_row_resize_impl(unique_id: String) -> bool {
    let trimmed_id = unique_id.trim();
    if trimmed_id.is_empty() {
        return false;
    }

    with_state(|state| {
        state.dragging_system.start_drag(
            crate::dragging::DividerType::SignalRowDivider {
                unique_id: trimmed_id.to_string(),
            },
            (0.0, 0.0),
        );
        true
    })
    .unwrap_or(false)
}

fn move_active_drag_impl(delta_y: f64) -> bool {
    if !delta_y.is_finite() {
        return false;
    }

    with_state(|state| {
        state
            .dragging_system
            .process_drag_movement((0.0, delta_y as f32));
        true
    })
    .unwrap_or(false)
}

fn end_active_drag_impl() -> bool {
    with_state(|state| {
        state.dragging_system.end_drag();
        true
    })
    .unwrap_or(false)
}

fn set_row_height_impl(unique_id: String, row_height: f64) -> bool {
    let trimmed_id = unique_id.trim();
    let Some(row_height) = f64_to_u32(row_height) else {
        return false;
    };
    if trimmed_id.is_empty() {
        return false;
    }

    with_state(|state| {
        state
            .selected_variables
            .update_row_height(trimmed_id, row_height.clamp(20, 300));
        state
            .app_config
            .update_variable_row_height(trimmed_id, row_height.clamp(20, 300));
        true
    })
    .unwrap_or(false)
}

fn set_variable_format_impl(unique_id: String, format: String) -> bool {
    let trimmed_id = unique_id.trim();
    if trimmed_id.is_empty() {
        return false;
    }

    let Some(format) = parse_var_format(&format) else {
        return false;
    };

    with_state(|state| {
        crate::format_selection::update_variable_format(
            trimmed_id,
            format,
            &state.selected_variables,
            &state.waveform_timeline,
            &state.app_config,
        );
        true
    })
    .unwrap_or(false)
}

fn set_analog_limits_impl(unique_id: String, auto: bool, min: f64, max: f64) -> bool {
    let trimmed_id = unique_id.trim();
    if trimmed_id.is_empty() {
        return false;
    }

    let analog_limits = if auto {
        Some(AnalogLimits::auto())
    } else if min.is_finite() && max.is_finite() && min < max {
        Some(AnalogLimits::manual(min, max))
    } else {
        return false;
    };

    with_state(|state| {
        state
            .selected_variables
            .update_analog_limits(trimmed_id, analog_limits.clone());
        state
            .app_config
            .update_variable_analog_limits(trimmed_id, analog_limits);
        true
    })
    .unwrap_or(false)
}

fn create_group_impl(name: String, member_ids: JsValue) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }
    let Some(member_ids) = js_value_to_string_vec(member_ids) else {
        return false;
    };

    with_state(|state| {
        state
            .selected_variables
            .create_group_from_members(trimmed.to_string(), member_ids);
        state
            .app_config
            .signal_groups_config
            .set(state.selected_variables.signal_groups_as_config());
        state.app_config.request_save();
        true
    })
    .unwrap_or(false)
}

fn rename_group_impl(index: f64, name: String) -> bool {
    let Some(index) = f64_to_usize(index) else {
        return false;
    };
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }

    with_state(|state| {
        state
            .selected_variables
            .rename_group(index, trimmed.to_string());
        state
            .app_config
            .signal_groups_config
            .set(state.selected_variables.signal_groups_as_config());
        state.app_config.request_save();
        true
    })
    .unwrap_or(false)
}

fn toggle_group_collapse_impl(index: f64) -> bool {
    let Some(index) = f64_to_usize(index) else {
        return false;
    };

    with_state(|state| {
        state.selected_variables.toggle_group_collapse(index);
        state
            .app_config
            .signal_groups_config
            .set(state.selected_variables.signal_groups_as_config());
        state.app_config.request_save();
        true
    })
    .unwrap_or(false)
}

fn delete_group_impl(index: f64) -> bool {
    let Some(index) = f64_to_usize(index) else {
        return false;
    };

    with_state(|state| {
        state.selected_variables.ungroup(index);
        state
            .app_config
            .signal_groups_config
            .set(state.selected_variables.signal_groups_as_config());
        state.app_config.request_save();
        true
    })
    .unwrap_or(false)
}

fn f64_to_u32(value: f64) -> Option<u32> {
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > u32::MAX as f64 {
        return None;
    }
    Some(value as u32)
}

fn f64_to_usize(value: f64) -> Option<usize> {
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > usize::MAX as f64 {
        return None;
    }
    Some(value as usize)
}

fn parse_var_format(value: &str) -> Option<shared::VarFormat> {
    match value.trim() {
        "ASCII" => Some(shared::VarFormat::ASCII),
        "Binary" => Some(shared::VarFormat::Binary),
        "BinaryWithGroups" => Some(shared::VarFormat::BinaryWithGroups),
        "Hexadecimal" => Some(shared::VarFormat::Hexadecimal),
        "Octal" => Some(shared::VarFormat::Octal),
        "Signed" => Some(shared::VarFormat::Signed),
        "Unsigned" => Some(shared::VarFormat::Unsigned),
        _ => None,
    }
}

fn js_value_to_string_vec(value: JsValue) -> Option<Vec<String>> {
    if !js_sys::Array::is_array(&value) {
        return None;
    }

    let values = js_sys::Array::from(&value);
    let mut strings = Vec::with_capacity(values.length() as usize);
    for entry in values.iter() {
        strings.push(entry.as_string()?);
    }
    Some(strings)
}
