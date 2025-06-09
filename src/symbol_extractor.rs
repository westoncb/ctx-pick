// src/symbol_extractor.rs

use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};

/// Extracts symbol definition lines from source code.
///
/// This function uses a `tags.scm` query file and filters for captures that
/// start with `@definition.`. It collects the first line of each definition node,
/// sorts them by source position, and removes duplicates to produce a clean
/// "skeleton" of the file.
pub fn extract_symbols(source_code: &str, file_extension: &str) -> Result<String, String> {
    let (language, query_source): (Language, &str) = match file_extension {
        "rs" => (
            tree_sitter_rust::LANGUAGE.into(),
            include_str!("../queries/rust/tags.scm"),
        ),
        "ts" => (
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            include_str!("../queries/typescript/tags.scm"),
        ),
        _ => {
            return Err(format!(
                "Symbol extraction not supported for file extension: '{}'",
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

    let query = Query::new(&language, query_source)
        .map_err(|e| format!("Internal error: Failed to compile tree-sitter query. {}", e))?;

    let mut query_cursor = QueryCursor::new();
    let mut matches = query_cursor.matches(&query, tree.root_node(), source_code.as_bytes());

    let mut captured_lines = Vec::new();

    while let Some(mat) = matches.next() {
        for cap in mat.captures {
            let capture_name = &query.capture_names()[cap.index as usize];

            // The final, simple filter: only care about definition tags.
            if capture_name.starts_with("definition") {
                let node = cap.node;
                let start_byte = node.start_byte();

                if let Some(line_text) = node
                    .utf8_text(source_code.as_bytes())
                    .ok()
                    .and_then(|s| s.lines().next())
                {
                    let trimmed_line = line_text.trim();
                    if !trimmed_line.is_empty() {
                        captured_lines.push((start_byte, trimmed_line.to_string()));
                    }
                }
            }
        }
    }

    if captured_lines.is_empty() {
        return Ok("(No symbols found)".to_string());
    }

    // Sort by source code position.
    captured_lines.sort_by_key(|(byte, _)| *byte);

    // Remove duplicate lines that may result from multiple matching tags.
    captured_lines.dedup_by(|a, b| a.1 == b.1);

    // Format the final output string.
    let result = captured_lines
        .into_iter()
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join("\n");

    Ok(result)
}
