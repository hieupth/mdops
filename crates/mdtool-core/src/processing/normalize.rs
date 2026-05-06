use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct NormalizeOptions {
    pub ensure_single_trailing_newline: bool, // default true
    pub trim_trailing_whitespace: bool,       // default true
    pub max_consecutive_blank_lines: usize,   // default 1
    pub preserve_fenced_code: bool,           // default true
    pub normalize_heading_spacing: bool,      // default true
}

impl Default for NormalizeOptions {
    fn default() -> Self {
        Self {
            ensure_single_trailing_newline: true,
            trim_trailing_whitespace: true,
            max_consecutive_blank_lines: 1,
            preserve_fenced_code: true,
            normalize_heading_spacing: true,
        }
    }
}

/// Normalize markdown text according to the given options.
///
/// Processing order:
/// 1. Line-by-line with fenced-code awareness
///    a. Track fenced code block boundaries
///    b. Normalize heading spacing (blank line before/after headings)
///    c. Collapse consecutive blank lines
///    d. Trim trailing whitespace
/// 2. Ensure single trailing newline
pub fn normalize(text: &str, options: &NormalizeOptions) -> String {
    if text.is_empty() {
        if options.ensure_single_trailing_newline {
            return "\n".to_string();
        }
        return String::new();
    }

    let lines: Vec<&str> = text.lines().collect();
    let mut result_lines: Vec<String> = Vec::with_capacity(lines.len());
    let mut in_fence = false;
    let mut fence_char = '`';
    let mut fence_len = 0usize;
    let mut consecutive_blanks = 0usize;
    let mut prev_was_heading = false;

    for line in lines.iter() {
        // Strip any trailing \r that might remain (CRLF normalization)
        let line = line.strip_suffix('\r').unwrap_or(line);

        let trimmed = line.trim_start();
        let is_fence_opener_or_closer = is_fence_line(trimmed);

        // --- Fenced code block tracking ---
        if is_fence_opener_or_closer {
            if !in_fence {
                // Opening a fence
                in_fence = true;
                fence_char = if trimmed.starts_with('~') { '~' } else { '`' };
                fence_len = count_fence_chars(trimmed);
                flush_blanks(
                    &mut result_lines,
                    &mut consecutive_blanks,
                    options.max_consecutive_blank_lines,
                );
                let processed = if options.trim_trailing_whitespace
                    && !options.preserve_fenced_code
                {
                    trim_trailing(line)
                } else {
                    line.to_string()
                };
                result_lines.push(processed);
                consecutive_blanks = 0;
                prev_was_heading = false;
                continue;
            } else {
                // Potential closer: must match fence_char and have >= fence_len chars
                let close_char = if trimmed.starts_with('~') { '~' } else { '`' };
                let close_len = count_fence_chars(trimmed);
                if close_char == fence_char && close_len >= fence_len {
                    in_fence = false;
                    let processed = if options.trim_trailing_whitespace
                        && !options.preserve_fenced_code
                    {
                        trim_trailing(line)
                    } else {
                        line.to_string()
                    };
                    result_lines.push(processed);
                    consecutive_blanks = 0;
                    prev_was_heading = false;
                    continue;
                }
                // Not a valid closer; treat as content inside fence
            }
        }

        // Inside a fenced code block: pass through unchanged
        if in_fence {
            let processed = if options.preserve_fenced_code {
                line.to_string()
            } else if options.trim_trailing_whitespace {
                trim_trailing(line)
            } else {
                line.to_string()
            };
            let is_blank = processed.trim().is_empty();
            if is_blank {
                consecutive_blanks += 1;
            } else {
                // Non-blank line inside fence: emit pending blanks then the line
                let to_emit = consecutive_blanks.min(options.max_consecutive_blank_lines);
                for _ in 0..to_emit {
                    result_lines.push(String::new());
                }
                consecutive_blanks = 0;
                result_lines.push(processed);
            }
            prev_was_heading = false;
            continue;
        }

        // --- Outside fenced code block ---

        let line_after_trim = if options.trim_trailing_whitespace {
            trim_trailing(line)
        } else {
            line.to_string()
        };
        let is_blank = line_after_trim.is_empty();

        // Determine if this is a heading line
        let is_heading = trimmed.starts_with('#')
            && trimmed
                .find(|c: char| c != '#')
                .map_or(false, |pos| {
                    let after_hashes = &trimmed[pos..];
                    after_hashes.is_empty()
                        || after_hashes.starts_with(' ')
                        || after_hashes.starts_with('\t')
                });

        if is_blank {
            consecutive_blanks += 1;
            continue;
        }

        // Non-blank line outside fenced code
        if is_heading && options.normalize_heading_spacing {
            // Ensure blank line before heading (unless it's the very first output line)
            if !result_lines.is_empty() && consecutive_blanks == 0 {
                result_lines.push(String::new());
            } else {
                flush_blanks(
                    &mut result_lines,
                    &mut consecutive_blanks,
                    1usize.min(options.max_consecutive_blank_lines),
                );
            }
        } else if prev_was_heading && options.normalize_heading_spacing {
            // Previous line was a heading; ensure blank line after it
            if consecutive_blanks == 0 {
                result_lines.push(String::new());
            } else {
                flush_blanks(
                    &mut result_lines,
                    &mut consecutive_blanks,
                    1usize.min(options.max_consecutive_blank_lines),
                );
            }
        } else {
            flush_blanks(
                &mut result_lines,
                &mut consecutive_blanks,
                options.max_consecutive_blank_lines,
            );
        }

        result_lines.push(line_after_trim);
        prev_was_heading = is_heading && options.normalize_heading_spacing;
        consecutive_blanks = 0;
    }

    // Flush any trailing blanks
    flush_blanks(
        &mut result_lines,
        &mut consecutive_blanks,
        options.max_consecutive_blank_lines,
    );

    let mut result = result_lines.join("\n");

    if options.ensure_single_trailing_newline {
        let trimmed_end = result.trim_end_matches('\n');
        result = trimmed_end.to_string() + "\n";
    }

    result
}

/// Emit pending blank lines up to the given maximum.
fn flush_blanks(
    result_lines: &mut Vec<String>,
    consecutive_blanks: &mut usize,
    max: usize,
) {
    let to_emit = (*consecutive_blanks).min(max);
    for _ in 0..to_emit {
        result_lines.push(String::new());
    }
    *consecutive_blanks = 0;
}

/// Trim trailing whitespace (spaces and tabs) from a line.
fn trim_trailing(line: &str) -> String {
    line.trim_end_matches(|c| c == ' ' || c == '\t').to_string()
}

/// Check if a trimmed line is a fence marker (``` or ~~~ with length >= 3).
fn is_fence_line(trimmed: &str) -> bool {
    if trimmed.is_empty() {
        return false;
    }
    let first = trimmed.chars().next().unwrap();
    if first != '`' && first != '~' {
        return false;
    }
    let len = count_fence_chars(trimmed);
    if len < 3 {
        return false;
    }
    // After the fence chars, the rest must be the info string (no backticks for ` fences)
    let rest = &trimmed[len..];
    if first == '`' && rest.contains('`') {
        return false;
    }
    true
}

/// Count the number of consecutive fence characters at the start of the line.
fn count_fence_chars(trimmed: &str) -> usize {
    let first = match trimmed.chars().next() {
        Some(c) if c == '`' || c == '~' => c,
        _ => return 0,
    };
    trimmed.chars().take_while(|&c| c == first).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- normalize_heading_spacing ----

    #[test]
    fn heading_spacing_inserts_blank_before_heading() {
        let input = "some text\n## Heading";
        let opts = NormalizeOptions {
            normalize_heading_spacing: true,
            ..Default::default()
        };
        let result = normalize(input, &opts);
        assert_eq!(result, "some text\n\n## Heading\n");
    }

    #[test]
    fn heading_spacing_inserts_blank_after_heading() {
        let input = "## Heading\nsome text";
        let opts = NormalizeOptions {
            normalize_heading_spacing: true,
            ..Default::default()
        };
        let result = normalize(input, &opts);
        assert_eq!(result, "## Heading\n\nsome text\n");
    }

    #[test]
    fn heading_spacing_no_blank_before_first_line_heading() {
        let input = "# Title\nsome text";
        let opts = NormalizeOptions {
            normalize_heading_spacing: true,
            ..Default::default()
        };
        let result = normalize(input, &opts);
        assert_eq!(result, "# Title\n\nsome text\n");
    }

    #[test]
    fn heading_spacing_disabled() {
        let input = "some text\n## Heading\nmore text";
        let opts = NormalizeOptions {
            normalize_heading_spacing: false,
            ..Default::default()
        };
        let result = normalize(input, &opts);
        assert_eq!(result, "some text\n## Heading\nmore text\n");
    }

    #[test]
    fn heading_spacing_already_has_blank_lines() {
        let input = "some text\n\n## Heading\n\nmore text";
        let opts = NormalizeOptions {
            normalize_heading_spacing: true,
            ..Default::default()
        };
        let result = normalize(input, &opts);
        assert_eq!(result, "some text\n\n## Heading\n\nmore text\n");
    }

    #[test]
    fn heading_spacing_multiple_headings_in_sequence() {
        let input = "# Title\n## Sub\n### SubSub";
        let opts = NormalizeOptions {
            normalize_heading_spacing: true,
            ..Default::default()
        };
        let result = normalize(input, &opts);
        // Headings in sequence: blank line between each
        assert_eq!(result, "# Title\n\n## Sub\n\n### SubSub\n");
    }

    // ---- collapse_blank_lines ----

    #[test]
    fn collapse_blank_lines_basic() {
        let input = "line1\n\n\n\nline2";
        let opts = NormalizeOptions {
            max_consecutive_blank_lines: 1,
            ..all_off()
        };
        let result = normalize(input, &opts);
        assert_eq!(result, "line1\n\nline2\n");
    }

    #[test]
    fn collapse_blank_lines_zero() {
        let input = "line1\n\n\n\nline2";
        let opts = NormalizeOptions {
            max_consecutive_blank_lines: 0,
            ..all_off()
        };
        let result = normalize(input, &opts);
        assert_eq!(result, "line1\nline2\n");
    }

    #[test]
    fn collapse_blank_lines_two_allowed() {
        let input = "line1\n\n\n\nline2";
        let opts = NormalizeOptions {
            max_consecutive_blank_lines: 2,
            ..all_off()
        };
        let result = normalize(input, &opts);
        assert_eq!(result, "line1\n\n\nline2\n");
    }

    // ---- trim_trailing_whitespace ----

    #[test]
    fn trim_trailing_basic() {
        let input = "line1   \nline2\t";
        let opts = NormalizeOptions {
            trim_trailing_whitespace: true,
            ..all_off()
        };
        let result = normalize(input, &opts);
        assert_eq!(result, "line1\nline2\n");
    }

    #[test]
    fn trim_trailing_disabled() {
        let input = "line1   \nline2\t";
        let opts = NormalizeOptions {
            trim_trailing_whitespace: false,
            ..all_off()
        };
        let result = normalize(input, &opts);
        assert_eq!(result, "line1   \nline2\t\n");
    }

    #[test]
    fn trim_trailing_preserves_fenced_code() {
        let input = "```rust\nline with spaces   \n```";
        let opts = NormalizeOptions {
            trim_trailing_whitespace: true,
            preserve_fenced_code: true,
            ..all_off()
        };
        let result = normalize(input, &opts);
        assert_eq!(result, "```rust\nline with spaces   \n```\n");
    }

    #[test]
    fn trim_trailing_inside_fence_when_not_preserving() {
        let input = "```rust\nline with spaces   \n```";
        let opts = NormalizeOptions {
            trim_trailing_whitespace: true,
            preserve_fenced_code: false,
            ..all_off()
        };
        let result = normalize(input, &opts);
        assert_eq!(result, "```rust\nline with spaces\n```\n");
    }

    // ---- ensure_single_trailing_newline ----

    #[test]
    fn trailing_newline_adds() {
        let input = "line1\nline2";
        let opts = NormalizeOptions {
            ensure_single_trailing_newline: true,
            ..all_off()
        };
        let result = normalize(input, &opts);
        assert_eq!(result, "line1\nline2\n");
    }

    #[test]
    fn trailing_newline_removes_extras() {
        let input = "line1\nline2\n\n\n";
        let opts = NormalizeOptions {
            ensure_single_trailing_newline: true,
            ..all_off()
        };
        let result = normalize(input, &opts);
        assert_eq!(result, "line1\nline2\n");
    }

    #[test]
    fn trailing_newline_disabled() {
        let input = "line1\nline2";
        let opts = NormalizeOptions {
            ensure_single_trailing_newline: false,
            ..all_off()
        };
        let result = normalize(input, &opts);
        assert_eq!(result, "line1\nline2");
    }

    // ---- fenced code block handling ----

    #[test]
    fn fenced_code_preserved() {
        let input = "```rust\n### not a heading\n\n\n\nstill code\n```";
        let opts = NormalizeOptions {
            max_consecutive_blank_lines: usize::MAX,
            ..NormalizeOptions::default()
        };
        let result = normalize(input, &opts);
        assert_eq!(
            result,
            "```rust\n### not a heading\n\n\n\nstill code\n```\n"
        );
    }

    #[test]
    fn fenced_code_collapses_blanks() {
        let input = "```rust\n### not a heading\n\n\n\nstill code\n```";
        let opts = NormalizeOptions::default();
        let result = normalize(input, &opts);
        // Default max_consecutive_blank_lines=1 collapses 4 blanks to 1
        assert_eq!(
            result,
            "```rust\n### not a heading\n\nstill code\n```\n"
        );
    }

    #[test]
    fn tilde_fence_preserved() {
        let input = "~~~\ncode here\n~~~";
        let opts = NormalizeOptions::default();
        let result = normalize(input, &opts);
        assert_eq!(result, "~~~\ncode here\n~~~\n");
    }

    #[test]
    fn fence_with_info_string() {
        let input = "```rust,no-run\nlet x = 1;\n```";
        let opts = NormalizeOptions::default();
        let result = normalize(input, &opts);
        assert_eq!(result, "```rust,no-run\nlet x = 1;\n```\n");
    }

    // ---- combined test ----

    #[test]
    fn combined_all_options() {
        let input = "# Title   \n\n\n\n## Section\nSome text   \n```code\n  inside fence   \n```\nMore text";
        let opts = NormalizeOptions::default();
        let result = normalize(input, &opts);
        assert_eq!(
            result,
            "# Title\n\n## Section\n\nSome text\n```code\n  inside fence   \n```\nMore text\n"
        );
    }

    // ---- idempotency ----

    #[test]
    fn idempotent_default_options() {
        let input = "# Title   \n\n\n\n## Section\nSome text   \n\n\n\nMore text";
        let opts = NormalizeOptions::default();
        let first = normalize(input, &opts);
        let second = normalize(&first, &opts);
        assert_eq!(first, second);
    }

    #[test]
    fn idempotent_various_inputs() {
        let inputs = vec![
            "",
            "hello",
            "# Title\n## Section\n",
            "```\ncode\n```\n",
            "line1\n\n\n\n\nline2",
            "line1   \nline2\t\t\n",
            "# H1\n\n## H2\n\n### H3\n",
        ];
        let opts = NormalizeOptions::default();
        for input in inputs {
            let first = normalize(input, &opts);
            let second = normalize(&first, &opts);
            assert_eq!(first, second, "not idempotent for input: {:?}", input);
        }
    }

    // ---- property test: idempotency ----

    #[cfg(test)]
    mod proptests {
        use super::*;

        proptest::proptest! {
            #[test]
            fn normalize_is_idempotent(input in ".*") {
                let opts = NormalizeOptions::default();
                let first = normalize(&input, &opts);
                let second = normalize(&first, &opts);
                assert_eq!(first, second);
            }
        }
    }

    // ---- empty input ----

    #[test]
    fn empty_input_with_trailing_newline() {
        let opts = NormalizeOptions {
            ensure_single_trailing_newline: true,
            ..Default::default()
        };
        let result = normalize("", &opts);
        assert_eq!(result, "\n");
    }

    #[test]
    fn empty_input_without_trailing_newline() {
        let opts = NormalizeOptions {
            ensure_single_trailing_newline: false,
            ..Default::default()
        };
        let result = normalize("", &opts);
        assert_eq!(result, "");
    }

    // ---- helpers ----

    /// Options with all normalizations disabled except trailing newline
    /// (so tests can enable specific features).
    fn all_off() -> NormalizeOptions {
        NormalizeOptions {
            ensure_single_trailing_newline: true,
            trim_trailing_whitespace: false,
            max_consecutive_blank_lines: usize::MAX,
            preserve_fenced_code: true,
            normalize_heading_spacing: false,
        }
    }
}
