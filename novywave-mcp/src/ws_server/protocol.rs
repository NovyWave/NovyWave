use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Command {
    Ping,
    GetStatus,
    Screenshot,
    ScreenshotCanvas,
    ScreenshotElement { selector: String },
    GetConsole,
    Refresh,
    Detach,
    Reload,

    PressKey { key: String, shift: bool },
    TypeText { text: String },
    Click { selector: String },
    ClickAt { x: i32, y: i32 },
    ClickText { text: String, exact: bool },
    FindText { text: String, exact: bool },
    GetPageText,

    GetTimelineState,
    GetCursorValues,
    GetSelectedVariables,
    GetLoadedFiles,

    EvaluateJs { script: String },

    SelectWorkspace { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Response {
    Success {
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
    },
    Error { message: String },
    Screenshot { base64: String },
    ScreenshotFile { filepath: String },
    Console { messages: Vec<ConsoleMessage> },
    Pong,
    Status {
        connected: bool,
        #[serde(rename = "pageUrl")]
        page_url: Option<String>,
        #[serde(rename = "appReady")]
        app_ready: bool,
    },
    TimelineState {
        #[serde(rename = "viewportStartPs")]
        viewport_start_ps: Option<u64>,
        #[serde(rename = "viewportEndPs")]
        viewport_end_ps: Option<u64>,
        #[serde(rename = "cursorPs")]
        cursor_ps: Option<u64>,
        #[serde(rename = "zoomCenterPs")]
        zoom_center_ps: Option<u64>,
    },
    CursorValues { values: serde_json::Value },
    SelectedVariables { variables: Vec<VariableInfo> },
    LoadedFiles { files: Vec<FileInfo> },
    JsResult { result: serde_json::Value },
    TextMatches { found: bool, count: u32, matches: Vec<String> },
    PageText { text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleMessage {
    pub level: String,
    pub text: String,
    pub timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableInfo {
    pub unique_id: String,
    pub name: String,
    pub scope_path: Vec<String>,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub path: String,
    pub status: String,
    pub variable_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: u64,
    pub command: Command,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage {
    pub id: u64,
    pub response: Response,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionHello {
    pub client_type: String,
}
