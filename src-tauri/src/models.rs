use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    Image,
    Archive,
    Pdf,
    Document,
    Spreadsheet,
    Presentation,
    Audio,
    Video,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActionCategory {
    Format,
    Transform,
    Optimize,
    Archive,
    Utility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ActionParameterType {
    Quality { min: u8, max: u8, default: u8 },
    Scale { presets: Vec<u32>, default: u32 },
    Dimensions { default_width: u32, default_height: u32 },
    BackgroundColor { default: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionDefinition {
    pub id: String,
    pub label: String,
    pub detail: String,
    pub symbol: String,
    pub category: ActionCategory,
    pub engine: String,
    pub input_formats: Vec<String>,
    pub output_format: Option<String>,
    pub available: bool,
    pub limitations: Option<String>,
    pub supports_batch: bool,
    pub supports_progress: bool,
    pub output_naming_rule: String,
    pub parameter: Option<ActionParameterType>,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    pub path: String,
    pub name: String,
    pub extension: String,
    pub kind: FileKind,
    pub mime: String,
    pub size: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format_name: Option<String>,
    pub has_alpha: Option<bool>,
    pub actions: Vec<ActionDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionExecutionRequest {
    pub path: String,
    pub action_id: String,
    pub quality: Option<u8>,
    pub scale_percent: Option<u32>,
    pub target_width: Option<u32>,
    pub target_height: Option<u32>,
    pub bg_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationResult {
    pub output_paths: Vec<String>,
    pub message: String,
    pub details: Option<String>,
}
