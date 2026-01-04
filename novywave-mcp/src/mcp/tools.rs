use serde_json::{json, Value};

pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

pub fn get_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "novywave_status".into(),
            description: "Check browser extension connection and NovyWave app readiness.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "novywave_screenshot".into(),
            description: "Take a full page screenshot. Saves PNG to /tmp/novywave-screenshots/.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "novywave_screenshot_canvas".into(),
            description: "Take a screenshot of the waveform canvas only.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "novywave_console".into(),
            description: "Get browser console logs with optional filtering.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max messages to return (default: 100)" },
                    "level": { "type": "string", "description": "Filter: 'error', 'warn', 'log', 'all'" },
                    "pattern": { "type": "string", "description": "Text pattern to filter" }
                },
                "required": []
            }),
        },
        Tool {
            name: "novywave_refresh".into(),
            description: "Refresh the page without disconnecting extension.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "novywave_timeline_zoom_in".into(),
            description: "Zoom into the timeline (W key). Use faster=true for Shift+W.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "faster": { "type": "boolean", "description": "Use Shift modifier for faster zoom" }
                },
                "required": []
            }),
        },
        Tool {
            name: "novywave_timeline_zoom_out".into(),
            description: "Zoom out of the timeline (S key). Use faster=true for Shift+S.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "faster": { "type": "boolean", "description": "Use Shift modifier for faster zoom" }
                },
                "required": []
            }),
        },
        Tool {
            name: "novywave_timeline_pan_left".into(),
            description: "Pan timeline left (A key). Use faster=true for Shift+A.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "faster": { "type": "boolean", "description": "Use Shift modifier for faster pan" }
                },
                "required": []
            }),
        },
        Tool {
            name: "novywave_timeline_pan_right".into(),
            description: "Pan timeline right (D key). Use faster=true for Shift+D.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "faster": { "type": "boolean", "description": "Use Shift modifier for faster pan" }
                },
                "required": []
            }),
        },
        Tool {
            name: "novywave_timeline_reset".into(),
            description: "Reset timeline zoom to show all (R key).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "novywave_cursor_left".into(),
            description: "Move cursor left (Q key). Use faster=true for Shift+Q.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "faster": { "type": "boolean", "description": "Use Shift modifier" }
                },
                "required": []
            }),
        },
        Tool {
            name: "novywave_cursor_right".into(),
            description: "Move cursor right (E key). Use faster=true for Shift+E.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "faster": { "type": "boolean", "description": "Use Shift modifier" }
                },
                "required": []
            }),
        },
        Tool {
            name: "novywave_get_timeline_state".into(),
            description: "Get current timeline state: viewport range, cursor position, zoom center.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "novywave_get_cursor_values".into(),
            description: "Get signal values at the current cursor position.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "novywave_get_selected_variables".into(),
            description: "Get list of currently selected variables.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "novywave_get_loaded_files".into(),
            description: "Get list of loaded waveform files and their status.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "novywave_click_text".into(),
            description: "Click an element by its text content.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to find and click" },
                    "exact": { "type": "boolean", "description": "Match exact text (default: false)" }
                },
                "required": ["text"]
            }),
        },
        Tool {
            name: "novywave_find_text".into(),
            description: "Search for text on the page without clicking. Returns matches found.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to search for" },
                    "exact": { "type": "boolean", "description": "Match exact text (default: false)" }
                },
                "required": ["text"]
            }),
        },
        Tool {
            name: "novywave_get_page_text".into(),
            description: "Get all visible text content from the page.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "novywave_type_text".into(),
            description: "Type text into the currently focused element.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to type" }
                },
                "required": ["text"]
            }),
        },
        Tool {
            name: "novywave_press_key".into(),
            description: "Press a keyboard key (Enter, Escape, Tab, etc.).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Key to press" }
                },
                "required": ["key"]
            }),
        },
        Tool {
            name: "novywave_launch_browser".into(),
            description: "Launch Chromium with NovyWave extension. Opens localhost:8080.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "headless": { "type": "boolean", "description": "Run headless (default: false)" }
                },
                "required": []
            }),
        },
        Tool {
            name: "novywave_detach".into(),
            description: "Detach CDP debugger. Use when 'debugger already attached' errors occur.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "novywave_verify".into(),
            description: "Run workspace verification tests. Tests: no stuck 'Loading workspace...', files restored, variables restored.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "workspace": { "type": "string", "description": "Path to workspace with .novywave config" },
                    "timeout": { "type": "integer", "description": "Timeout in ms (default: 15000)" }
                },
                "required": ["workspace"]
            }),
        },
    ]
}
