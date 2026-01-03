use std::path::Path;

use syntect::dumps::dump_to_uncompressed_file;
use syntect::parsing::{SyntaxDefinition, SyntaxSet};

/// Embedded mermaid grammar for syntax highlighting mermaid code blocks.
const MERMAID_GRAMMAR: &str = include_str!("grammars/mermaid.sublime-syntax");

fn main() {
    // Generate pre-compiled SyntaxSet for fast startup
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let syntax_dump_path = Path::new(&out_dir).join("syntaxes.packdump");

    // Only regenerate if needed (grammar file changed)
    println!("cargo:rerun-if-changed=grammars/mermaid.sublime-syntax");

    // Build syntax set with defaults + custom mermaid grammar
    let mut builder = SyntaxSet::load_defaults_newlines().into_builder();

    if let Ok(mermaid_syntax) = SyntaxDefinition::load_from_str(MERMAID_GRAMMAR, true, None) {
        builder.add(mermaid_syntax);
    }

    let syntax_set = builder.build();

    // Dump to file for include_bytes! at runtime
    dump_to_uncompressed_file(&syntax_set, &syntax_dump_path)
        .expect("Failed to dump syntax set");

    // Standard Tauri build
    tauri_build::build()
}
