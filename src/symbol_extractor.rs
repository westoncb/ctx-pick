// src/symbol_extractor.rs

use tree_sitter::{Language, Node, Parser};

/// Creates a code "skeleton" by walking the CST up to a specified depth.
///
/// This function walks the Concrete Syntax Tree of the source code down to the
/// `max_depth`. It collects the text of all terminal nodes (leaves) it finds
/// within that depth, and then joins them with spaces to create a flattened,
/// high-level representation of the code's structure.
pub fn create_skeleton_by_depth(
    source_code: &str,
    file_extension: &str,
    max_depth: usize,
) -> Result<String, String> {
    // --- Language loading ---
    let language: Language = match file_extension {
        "rs" => tree_sitter_rust::LANGUAGE.into(),
        "py" => tree_sitter_python::LANGUAGE.into(),
        "ts" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        _ => {
            return Err(format!(
                "Language support not configured for file extension: '{}'",
                file_extension
            ));
        }
    };

    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .map_err(|e| format!("Error setting language: {}", e))?;

    let tree = parser
        .parse(source_code, None)
        .ok_or("Internal error: Failed to parse source code.")?;

    // --- Core Logic: Depth-Limited Walk ---

    let mut tokens: Vec<String> = Vec::new();
    let root_node = tree.root_node();

    // Start the recursive walk from the root node (depth 0).
    collect_tokens_at_depth(
        root_node,
        0, // current_depth
        max_depth + 1,
        &mut tokens,
        source_code.as_bytes(),
    );

    if tokens.is_empty() {
        return Ok("(No structure found)".to_string());
    }

    // Join the collected tokens with a space (likely breaks syntactic validity; should be fine for LLMs)
    Ok(tokens.join(" "))
}

/// A recursive helper function to walk the tree to a max depth.
fn collect_tokens_at_depth(
    node: Node,
    current_depth: usize,
    max_depth: usize,
    tokens: &mut Vec<String>,
    source_bytes: &[u8],
) {
    // Base Case: If we've exceeded the max depth, stop recursing.
    if current_depth > max_depth {
        return;
    }

    // If a node is a "leaf" (has no children), it's a terminal token.
    // We capture its text.
    if node.child_count() == 0 {
        if let Ok(text) = node.utf8_text(source_bytes) {
            let trimmed_text = text.trim();
            if !trimmed_text.is_empty() {
                tokens.push(trimmed_text.to_string());
            }
        }
        return; // No children to recurse into.
    }

    // If the node is not a leaf, recurse into its children.
    // We use a TreeCursor for an efficient walk.
    let mut cursor = node.walk();
    for child_node in node.children(&mut cursor) {
        collect_tokens_at_depth(
            child_node,
            current_depth + 1, // Increment depth for the next level
            max_depth,
            tokens,
            source_bytes,
        );
    }
}
