use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Input for the review_file tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReviewFileInput {
    #[schemars(description = "absolute or relative path to the file")]
    pub file_path: String,

    #[schemars(description = "optional exit modes for the review session")]
    pub exit_modes: Option<Vec<ExitModeInput>>,
}

/// Input for the review_content tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReviewContentInput {
    #[schemars(description = "markdown-formatted text content to review")]
    pub content: String,

    #[schemars(description = "display name with .md extension")]
    pub label: String,

    #[schemars(description = "optional exit modes for the review session")]
    pub exit_modes: Option<Vec<ExitModeInput>>,
}

/// Input for the review_diff tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReviewDiffInput {
    #[schemars(description = "git diff arguments (e.g. [\"--staged\"])")]
    pub git_diff_args: Option<Vec<String>>,

    #[schemars(description = "raw unified diff content")]
    pub diff_content: Option<String>,

    #[schemars(description = "display name (default: diff)")]
    pub label: Option<String>,

    #[schemars(description = "optional exit modes for the review session")]
    pub exit_modes: Option<Vec<ExitModeInput>>,
}

/// An exit mode provided by the MCP caller.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExitModeInput {
    #[schemars(description = "display name for the exit mode")]
    pub name: String,

    #[schemars(description = "context/instruction for downstream processing")]
    pub instruction: String,

    #[schemars(description = "color name: green, yellow, red, blue, purple, orange (auto-assigned if empty)")]
    pub color: Option<String>,
}

/// An image collected from annotations.
#[derive(Debug, Clone)]
pub struct SessionImage {
    /// Figure number (1-indexed)
    pub figure: usize,
    /// Base64-encoded image data (without data URL prefix)
    pub data: String,
    /// MIME type (e.g., "image/png")
    pub mime_type: String,
}

/// Output from a review session.
#[derive(Debug, Clone)]
pub struct SessionOutput {
    pub text: String,
    pub images: Vec<SessionImage>,
}

/// Convert ExitModeInput to internal ExitMode format.
impl ExitModeInput {
    /// Map color name to CSS hex color.
    fn color_to_hex(color: &Option<String>, index: usize) -> String {
        let default_colors = ["#22c55e", "#eab308", "#ef4444", "#3b82f6", "#a855f7", "#f97316"];

        match color.as_deref() {
            Some("green") => "#22c55e".to_string(),
            Some("yellow") => "#eab308".to_string(),
            Some("red") => "#ef4444".to_string(),
            Some("blue") => "#3b82f6".to_string(),
            Some("purple") => "#a855f7".to_string(),
            Some("orange") => "#f97316".to_string(),
            _ => default_colors[index % default_colors.len()].to_string(),
        }
    }

    /// Convert to internal ExitMode.
    pub fn to_exit_mode(&self, index: usize) -> crate::state::ExitMode {
        use crate::state::ExitModeOrigin;
        crate::state::ExitMode {
            id: format!("transient-{}", index),
            name: self.name.clone(),
            color: Self::color_to_hex(&self.color, index),
            instruction: self.instruction.clone(),
            order: index as u32,
            origin: ExitModeOrigin::Transient,
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
// BOOKMARK TOOLS
// ════════════════════════════════════════════════════════════════════════════

/// Input for the get_bookmark tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetBookmarkInput {
    #[schemars(description = "Full or prefix ID")]
    pub id: String,
}

/// Input for the list_bookmarks tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListBookmarksInput {
    #[schemars(description = "Maximum number of bookmarks to return")]
    pub limit: Option<usize>,

    #[schemars(description = "Search query to filter by label, selected text, or context")]
    pub search: Option<String>,

    #[schemars(description = "Filter by project path")]
    pub project: Option<String>,
}
