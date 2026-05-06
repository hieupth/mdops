use super::model::{AsciiBlock, AsciiEdit, AsciiMode};
use crate::error::{Diagnostic, Severity};

/// Format an ASCII art block according to the given mode.
pub fn format_block(block: &AsciiBlock, mode: AsciiMode) -> AsciiEdit {
    match mode {
        AsciiMode::FormatOnly => format_only(block),
        AsciiMode::RepairSafe => repair_safe(block),
        AsciiMode::RepairAggressive => repair_aggressive(block),
    }
}

/// FormatOnly: normalize tabs, trim trailing spaces, left-normalize indentation.
fn format_only(block: &AsciiBlock) -> AsciiEdit {
    let before = block.lines.clone();
    let after = apply_formatting(&before);
    let changed = before != after;

    AsciiEdit {
        block_id: block.block_id,
        changed,
        before,
        after,
        diagnostics: vec![],
    }
}

/// Apply formatting transformations:
/// 1. Normalize tabs -> 4 spaces
/// 2. Trim trailing spaces per line
/// 3. Left-normalize indentation (find min indent, subtract from all lines)
fn apply_formatting(lines: &[String]) -> Vec<String> {
    if lines.is_empty() {
        return lines.to_vec();
    }

    // Step 1: tabs -> 4 spaces
    let tab_normalized: Vec<String> = lines.iter().map(|l| l.replace('\t', "    ")).collect();

    // Step 2: trim trailing spaces
    let trailing_trimmed: Vec<String> = tab_normalized.iter().map(|l| l.trim_end().to_string()).collect();

    // Step 3: left-normalize indentation
    let min_indent = compute_min_indent(&trailing_trimmed);
    if min_indent == 0 {
        return trailing_trimmed;
    }

    trailing_trimmed
        .iter()
        .map(|l| {
            if l.is_empty() {
                l.clone()
            } else {
                l.chars().skip(min_indent).collect()
            }
        })
        .collect()
}

/// Compute minimum indentation across all non-empty lines.
fn compute_min_indent(lines: &[String]) -> usize {
    let mut min_indent = usize::MAX;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let leading = line.chars().take_while(|&c| c == ' ').count();
        if leading < min_indent {
            min_indent = leading;
        }
    }
    if min_indent == usize::MAX { 0 } else { min_indent }
}

/// Check if a line looks like a box border (e.g., "+---+", "+==+", "+---+---+").
fn is_box_border(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 3 {
        return false;
    }
    // Must start with '+' and end with '+'
    if !trimmed.starts_with('+') || !trimmed.ends_with('+') {
        return false;
    }
    // Inner content must be only '-', '=', or '+'
    let inner: String = trimmed[1..trimmed.len() - 1].to_string();
    if inner.is_empty() {
        return false;
    }
    inner.chars().all(|c| c == '-' || c == '=' || c == '+')
}

/// RepairSafe: FormatOnly + align box borders if confident.
fn repair_safe(block: &AsciiBlock) -> AsciiEdit {
    let before = block.lines.clone();
    let formatted = apply_formatting(&before);

    // Detect box borders
    let border_count = formatted.iter().filter(|l| is_box_border(l)).count();

    // Confidence check: at least 50% of non-empty lines should look like borders
    // OR at least 2 border lines if there are few lines
    let non_empty_count = formatted.iter().filter(|l| !l.is_empty()).count().max(1);
    let border_ratio = border_count as f64 / non_empty_count as f64;

    if border_ratio < 0.5 || border_count < 2 {
        // Low confidence - return formatted only with a warning
        let changed = before != formatted;
        let mut diagnostics = Vec::new();
        if border_count > 0 {
            diagnostics.push(Diagnostic {
                code: "ASCII_LOW_CONFIDENCE".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "Box border detection ambiguous ({}/{} non-empty lines are borders). Skipping repair.",
                    border_count, non_empty_count
                ),
                line: None,
                column: None,
                line_range: None,
                suggested_fix: None,
            });
        }
        return AsciiEdit {
            block_id: block.block_id,
            changed,
            before,
            after: formatted,
            diagnostics,
        };
    }

    // High confidence: align right edges
    let max_width = formatted.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let aligned: Vec<String> = formatted
        .iter()
        .map(|l| {
            let current_len = l.chars().count();
            if current_len < max_width {
                // Pad with spaces, but for border lines, extend the border pattern
                if is_box_border(l) {
                    let padding_needed = max_width - current_len;
                    let trimmed = l.trim();
                    let inner: String = trimmed[1..trimmed.len() - 1].to_string();
                    // Determine the fill character
                    let fill_char = if inner.contains('=') { '=' } else { '-' };
                    let new_inner_len = inner.chars().count() + padding_needed;
                    let new_inner: String = (0..new_inner_len).map(|_| fill_char).collect();
                    format!("+{}+", new_inner)
                } else {
                    let padding = " ".repeat(max_width - current_len);
                    format!("{}{}", l, padding)
                }
            } else {
                l.clone()
            }
        })
        .collect();

    let changed = before != aligned;
    AsciiEdit {
        block_id: block.block_id,
        changed,
        before,
        after: aligned,
        diagnostics: vec![],
    }
}

/// RepairAggressive: stub - just do FormatOnly and emit info diagnostic.
fn repair_aggressive(block: &AsciiBlock) -> AsciiEdit {
    let before = block.lines.clone();
    let after = apply_formatting(&before);
    let changed = before != after;

    let diagnostic = Diagnostic {
        code: "ASCII_AGGRESSIVE_NOT_IMPLEMENTED".to_string(),
        severity: Severity::Info,
        message: "RepairAggressive mode is not yet implemented. Falling back to FormatOnly.".to_string(),
        line: None,
        column: None,
        line_range: None,
        suggested_fix: None,
    };

    AsciiEdit {
        block_id: block.block_id,
        changed,
        before,
        after,
        diagnostics: vec![diagnostic],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::BlockId;

    fn make_block(id: u32, lines: Vec<&str>) -> AsciiBlock {
        AsciiBlock {
            block_id: BlockId(id),
            info_string: "ascii".to_string(),
            indent: 0,
            lines: lines.iter().map(|s| s.to_string()).collect(),
        }
    }

    // --- FormatOnly tests ---

    #[test]
    fn format_only_tabs_to_spaces() {
        let block = make_block(1, vec!["\thello", "\t\tworld"]);
        let edit = format_block(&block, AsciiMode::FormatOnly);
        assert!(edit.changed);
        assert_eq!(edit.after[0], "hello");
        assert_eq!(edit.after[1], "    world");
    }

    #[test]
    fn format_only_trailing_trim() {
        let block = make_block(1, vec!["hello   ", "world  "]);
        let edit = format_block(&block, AsciiMode::FormatOnly);
        assert!(edit.changed);
        assert_eq!(edit.after[0], "hello");
        assert_eq!(edit.after[1], "world");
    }

    #[test]
    fn format_only_indent_normalization() {
        let block = make_block(1, vec!["    line1", "        line2", "    line3"]);
        let edit = format_block(&block, AsciiMode::FormatOnly);
        assert!(edit.changed);
        assert_eq!(edit.after[0], "line1");
        assert_eq!(edit.after[1], "    line2");
        assert_eq!(edit.after[2], "line3");
    }

    #[test]
    fn format_only_unchanged() {
        let block = make_block(1, vec!["hello", "world"]);
        let edit = format_block(&block, AsciiMode::FormatOnly);
        assert!(!edit.changed);
    }

    #[test]
    fn format_only_idempotent() {
        let block = make_block(1, vec!["\thello   ", "  \t  world"]);
        let edit1 = format_block(&block, AsciiMode::FormatOnly);

        // Apply again on the result
        let block2 = AsciiBlock {
            block_id: block.block_id,
            info_string: block.info_string.clone(),
            indent: 0,
            lines: edit1.after.clone(),
        };
        let edit2 = format_block(&block2, AsciiMode::FormatOnly);
        assert!(!edit2.changed, "Second application should not change anything");
        assert_eq!(edit1.after, edit2.after);
    }

    // --- RepairSafe tests ---

    #[test]
    fn repair_safe_aligns_simple_box() {
        let block = make_block(1, vec![
            "+---+",
            "| a |",
            "+---+",
            "| b   |",
            "+-----+",
        ]);
        let edit = format_block(&block, AsciiMode::RepairSafe);
        assert!(edit.changed);
        // All lines should have the same width
        let widths: Vec<usize> = edit.after.iter().map(|l| l.chars().count()).collect();
        let max_w = *widths.iter().max().unwrap();
        for (i, w) in widths.iter().enumerate() {
            assert_eq!(*w, max_w, "Line {} has width {} but expected {}", i, w, max_w);
        }
    }

    #[test]
    fn repair_safe_low_confidence_warning() {
        // Only 1 border line out of many - low confidence
        let block = make_block(1, vec![
            "+---+",
            "some random text",
            "more text here",
            "even more text",
            "and more",
        ]);
        let edit = format_block(&block, AsciiMode::RepairSafe);
        // Should emit a warning diagnostic
        assert!(edit.diagnostics.iter().any(|d| d.code == "ASCII_LOW_CONFIDENCE"),
            "Expected ASCII_LOW_CONFIDENCE warning");
    }

    #[test]
    fn repair_safe_no_borders_no_warning() {
        let block = make_block(1, vec!["hello", "world"]);
        let edit = format_block(&block, AsciiMode::RepairSafe);
        // No border-related warnings expected
        assert!(!edit.diagnostics.iter().any(|d| d.code == "ASCII_LOW_CONFIDENCE"));
    }

    // --- RepairAggressive tests ---

    #[test]
    fn repair_aggressive_stub() {
        let block = make_block(1, vec!["  hello  "]);
        let edit = format_block(&block, AsciiMode::RepairAggressive);
        // Should have the info diagnostic about not being implemented
        assert!(edit.diagnostics.iter().any(|d| d.code == "ASCII_AGGRESSIVE_NOT_IMPLEMENTED"),
            "Expected ASCII_AGGRESSIVE_NOT_IMPLEMENTED info diagnostic");
        // Should still do FormatOnly operations
        assert!(edit.changed);
        assert_eq!(edit.after[0], "hello");
    }

    #[test]
    fn repair_aggressive_unchanged() {
        let block = make_block(1, vec!["hello", "world"]);
        let edit = format_block(&block, AsciiMode::RepairAggressive);
        assert!(!edit.changed);
        assert_eq!(edit.diagnostics.len(), 1);
        assert_eq!(edit.diagnostics[0].code, "ASCII_AGGRESSIVE_NOT_IMPLEMENTED");
    }

    // --- Box border detection tests ---

    #[test]
    fn is_box_border_valid() {
        assert!(is_box_border("+---+"));
        assert!(is_box_border("+===+"));
        assert!(is_box_border("+---+---+"));
        assert!(is_box_border("+=====+"));
        assert!(is_box_border("  +---+  ")); // with surrounding whitespace
    }

    #[test]
    fn is_box_border_invalid() {
        assert!(!is_box_border(""));
        assert!(!is_box_border("++not a border"));
        assert!(!is_box_border("| text |"));
        assert!(!is_box_border("hello"));
        assert!(!is_box_border("+"));
        assert!(!is_box_border("++"));
    }
}
