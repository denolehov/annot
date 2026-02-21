//! Snapshot tests for output formatting using insta.
//!
//! Run `cargo insta review` to interactively accept/reject snapshots.
//! Run `cargo insta test` to run tests and create pending snapshots.
//!
//! These tests validate the complete output format for various scenarios,
//! making it easy to catch unintended format changes.

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use chrono::Utc;
use tempfile::NamedTempFile;

use crate::input::{CliSource, ContentSource, DiffSource, McpSource};
use crate::review::Review;
use crate::state::{
    AnnotationRefSnapshot, Annotation, Bookmark, BookmarkSnapshot, ContentMetadata,
    ContentModel, ContentNode, ExitMode, ExitModeSource, Line, LineOrigin, LineRange,
    LineSemantics, RefSnapshot, SessionType, UserConfig,
};

use super::{format_output, OutputMode};

// ========== Test Helpers ==========

fn make_line(path: &str, number: u32, content: &str) -> Line {
    Line {
        content: content.to_string(),
        html: None,
        origin: LineOrigin::Source {
            path: path.to_string(),
            line: number,
        },
        semantics: LineSemantics::Plain,
    }
}

fn make_lines(path: &str, start: u32, end: u32) -> Vec<Line> {
    (start..=end)
        .map(|n| make_line(path, n, &format!("    line {} content", n)))
        .collect()
}

fn make_review(label: &str, lines: Vec<Line>, annotations: HashMap<LineRange, Annotation>) -> Review {
    let source = ContentSource::Cli(CliSource::File {
        path: PathBuf::from(label),
    });
    let content = ContentModel {
        label: label.to_string(),
        lines,
        source,
        metadata: ContentMetadata::Plain,
        portals: Vec::new(),
    };
    let config = UserConfig::empty();
    let mut review = Review::cli(content, config, "main".to_string());
    if let Some(file) = review.files.values_mut().next() {
        file.annotations = annotations;
    }
    review
}

fn make_review_with_config(
    label: &str,
    lines: Vec<Line>,
    annotations: HashMap<LineRange, Annotation>,
    config: UserConfig,
) -> Review {
    let source = ContentSource::Cli(CliSource::File {
        path: PathBuf::from(label),
    });
    let content = ContentModel {
        label: label.to_string(),
        lines,
        source,
        metadata: ContentMetadata::Plain,
        portals: Vec::new(),
    };
    let mut review = Review::cli(content, config, "main".to_string());
    if let Some(file) = review.files.values_mut().next() {
        file.annotations = annotations;
    }
    review
}

// ========== Empty/Minimal Cases ==========

#[test]
fn empty_review() {
    let review = make_review("test.rs", vec![], HashMap::new());
    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

// ========== Single Annotation ==========

#[test]
fn single_line_annotation() {
    let mut annotations = HashMap::new();
    annotations.insert(
        LineRange::new(5, 5),
        Annotation {
            start_line: 5,
            end_line: 5,
            content: vec![ContentNode::Text {
                text: "Fix this bug".to_string(),
            }],
        },
    );

    let review = make_review("src/lib.rs", make_lines("src/lib.rs", 1, 10), annotations);
    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

#[test]
fn multi_line_annotation() {
    let mut annotations = HashMap::new();
    annotations.insert(
        LineRange::new(10, 15),
        Annotation {
            start_line: 10,
            end_line: 15,
            content: vec![ContentNode::Text {
                text: "This entire block needs refactoring".to_string(),
            }],
        },
    );

    let review = make_review("src/main.rs", make_lines("src/main.rs", 1, 20), annotations);
    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

#[test]
fn annotation_multiline_content() {
    let mut annotations = HashMap::new();
    annotations.insert(
        LineRange::new(5, 5),
        Annotation {
            start_line: 5,
            end_line: 5,
            content: vec![ContentNode::Text {
                text: "First line of feedback\nSecond line continues\nThird line concludes".to_string(),
            }],
        },
    );

    let review = make_review("file.rs", make_lines("file.rs", 1, 10), annotations);
    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

// ========== Multiple Annotations ==========

#[test]
fn multiple_annotations_sorted() {
    let mut annotations = HashMap::new();
    // Insert in reverse order to verify sorting
    annotations.insert(
        LineRange::new(20, 20),
        Annotation {
            start_line: 20,
            end_line: 20,
            content: vec![ContentNode::Text {
                text: "Second annotation".to_string(),
            }],
        },
    );
    annotations.insert(
        LineRange::new(5, 5),
        Annotation {
            start_line: 5,
            end_line: 5,
            content: vec![ContentNode::Text {
                text: "First annotation".to_string(),
            }],
        },
    );

    let review = make_review("test.rs", make_lines("test.rs", 1, 25), annotations);
    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

// ========== Tags ==========

#[test]
fn single_tag() {
    let mut annotations = HashMap::new();
    annotations.insert(
        LineRange::new(5, 5),
        Annotation {
            start_line: 5,
            end_line: 5,
            content: vec![
                ContentNode::Tag {
                    id: "sec001".to_string(),
                    name: "SECURITY".to_string(),
                    instruction: "Review for security vulnerabilities".to_string(),
                },
                ContentNode::Text {
                    text: " Validate user input here".to_string(),
                },
            ],
        },
    );

    let review = make_review("auth.rs", make_lines("auth.rs", 1, 10), annotations);
    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

#[test]
fn multiple_tags_alphabetized() {
    let mut annotations = HashMap::new();
    annotations.insert(
        LineRange::new(5, 5),
        Annotation {
            start_line: 5,
            end_line: 5,
            content: vec![
                ContentNode::Tag {
                    id: "todo001".to_string(),
                    name: "TODO".to_string(),
                    instruction: "Mark items for follow-up".to_string(),
                },
                ContentNode::Tag {
                    id: "bug001".to_string(),
                    name: "BUG".to_string(),
                    instruction: "Known bug to fix".to_string(),
                },
                ContentNode::Tag {
                    id: "sec001".to_string(),
                    name: "SECURITY".to_string(),
                    instruction: "Security concern".to_string(),
                },
            ],
        },
    );

    let review = make_review("code.rs", make_lines("code.rs", 1, 10), annotations);
    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

// ========== GENERAL Block ==========

#[test]
fn general_comment_only() {
    let mut review = make_review("test.rs", vec![], HashMap::new());
    review.session_comment = Some(vec![ContentNode::Text {
        text: "Overall the code looks good, just a few minor issues".to_string(),
    }]);

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

#[test]
fn general_comment_multiline() {
    let mut review = make_review("test.rs", vec![], HashMap::new());
    review.session_comment = Some(vec![ContentNode::Text {
        text: "First paragraph of feedback.\n\nSecond paragraph with more details.\n\nConclusion.".to_string(),
    }]);

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

// ========== NEXT Block (Exit Mode) ==========

#[test]
fn next_apply() {
    let config = UserConfig::with_data(
        vec![],
        vec![ExitMode {
            id: "apply".to_string(),
            name: "Apply".to_string(),
            color: "#22c55e".to_string(),
            instruction: "Proceed with implementation as planned".to_string(),
            order: 0,
            source: ExitModeSource::Persisted,
        }],
    );
    let mut review = make_review_with_config("plan.md", vec![], HashMap::new(), config);
    review.selected_exit_mode_id = Some("apply".to_string());

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

#[test]
fn next_reject() {
    let config = UserConfig::with_data(
        vec![],
        vec![ExitMode {
            id: "reject".to_string(),
            name: "Reject".to_string(),
            color: "#ef4444".to_string(),
            instruction: "Do not proceed, rethink the approach".to_string(),
            order: 0,
            source: ExitModeSource::Persisted,
        }],
    );
    let mut review = make_review_with_config("proposal.md", vec![], HashMap::new(), config);
    review.selected_exit_mode_id = Some("reject".to_string());

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

// ========== Combined Sections ==========

#[test]
fn general_and_next() {
    let config = UserConfig::with_data(
        vec![],
        vec![ExitMode {
            id: "apply".to_string(),
            name: "Apply".to_string(),
            color: "#22c55e".to_string(),
            instruction: "Apply changes".to_string(),
            order: 0,
            source: ExitModeSource::Persisted,
        }],
    );
    let mut review = make_review_with_config("test.rs", vec![], HashMap::new(), config);
    review.session_comment = Some(vec![ContentNode::Text {
        text: "Looks good with minor suggestions below".to_string(),
    }]);
    review.selected_exit_mode_id = Some("apply".to_string());

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

#[test]
fn tags_general_next_annotations() {
    let config = UserConfig::with_data(
        vec![],
        vec![ExitMode {
            id: "revise".to_string(),
            name: "Revise".to_string(),
            color: "#f59e0b".to_string(),
            instruction: "Address the feedback and resubmit".to_string(),
            order: 0,
            source: ExitModeSource::Persisted,
        }],
    );

    let mut annotations = HashMap::new();
    annotations.insert(
        LineRange::new(5, 5),
        Annotation {
            start_line: 5,
            end_line: 5,
            content: vec![
                ContentNode::Tag {
                    id: "sec001".to_string(),
                    name: "SECURITY".to_string(),
                    instruction: "Security review needed".to_string(),
                },
                ContentNode::Text {
                    text: " Sanitize this input".to_string(),
                },
            ],
        },
    );
    annotations.insert(
        LineRange::new(12, 14),
        Annotation {
            start_line: 12,
            end_line: 14,
            content: vec![
                ContentNode::Tag {
                    id: "perf001".to_string(),
                    name: "PERF".to_string(),
                    instruction: "Performance optimization".to_string(),
                },
                ContentNode::Text {
                    text: " Consider caching this".to_string(),
                },
            ],
        },
    );

    let mut review = make_review_with_config("handler.rs", make_lines("handler.rs", 1, 20), annotations, config);
    review.session_comment = Some(vec![ContentNode::Text {
        text: "Good progress, but security and performance need attention".to_string(),
    }]);
    review.selected_exit_mode_id = Some("revise".to_string());

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

// ========== Saved Path ==========

#[test]
fn saved_to_only() {
    let mut review = make_review("test.rs", vec![], HashMap::new());
    review.saved_to = Some(PathBuf::from("/tmp/review-output.md"));

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

#[test]
fn annotations_with_saved_to() {
    let mut annotations = HashMap::new();
    annotations.insert(
        LineRange::new(5, 5),
        Annotation {
            start_line: 5,
            end_line: 5,
            content: vec![ContentNode::Text {
                text: "Note here".to_string(),
            }],
        },
    );

    let mut review = make_review("code.rs", make_lines("code.rs", 1, 10), annotations);
    review.saved_to = Some(PathBuf::from("/home/user/reviews/code-review.md"));

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

// ========== Diff Mode ==========

#[test]
fn diff_annotation_added_line() {
    const DIFF: &str = r#"diff --git a/file.rs b/file.rs
--- a/file.rs
+++ b/file.rs
@@ -1,3 +1,4 @@
 fn main() {
-    old_code();
+    new_code();
+    more_code();
 }
"#;

    let source = ContentSource::Mcp(McpSource::Diff {
        label: Some("test.diff".to_string()),
        source: DiffSource::Raw,
    });
    let content = ContentModel::from_diff(DIFF, source).unwrap();
    let config = UserConfig::empty();
    let mut review = Review::cli(content, config, "main".to_string());

    // Add annotation on added line
    let diff_file_key = crate::review::FileKey::diff_file(0);
    let target = review.files.get_mut(&diff_file_key).unwrap();
    target.upsert_annotation(
        4, // line with more_code()
        4,
        vec![ContentNode::Text {
            text: "Review this new addition".to_string(),
        }],
    );

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

#[test]
fn diff_annotation_deleted_line() {
    const DIFF: &str = r#"diff --git a/file.rs b/file.rs
--- a/file.rs
+++ b/file.rs
@@ -1,3 +1,2 @@
 fn main() {
-    removed_line();
 }
"#;

    let source = ContentSource::Mcp(McpSource::Diff {
        label: Some("deletion.diff".to_string()),
        source: DiffSource::Raw,
    });
    let content = ContentModel::from_diff(DIFF, source).unwrap();
    let config = UserConfig::empty();
    let mut review = Review::cli(content, config, "main".to_string());

    let diff_file_key = crate::review::FileKey::diff_file(0);
    let target = review.files.get_mut(&diff_file_key).unwrap();
    target.upsert_annotation(
        2, // deleted line
        2,
        vec![ContentNode::Text {
            text: "Why was this removed?".to_string(),
        }],
    );

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

// ========== Replace Blocks ==========

#[test]
fn replace_block() {
    let mut annotations = HashMap::new();
    annotations.insert(
        LineRange::new(5, 5),
        Annotation {
            start_line: 5,
            end_line: 5,
            content: vec![ContentNode::Replace {
                original: "let x = dangerous_call(input);".to_string(),
                replacement: "let x = safe_call(sanitize(input));".to_string(),
            }],
        },
    );

    let review = make_review("security.rs", make_lines("security.rs", 1, 10), annotations);
    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

// ========== Error Nodes ==========

#[test]
fn error_node() {
    let mut annotations = HashMap::new();
    annotations.insert(
        LineRange::new(5, 5),
        Annotation {
            start_line: 5,
            end_line: 5,
            content: vec![ContentNode::Error {
                source: "parser".to_string(),
                message: "Failed to parse embedded code block".to_string(),
            }],
        },
    );

    let review = make_review("broken.rs", make_lines("broken.rs", 1, 10), annotations);
    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

// ========== Line Width Variations ==========

#[test]
fn large_line_numbers() {
    let lines: Vec<Line> = (990..=1010)
        .map(|n| make_line("large_file.rs", n, &format!("    content at line {}", n)))
        .collect();

    let mut annotations = HashMap::new();
    annotations.insert(
        LineRange::new(1000, 1005),
        Annotation {
            start_line: 1000,
            end_line: 1005,
            content: vec![ContentNode::Text {
                text: "Wide line numbers should align properly".to_string(),
            }],
        },
    );

    let review = make_review("large_file.rs", lines, annotations);
    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

// ========== Context Line Edge Cases ==========

#[test]
fn annotation_at_line_one() {
    let mut annotations = HashMap::new();
    annotations.insert(
        LineRange::new(1, 1),
        Annotation {
            start_line: 1,
            end_line: 1,
            content: vec![ContentNode::Text {
                text: "No context line above".to_string(),
            }],
        },
    );

    let review = make_review("first.rs", make_lines("first.rs", 1, 5), annotations);
    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

#[test]
fn context_line_whitespace_only() {
    let mut lines = make_lines("whitespace.rs", 1, 5);
    lines[1].content = "   ".to_string(); // Line 2 is whitespace only

    let mut annotations = HashMap::new();
    annotations.insert(
        LineRange::new(3, 3),
        Annotation {
            start_line: 3,
            end_line: 3,
            content: vec![ContentNode::Text {
                text: "Context line 2 should be skipped".to_string(),
            }],
        },
    );

    let review = make_review("whitespace.rs", lines, annotations);
    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

// ========== Kitchen Sink Test ==========

/// Comprehensive test that exercises every output feature:
/// - TAGS section (multiple tags)
/// - BOOKMARKS section (both "this session" and pre-existing)
/// - CONTEXT (would need portals, skipped for simplicity)
/// - GENERAL block (session comment with tags and refs)
/// - NEXT block (exit mode)
/// - Multiple annotations with:
///   - Tags
///   - Bookmark references (BookmarkRef and Ref variants)
///   - Annotation references
///   - File references
///   - Replace blocks
///   - Error nodes
///   - Paste nodes
///   - Excalidraw (CLI mode shows JSON)
///   - Media (figure placeholder)
/// - Saved to path
#[test]
fn kitchen_sink_everything() {
    let config = UserConfig::with_data(
        vec![], // tags loaded from annotations
        vec![ExitMode {
            id: "apply-with-changes".to_string(),
            name: "Apply with Changes".to_string(),
            color: "#22c55e".to_string(),
            instruction: "Apply the code but incorporate the suggested modifications".to_string(),
            order: 0,
            source: ExitModeSource::Persisted,
        }],
    );

    // Create a pre-existing bookmark (not created this session)
    let old_bookmark = Bookmark {
        id: "oldbookmark1".to_string(),
        label: Some("auth-validation".to_string()),
        created_at: Utc::now(),
        project_path: Some(PathBuf::from("/projects/myapp")),
        snapshot: BookmarkSnapshot::Selection {
            source_type: SessionType::File,
            source_title: "src/auth.rs".to_string(),
            context: "fn validate_token(token: &str) -> Result<User, AuthError> {\n    // validation logic\n}".to_string(),
            selected_text: "validate_token".to_string(),
        },
    };

    // Create a bookmark from "this session"
    let new_bookmark = Bookmark {
        id: "newbookmark2".to_string(),
        label: Some("error-handler".to_string()),
        created_at: Utc::now(),
        project_path: None,
        snapshot: BookmarkSnapshot::Session {
            source_type: SessionType::Content,
            source_title: "error-handling-plan.md".to_string(),
            context: "# Error Handling Plan\n\nHandle errors gracefully.".to_string(),
        },
    };

    let mut annotations = HashMap::new();

    // Annotation 1: Tags + text + bookmark ref (legacy format)
    annotations.insert(
        LineRange::new(10, 12),
        Annotation {
            start_line: 10,
            end_line: 12,
            content: vec![
                ContentNode::Tag {
                    id: "sec001".to_string(),
                    name: "SECURITY".to_string(),
                    instruction: "Review for security vulnerabilities".to_string(),
                },
                ContentNode::Text {
                    text: " This authentication logic needs review. See ".to_string(),
                },
                ContentNode::BookmarkRef {
                    id: "oldbookmark1".to_string(),
                    label: "auth-validation".to_string(),
                    bookmark: old_bookmark.clone(),
                },
                ContentNode::Text {
                    text: " for context.".to_string(),
                },
            ],
        },
    );

    // Annotation 2: Replace block
    annotations.insert(
        LineRange::new(25, 25),
        Annotation {
            start_line: 25,
            end_line: 25,
            content: vec![
                ContentNode::Tag {
                    id: "refactor001".to_string(),
                    name: "REFACTOR".to_string(),
                    instruction: "Code improvement suggestion".to_string(),
                },
                ContentNode::Text {
                    text: " ".to_string(),
                },
                ContentNode::Replace {
                    original: "let result = unsafe_operation(input);".to_string(),
                    replacement: "let result = safe_operation(sanitize(input))?;".to_string(),
                },
            ],
        },
    );

    // Annotation 3: Annotation ref + file ref + new-style bookmark ref
    annotations.insert(
        LineRange::new(40, 42),
        Annotation {
            start_line: 40,
            end_line: 42,
            content: vec![
                ContentNode::Tag {
                    id: "todo001".to_string(),
                    name: "TODO".to_string(),
                    instruction: "Action item for follow-up".to_string(),
                },
                ContentNode::Text {
                    text: " Cross-reference: see ".to_string(),
                },
                ContentNode::Ref {
                    ref_type: "annotation".to_string(),
                    snapshot: RefSnapshot::Annotation(AnnotationRefSnapshot {
                        source_key: "10-12".to_string(),
                        source_file: None,
                        preview: "[# SECURITY] This authentication...".to_string(),
                        content: vec![ContentNode::Text {
                            text: "Referenced annotation content".to_string(),
                        }],
                    }),
                },
                ContentNode::Text {
                    text: " and ".to_string(),
                },
                ContentNode::File {
                    path: "src/handlers/api.rs".to_string(),
                },
                ContentNode::Text {
                    text: ". Also related to ".to_string(),
                },
                ContentNode::Ref {
                    ref_type: "bookmark".to_string(),
                    snapshot: RefSnapshot::Bookmark {
                        bookmark: new_bookmark.clone(),
                    },
                },
            ],
        },
    );

    // Annotation 4: Error node + paste
    annotations.insert(
        LineRange::new(55, 55),
        Annotation {
            start_line: 55,
            end_line: 55,
            content: vec![
                ContentNode::Error {
                    source: "mermaid".to_string(),
                    message: "Failed to parse diagram syntax".to_string(),
                },
                ContentNode::Text {
                    text: " Intended diagram:\n".to_string(),
                },
                ContentNode::Paste {
                    content: "graph LR\n    A --> B --> C".to_string(),
                },
            ],
        },
    );

    // Annotation 5: Excalidraw + Media
    annotations.insert(
        LineRange::new(70, 70),
        Annotation {
            start_line: 70,
            end_line: 70,
            content: vec![
                ContentNode::Text {
                    text: "Architecture diagram: ".to_string(),
                },
                ContentNode::Excalidraw {
                    elements: r#"[{"type":"rectangle","x":0,"y":0}]"#.to_string(),
                    image: Some("data:image/png;base64,iVBORw0KGgo=".to_string()),
                },
                ContentNode::Text {
                    text: "\nScreenshot of expected UI: ".to_string(),
                },
                ContentNode::Media {
                    image: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUg==".to_string(),
                    mime_type: "image/png".to_string(),
                },
            ],
        },
    );

    // Build the review
    let mut review = make_review_with_config(
        "src/main.rs",
        make_lines("src/main.rs", 1, 80),
        annotations,
        config,
    );

    // Set session comment with tags and bookmark ref
    review.session_comment = Some(vec![
        ContentNode::Text {
            text: "Overall code review feedback.\n\n".to_string(),
        },
        ContentNode::Tag {
            id: "perf001".to_string(),
            name: "PERF".to_string(),
            instruction: "Performance consideration".to_string(),
        },
        ContentNode::Text {
            text: " Watch for N+1 queries in the data layer.\n\nSee also: ".to_string(),
        },
        ContentNode::BookmarkRef {
            id: "newbookmark2".to_string(),
            label: "error-handler".to_string(),
            bookmark: new_bookmark.clone(),
        },
        ContentNode::Text {
            text: " for error handling patterns.".to_string(),
        },
    ]);

    // Mark new_bookmark as created this session
    review.session_created_bookmarks.insert("newbookmark2".to_string());

    // Set exit mode
    review.selected_exit_mode_id = Some("apply-with-changes".to_string());

    // Set saved path
    review.saved_to = Some(PathBuf::from("/home/user/reviews/main-rs-review.md"));

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

// ========== Command Exit Mode ==========

/// Test command-linked exit mode that includes the skill file content.
/// These are exit modes like "/commit" or "/review-pr" that trigger
/// Claude Code slash commands with embedded instructions.
#[test]
fn command_exit_mode_with_skill_content() {
    // Create a temp file simulating a Claude Code skill/command
    let mut skill_file = NamedTempFile::with_suffix(".md").unwrap();
    writeln!(
        skill_file,
        r#"---
description: "Create a well-structured git commit"
---

# /commit

## Instructions

1. Review all staged changes with `git diff --staged`
2. Write a commit message following conventional commits format
3. The first line should be max 72 characters
4. Include a body explaining the "why" not just the "what"

## Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

## Types

- feat: New feature
- fix: Bug fix
- docs: Documentation only
- refactor: Code change that neither fixes a bug nor adds a feature
"#
    )
    .unwrap();

    let config = UserConfig::with_data(
        vec![],
        vec![ExitMode {
            id: "cmd-commit".to_string(),
            name: "/commit".to_string(),
            color: "#8b5cf6".to_string(),
            instruction: "Create a git commit with the reviewed changes".to_string(),
            order: 0,
            source: ExitModeSource::Command {
                path: skill_file.path().to_path_buf(),
            },
        }],
    );

    let mut annotations = HashMap::new();
    annotations.insert(
        LineRange::new(10, 12),
        Annotation {
            start_line: 10,
            end_line: 12,
            content: vec![
                ContentNode::Tag {
                    id: "commit001".to_string(),
                    name: "COMMIT".to_string(),
                    instruction: "Include in commit message".to_string(),
                },
                ContentNode::Text {
                    text: " This refactors the auth module for clarity".to_string(),
                },
            ],
        },
    );

    let mut review = make_review_with_config(
        "src/auth.rs",
        make_lines("src/auth.rs", 1, 20),
        annotations,
        config,
    );

    review.session_comment = Some(vec![ContentNode::Text {
        text: "Ready to commit. All tests pass.".to_string(),
    }]);

    review.selected_exit_mode_id = Some("cmd-commit".to_string());

    let output = format_output(&review, OutputMode::Cli).text;

    // Replace the temp file path with a stable placeholder for snapshot
    let stable_output = output.replace(
        skill_file.path().to_str().unwrap(),
        "/path/to/skills/commit.md",
    );

    insta::assert_snapshot!(stable_output);
}

// ========== Terraform Regions ==========

#[test]
fn terraform_single_region() {
    use crate::terraform::{FormType, Intensity, MassChange, TerraformIntent, TerraformRegion};

    let source = ContentSource::Cli(CliSource::File {
        path: PathBuf::from("plan.md"),
    });
    let content = ContentModel {
        label: "plan.md".to_string(),
        lines: make_lines("plan.md", 1, 20),
        source,
        metadata: ContentMetadata::Plain,
        portals: Vec::new(),
    };
    let config = UserConfig::empty();
    let mut review = Review::cli(content, config, "main".to_string());

    // Add a terraform region
    if let Some(file) = review.files.values_mut().next() {
        file.terraform_regions.push(TerraformRegion {
            start_line: 5,
            end_line: 8,
            intent: TerraformIntent::Transform {
                form: vec![FormType::Table],
                mass: Some(MassChange::Expand {
                    intensity: Intensity::Moderately,
                }),
                gravity: None,
                direction: None,
            },
        });
    }

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

#[test]
fn terraform_multiple_regions() {
    use crate::terraform::{
        DirectionDirective, FormType, GravityChange, Intensity, MassChange, TerraformIntent,
        TerraformRegion,
    };

    let source = ContentSource::Cli(CliSource::File {
        path: PathBuf::from("content.md"),
    });
    let content = ContentModel {
        label: "content.md".to_string(),
        lines: make_lines("content.md", 1, 30),
        source,
        metadata: ContentMetadata::Plain,
        portals: Vec::new(),
    };
    let config = UserConfig::empty();
    let mut review = Review::cli(content, config, "main".to_string());

    if let Some(file) = review.files.values_mut().next() {
        // First region: expand into table and prose
        file.terraform_regions.push(TerraformRegion {
            start_line: 5,
            end_line: 8,
            intent: TerraformIntent::Transform {
                form: vec![FormType::Table, FormType::Prose],
                mass: Some(MassChange::Expand {
                    intensity: Intensity::Moderately,
                }),
                gravity: Some(GravityChange::Focus {
                    intensity: Intensity::Slightly,
                }),
                direction: None,
            },
        });

        // Second region: condense with direction
        file.terraform_regions.push(TerraformRegion {
            start_line: 15,
            end_line: 18,
            intent: TerraformIntent::Transform {
                form: vec![],
                mass: Some(MassChange::Condense {
                    intensity: Intensity::Significantly,
                }),
                gravity: None,
                direction: Some(DirectionDirective::MoveAway {
                    intensity: Intensity::Moderately,
                }),
            },
        });
    }

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

#[test]
fn terraform_with_annotations() {
    use crate::terraform::{FormType, Intensity, MassChange, TerraformIntent, TerraformRegion};

    let source = ContentSource::Cli(CliSource::File {
        path: PathBuf::from("mixed.md"),
    });
    let content = ContentModel {
        label: "mixed.md".to_string(),
        lines: make_lines("mixed.md", 1, 30),
        source,
        metadata: ContentMetadata::Plain,
        portals: Vec::new(),
    };
    let config = UserConfig::empty();
    let mut review = Review::cli(content, config, "main".to_string());

    if let Some(file) = review.files.values_mut().next() {
        // Terraform region
        file.terraform_regions.push(TerraformRegion {
            start_line: 5,
            end_line: 8,
            intent: TerraformIntent::Transform {
                form: vec![FormType::List],
                mass: Some(MassChange::Expand {
                    intensity: Intensity::Moderately,
                }),
                gravity: None,
                direction: None,
            },
        });

        // Annotation
        file.annotations.insert(
            LineRange::new(15, 16),
            Annotation {
                start_line: 15,
                end_line: 16,
                content: vec![ContentNode::Text {
                    text: "This needs clarification".to_string(),
                }],
            },
        );
    }

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

#[test]
fn terraform_pin_and_dissolve() {
    use crate::terraform::{TerraformIntent, TerraformRegion};

    let source = ContentSource::Cli(CliSource::File {
        path: PathBuf::from("gravity.md"),
    });
    let content = ContentModel {
        label: "gravity.md".to_string(),
        lines: make_lines("gravity.md", 1, 20),
        source,
        metadata: ContentMetadata::Plain,
        portals: Vec::new(),
    };
    let config = UserConfig::empty();
    let mut review = Review::cli(content, config, "main".to_string());

    if let Some(file) = review.files.values_mut().next() {
        // Pin region
        file.terraform_regions.push(TerraformRegion {
            start_line: 3,
            end_line: 4,
            intent: TerraformIntent::Pin,
        });

        // Dissolve region
        file.terraform_regions.push(TerraformRegion {
            start_line: 12,
            end_line: 14,
            intent: TerraformIntent::Dissolve { direction: None },
        });
    }

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

#[test]
fn terraform_reframe() {
    use crate::terraform::{DirectionDirective, TerraformIntent, TerraformRegion};

    let source = ContentSource::Cli(CliSource::File {
        path: PathBuf::from("reframe.md"),
    });
    let content = ContentModel {
        label: "reframe.md".to_string(),
        lines: make_lines("reframe.md", 1, 15),
        source,
        metadata: ContentMetadata::Plain,
        portals: Vec::new(),
    };
    let config = UserConfig::empty();
    let mut review = Review::cli(content, config, "main".to_string());

    if let Some(file) = review.files.values_mut().next() {
        file.terraform_regions.push(TerraformRegion {
            start_line: 5,
            end_line: 8,
            intent: TerraformIntent::Transform {
                form: vec![],
                mass: None,
                gravity: None,
                direction: Some(DirectionDirective::Reframe),
            },
        });
    }

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

#[test]
fn terraform_remove() {
    use crate::terraform::{TerraformIntent, TerraformRegion};

    let source = ContentSource::Cli(CliSource::File {
        path: PathBuf::from("remove.md"),
    });
    let content = ContentModel {
        label: "remove.md".to_string(),
        lines: make_lines("remove.md", 1, 15),
        source,
        metadata: ContentMetadata::Plain,
        portals: Vec::new(),
    };
    let config = UserConfig::empty();
    let mut review = Review::cli(content, config, "main".to_string());

    if let Some(file) = review.files.values_mut().next() {
        file.terraform_regions.push(TerraformRegion {
            start_line: 5,
            end_line: 8,
            intent: TerraformIntent::Remove,
        });
    }

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

/// Comprehensive terraform test that exercises every feature:
/// - Multiple terraform regions with all intents
/// - All form types (single, double, multi-select)
/// - All mass changes (expand, condense)
/// - All gravity changes (focus, blur)
/// - Terminal states (remove, pin, dissolve)
/// - All direction directives (lean-in, move-away, reframe)
/// - All intensity levels
/// - Combined with annotations
/// - Combined with session comment and exit mode
#[test]
fn terraform_kitchen_sink() {
    use crate::terraform::{
        DirectionDirective, FormType, GravityChange, Intensity, MassChange, TerraformIntent,
        TerraformRegion,
    };

    let config = UserConfig::with_data(
        vec![],
        vec![ExitMode {
            id: "iterate".to_string(),
            name: "Iterate".to_string(),
            color: "#8b5cf6".to_string(),
            instruction: "Apply terraform directives and regenerate content".to_string(),
            order: 0,
            source: ExitModeSource::Persisted,
        }],
    );

    let source = ContentSource::Cli(CliSource::File {
        path: PathBuf::from("design-doc.md"),
    });
    let content = ContentModel {
        label: "design-doc.md".to_string(),
        lines: make_lines("design-doc.md", 1, 80),
        source,
        metadata: ContentMetadata::Plain,
        portals: Vec::new(),
    };
    let mut review = Review::cli(content, config, "main".to_string());

    if let Some(file) = review.files.values_mut().next() {
        // Region 1: Single form + expand slightly + focus moderately + lean-in completely
        file.terraform_regions.push(TerraformRegion {
            start_line: 5,
            end_line: 8,
            intent: TerraformIntent::Transform {
                form: vec![FormType::Table],
                mass: Some(MassChange::Expand {
                    intensity: Intensity::Slightly,
                }),
                gravity: Some(GravityChange::Focus {
                    intensity: Intensity::Moderately,
                }),
                direction: Some(DirectionDirective::LeanIn {
                    intensity: Intensity::Significantly,
                }),
            },
        });

        // Region 2: Two forms + condense significantly
        file.terraform_regions.push(TerraformRegion {
            start_line: 15,
            end_line: 18,
            intent: TerraformIntent::Transform {
                form: vec![FormType::List, FormType::Diagram],
                mass: Some(MassChange::Condense {
                    intensity: Intensity::Significantly,
                }),
                gravity: None,
                direction: None,
            },
        });

        // Region 3: Multiple forms + blur a bit + move-away moderately
        file.terraform_regions.push(TerraformRegion {
            start_line: 25,
            end_line: 28,
            intent: TerraformIntent::Transform {
                form: vec![FormType::Prose, FormType::Code, FormType::Table],
                mass: None,
                gravity: Some(GravityChange::Blur {
                    intensity: Intensity::Moderately,
                }),
                direction: Some(DirectionDirective::MoveAway {
                    intensity: Intensity::Moderately,
                }),
            },
        });

        // Region 4: Pin only (preserve exactly)
        file.terraform_regions.push(TerraformRegion {
            start_line: 35,
            end_line: 36,
            intent: TerraformIntent::Pin,
        });

        // Region 5: Dissolve only (integrate into surroundings)
        file.terraform_regions.push(TerraformRegion {
            start_line: 42,
            end_line: 45,
            intent: TerraformIntent::Dissolve { direction: None },
        });

        // Region 6: Remove entirely
        file.terraform_regions.push(TerraformRegion {
            start_line: 52,
            end_line: 55,
            intent: TerraformIntent::Remove,
        });

        // Region 7: Reframe only
        file.terraform_regions.push(TerraformRegion {
            start_line: 62,
            end_line: 65,
            intent: TerraformIntent::Transform {
                form: vec![FormType::Prose],
                mass: None,
                gravity: None,
                direction: Some(DirectionDirective::Reframe),
            },
        });

        // Add some annotations alongside terraform regions
        file.annotations.insert(
            LineRange::new(10, 12),
            Annotation {
                start_line: 10,
                end_line: 12,
                content: vec![
                    ContentNode::Tag {
                        id: "clarity001".to_string(),
                        name: "CLARITY".to_string(),
                        instruction: "Needs clearer explanation".to_string(),
                    },
                    ContentNode::Text {
                        text: " This section is confusing".to_string(),
                    },
                ],
            },
        );

        file.annotations.insert(
            LineRange::new(70, 72),
            Annotation {
                start_line: 70,
                end_line: 72,
                content: vec![ContentNode::Text {
                    text: "Good example, keep this".to_string(),
                }],
            },
        );
    }

    // Set session comment
    review.session_comment = Some(vec![
        ContentNode::Text {
            text: "Overall feedback on the design document.\n\n".to_string(),
        },
        ContentNode::Text {
            text: "The structure needs work - use terraform directives to reshape.".to_string(),
        },
    ]);

    // Set exit mode
    review.selected_exit_mode_id = Some("iterate".to_string());

    let output = format_output(&review, OutputMode::Cli).text;
    insta::assert_snapshot!(output);
}

// ========== JSON Output Tests ==========

#[test]
fn json_output_empty_review() {
    let review = make_review("test.rs", vec![], HashMap::new());
    let result = format_output(&review, OutputMode::Mcp);
    let json_str = super::format_json(&result);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(parsed["text"], "");
    assert_eq!(parsed["images"], serde_json::json!([]));
}

#[test]
fn json_output_text_only() {
    let mut annotations = HashMap::new();
    annotations.insert(
        LineRange::new(5, 5),
        Annotation {
            start_line: 5,
            end_line: 5,
            content: vec![ContentNode::Text {
                text: "Fix this".to_string(),
            }],
        },
    );

    let lines = make_lines("handler.rs", 1, 10);
    let review = make_review("handler.rs", lines, annotations);
    let result = format_output(&review, OutputMode::Mcp);
    let json_str = super::format_json(&result);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Text should contain the annotation
    let text = parsed["text"].as_str().unwrap();
    assert!(text.contains("Fix this"), "JSON text should contain annotation");
    assert!(text.contains("handler.rs:5"), "JSON text should contain file location");

    // No images
    assert_eq!(parsed["images"].as_array().unwrap().len(), 0);
}

#[test]
fn json_output_with_images() {
    use crate::mcp::tools::SessionImage;

    let result = super::FormatResult {
        text: "handler.rs:5\n>   5 | code\n      └──> Has image: [Figure 1]".to_string(),
        images: vec![SessionImage {
            figure: 1,
            data: "iVBORw0KGgoAAAANS".to_string(),
            mime_type: "image/png".to_string(),
        }],
        metadata: super::FormatMetadata::default(),
    };

    let json_str = super::format_json(&result);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Text present
    assert!(parsed["text"].as_str().unwrap().contains("[Figure 1]"));

    // Image present with correct fields
    let images = parsed["images"].as_array().unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0]["figure"], 1);
    assert_eq!(images[0]["data"], "iVBORw0KGgoAAAANS");
    assert_eq!(images[0]["mime_type"], "image/png");
}

#[test]
fn json_output_with_multiple_images() {
    use crate::mcp::tools::SessionImage;

    let result = super::FormatResult {
        text: "[Figure 1] and [Figure 2]".to_string(),
        images: vec![
            SessionImage {
                figure: 1,
                data: "png_data_1".to_string(),
                mime_type: "image/png".to_string(),
            },
            SessionImage {
                figure: 2,
                data: "jpeg_data_2".to_string(),
                mime_type: "image/jpeg".to_string(),
            },
        ],
        metadata: super::FormatMetadata::default(),
    };

    let json_str = super::format_json(&result);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    let images = parsed["images"].as_array().unwrap();
    assert_eq!(images.len(), 2);
    assert_eq!(images[0]["figure"], 1);
    assert_eq!(images[0]["mime_type"], "image/png");
    assert_eq!(images[1]["figure"], 2);
    assert_eq!(images[1]["mime_type"], "image/jpeg");
}

#[test]
fn json_output_is_valid_json() {
    // Test with a realistic review containing special characters
    let mut annotations = HashMap::new();
    annotations.insert(
        LineRange::new(3, 3),
        Annotation {
            start_line: 3,
            end_line: 3,
            content: vec![ContentNode::Text {
                text: "Contains \"quotes\" and\nnewlines and <html>".to_string(),
            }],
        },
    );

    let lines = make_lines("test.rs", 1, 5);
    let review = make_review("test.rs", lines, annotations);
    let result = format_output(&review, OutputMode::Mcp);
    let json_str = super::format_json(&result);

    // Must be valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
    assert!(parsed.is_ok(), "Output must be valid JSON: {}", json_str);

    // Must have both fields
    let value = parsed.unwrap();
    assert!(value.get("text").is_some());
    assert!(value.get("images").is_some());
}

#[test]
fn json_output_mcp_mode_collects_media_as_images() {
    // When a Media node is in an annotation, MCP mode collects it as an image
    let mut annotations = HashMap::new();
    annotations.insert(
        LineRange::new(5, 5),
        Annotation {
            start_line: 5,
            end_line: 5,
            content: vec![
                ContentNode::Text {
                    text: "Screenshot: ".to_string(),
                },
                ContentNode::Media {
                    image: "data:image/png;base64,AAAA".to_string(),
                    mime_type: "image/png".to_string(),
                },
            ],
        },
    );

    let lines = make_lines("app.rs", 1, 10);
    let review = make_review("app.rs", lines, annotations);
    let result = format_output(&review, OutputMode::Mcp);

    // Text should have [Figure N] placeholder
    assert!(result.text.contains("[Figure 1]"));
    // Image should be collected
    assert_eq!(result.images.len(), 1);
    assert_eq!(result.images[0].data, "AAAA");
    assert_eq!(result.images[0].mime_type, "image/png");

    // JSON should include both
    let json_str = super::format_json(&result);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert!(parsed["text"].as_str().unwrap().contains("[Figure 1]"));
    assert_eq!(parsed["images"].as_array().unwrap().len(), 1);
}

