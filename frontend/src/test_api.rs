use crate::selected_variables::SelectedVariables;
use crate::tracked_files::TrackedFiles;
use crate::visualizer::timeline::TimePs;
use crate::visualizer::timeline::timeline_actor::WaveformTimeline;
use shared::AnalogLimits;
use std::cell::RefCell;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use zoon::SendWrapper;

pub struct TestApiState {
    pub tracked_files: TrackedFiles,
    pub selected_variables: SelectedVariables,
    pub waveform_timeline: WaveformTimeline,
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
    app_config: crate::config::AppConfig,
    connection: Arc<SendWrapper<zoon::Connection<shared::UpMsg, shared::DownMsg>>>,
) {
    TEST_API_STATE.with(|cell| {
        *cell.borrow_mut() = Some(TestApiState {
            tracked_files,
            selected_variables,
            waveform_timeline,
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
                        &JsValue::from_f64(variable.row_height.unwrap_or(30) as f64),
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
            .update_analog_limits(trimmed_id, analog_limits);
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
