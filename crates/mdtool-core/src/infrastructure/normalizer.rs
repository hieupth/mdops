/// Normalize line endings: CRLF -> LF.
pub fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n")
}

/// Strip UTF-8 BOM prefix (3 bytes: 0xEF, 0xBB, 0xBF).
pub fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

/// Compute byte offsets of the start of each line in the text.
/// Returns a vector where index i gives the byte offset of line i+1.
/// Always starts with 0 (start of first line).
pub fn compute_line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, ch) in text.char_indices() {
        if ch == '\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// Pre-parse normalization: strip BOM, normalize line endings.
pub fn preprocess(text: &str) -> String {
    let text = strip_bom(text);
    normalize_line_endings(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_crlf() {
        assert_eq!(normalize_line_endings("a\r\nb\r\n"), "a\nb\n");
    }

    #[test]
    fn normalize_no_crlf() {
        assert_eq!(normalize_line_endings("a\nb\n"), "a\nb\n");
    }

    #[test]
    fn strip_bom_present() {
        let with_bom = "\u{feff}hello";
        assert_eq!(strip_bom(with_bom), "hello");
    }

    #[test]
    fn strip_bom_absent() {
        assert_eq!(strip_bom("hello"), "hello");
    }

    #[test]
    fn line_offsets_basic() {
        let offsets = compute_line_starts("a\nb\nc");
        assert_eq!(offsets, vec![0, 2, 4]);
    }

    #[test]
    fn line_offsets_empty() {
        let offsets = compute_line_starts("");
        assert_eq!(offsets, vec![0]);
    }

    #[test]
    fn line_offsets_trailing_newline() {
        let offsets = compute_line_starts("a\nb\n");
        assert_eq!(offsets, vec![0, 2, 4]);
    }

    #[test]
    fn preprocess_combined() {
        let input = "\u{feff}a\r\nb\r\n";
        assert_eq!(preprocess(input), "a\nb\n");
    }
}
