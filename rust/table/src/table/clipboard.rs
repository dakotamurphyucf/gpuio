//! Bounded clipboard encoding. Callers must supply the complete selection.
pub const MAX_COPY_BYTES: usize = 1024 * 1024;

/// Quote TSV fields containing tabs, line breaks or quotes. Returns no partial
/// output when escaping or separators would exceed the byte budget. No trailing
/// newline is added, and all Unicode is preserved byte-for-byte.
pub fn tsv<'a>(
    rows: impl IntoIterator<Item = impl IntoIterator<Item = &'a str>>,
) -> Option<String> {
    let mut output = String::new();
    for (row_index, row) in rows.into_iter().enumerate() {
        if row_index > 0 {
            append(&mut output, "\n")?;
        }
        for (column, text) in row.into_iter().enumerate() {
            if column > 0 {
                append(&mut output, "\t")?;
            }
            if text.contains(['\t', '\r', '\n', '"']) {
                append(&mut output, "\"")?;
                for (index, part) in text.split('"').enumerate() {
                    if index > 0 {
                        append(&mut output, "\"\"")?;
                    }
                    append(&mut output, part)?;
                }
                append(&mut output, "\"")?;
            } else {
                append(&mut output, text)?;
            }
        }
    }
    Some(output)
}
fn append(output: &mut String, text: &str) -> Option<()> {
    if text.len() > MAX_COPY_BYTES - output.len() {
        return None;
    }
    output.push_str(text);
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_empty_and_escaped_fields() {
        assert_eq!(
            tsv([
                ["日本語 👨‍👩‍👧‍👦", "a\tb", ""],
                ["\"quoted\"", "one\r\ntwo", "e\u{301}"]
            ]),
            Some("日本語 👨‍👩‍👧‍👦\t\"a\tb\"\t\n\"\"\"quoted\"\"\"\t\"one\r\ntwo\"\te\u{301}".into())
        );
    }
    #[test]
    fn budget_counts_utf8_escaping_and_separators() {
        let text = "é".repeat(MAX_COPY_BYTES / 2);
        assert_eq!(tsv([[text.as_str()]]).unwrap().len(), MAX_COPY_BYTES);
        assert!(tsv([[text.as_str(), ""]]).is_none());
        assert!(tsv([[text.as_str()], [""]]).is_none());
        let quotes = "\"".repeat(MAX_COPY_BYTES / 2);
        assert!(tsv([[quotes.as_str()]]).is_none());
        let exact = "\"".repeat(MAX_COPY_BYTES / 2 - 1);
        assert_eq!(tsv([[exact.as_str()]]).unwrap().len(), MAX_COPY_BYTES);
    }
}
