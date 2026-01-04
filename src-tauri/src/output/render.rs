//! Content node rendering to plain text.

use crate::mcp::tools::SessionImage;
use crate::state::{ContentNode, RefSnapshot};

use super::OutputMode;

/// Render content nodes to plain text, collecting images with figure numbers.
pub fn render_content(
    nodes: &[ContentNode],
    images: &mut Vec<SessionImage>,
    figure_counter: &mut usize,
    mode: OutputMode,
) -> String {
    nodes
        .iter()
        .map(|node| render_node(node, images, figure_counter, mode))
        .collect::<Vec<_>>()
        .join("")
}

fn render_node(
    node: &ContentNode,
    images: &mut Vec<SessionImage>,
    figure_counter: &mut usize,
    mode: OutputMode,
) -> String {
    match node {
        ContentNode::Text { text } => text.clone(),
        ContentNode::Tag { name, .. } => format!("[# {}]", name),
        ContentNode::Media { image, mime_type } => {
            *figure_counter += 1;
            let figure_num = *figure_counter;

            // Extract base64 data from data URL (strip "data:image/png;base64," prefix)
            let data = if let Some(idx) = image.find(',') {
                image[idx + 1..].to_string()
            } else {
                image.clone()
            };

            images.push(SessionImage {
                figure: figure_num,
                data,
                mime_type: mime_type.clone(),
            });

            format!("[Figure {}]", figure_num)
        }
        ContentNode::Excalidraw { elements, image } => {
            *figure_counter += 1;
            let figure_num = *figure_counter;

            // If PNG is available, include it for MCP
            if let Some(ref png_data) = image {
                let data = if let Some(idx) = png_data.find(',') {
                    png_data[idx + 1..].to_string()
                } else {
                    png_data.clone()
                };
                images.push(SessionImage {
                    figure: figure_num,
                    data,
                    mime_type: "image/png".to_string(),
                });
            }

            match mode {
                // CLI: include JSON so diagram data is preserved in stdout
                OutputMode::Cli => format!("[EXCALIDRAW Figure {}]\n{}", figure_num, elements),
                // MCP/Clipboard: just figure reference, no JSON blob
                OutputMode::Mcp | OutputMode::Clipboard => {
                    format!("[EXCALIDRAW Figure {}]", figure_num)
                }
            }
        }
        ContentNode::Replace {
            original,
            replacement,
        } => {
            // Format as a diff block
            let mut diff = String::from("[REPLACE]\n```diff\n");
            for line in original.lines() {
                diff.push_str(&format!("- {}\n", line));
            }
            for line in replacement.lines() {
                diff.push_str(&format!("+ {}\n", line));
            }
            diff.push_str("```");
            diff
        }
        ContentNode::Error { source, message } => {
            format!("[ERROR:{}] {}", source, message)
        }
        ContentNode::Paste { content } => {
            // Output pasted content as plain text
            content.clone()
        }
        ContentNode::BookmarkRef { id, .. } => {
            let short_id = &id[..id.len().min(3)];
            format!("[BOOKMARK {}]", short_id)
        }
        ContentNode::Ref { snapshot, .. } => {
            match snapshot {
                RefSnapshot::Annotation(snap) => {
                    format!("[ANNOTATION L{}]", snap.source_key)
                }
                RefSnapshot::Bookmark { bookmark } => {
                    let short_id = &bookmark.id[..bookmark.id.len().min(3)];
                    format!("[BOOKMARK {}]", short_id)
                }
                RefSnapshot::Heading(snap) => {
                    format!("[H{} {}]", snap.level, snap.title)
                }
            }
        }
        ContentNode::File { path } => {
            // File reference format: @ref:file:path/to/file.ts
            format!("@ref:file:{}", path)
        }
    }
}
