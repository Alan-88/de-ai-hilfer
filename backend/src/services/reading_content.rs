use crate::reading_models::BookmarkValidationIssue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedBookmark {
    pub occurrence_index: i32,
    pub surface: String,
    pub form_of: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedPart {
    Text(String),
    Bookmark(ParsedBookmark),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedReadingContent {
    pub parts: Vec<ParsedPart>,
    pub bookmarks: Vec<ParsedBookmark>,
}

pub fn parse_reading_content(
    body: &str,
) -> Result<ParsedReadingContent, Vec<BookmarkValidationIssue>> {
    let mut parts = Vec::new();
    let mut bookmarks = Vec::new();
    let mut cursor = 0;
    let mut text_start = 0;

    while let Some(relative_open) = body[cursor..].find('[') {
        let open = cursor + relative_open;
        let Some(relative_close) = body[open + 1..].find(']') else {
            break;
        };
        let close = open + 1 + relative_close;
        if body.as_bytes().get(close + 1) != Some(&b'(') {
            cursor = open + 1;
            continue;
        }

        let form_start = close + 2;
        let Some(relative_end) = body[form_start..].find(')') else {
            return Err(vec![syntax_issue(
                bookmarks.len() as i32,
                body[open + 1..close].trim(),
                "",
                "书签缺少右括号 )",
            )]);
        };
        let end = form_start + relative_end;
        let surface = body[open + 1..close].trim();
        let form_of = body[form_start..end].trim();

        if surface.is_empty()
            || form_of.is_empty()
            || surface.contains(['[', ']', '\n'])
            || form_of.contains(['(', ')', '\n'])
        {
            return Err(vec![syntax_issue(
                bookmarks.len() as i32,
                surface,
                form_of,
                "书签必须使用 [正文词形](原型)，且两部分均不能为空",
            )]);
        }

        if open > text_start {
            parts.push(ParsedPart::Text(body[text_start..open].to_string()));
        }
        let bookmark = ParsedBookmark {
            occurrence_index: bookmarks.len() as i32,
            surface: surface.to_string(),
            form_of: form_of.to_string(),
        };
        parts.push(ParsedPart::Bookmark(bookmark.clone()));
        bookmarks.push(bookmark);
        cursor = end + 1;
        text_start = cursor;
    }

    if text_start < body.len() {
        parts.push(ParsedPart::Text(body[text_start..].to_string()));
    }

    Ok(ParsedReadingContent { parts, bookmarks })
}

fn syntax_issue(
    occurrence_index: i32,
    surface: &str,
    form_of: &str,
    reason: &str,
) -> BookmarkValidationIssue {
    BookmarkValidationIssue {
        occurrence_index,
        surface: surface.to_string(),
        form_of: form_of.to_string(),
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_reading_content, ParsedPart};

    #[test]
    fn parses_repeated_inline_bookmarks_without_losing_text() {
        let parsed = parse_reading_content(
            "Obwohl eine [Pauschalreise](Pauschalreise) bequem ist, mag ich [Reisen](Reise).",
        )
        .unwrap();

        assert_eq!(parsed.bookmarks.len(), 2);
        assert_eq!(parsed.bookmarks[1].surface, "Reisen");
        assert_eq!(parsed.bookmarks[1].form_of, "Reise");
        assert!(matches!(parsed.parts[1], ParsedPart::Bookmark(_)));
        assert_eq!(parsed.parts.len(), 5);
    }

    #[test]
    fn rejects_incomplete_bookmark_syntax() {
        let issues = parse_reading_content("Das ist [bequem](bequem.").unwrap_err();
        assert_eq!(issues[0].reason, "书签缺少右括号 )");
    }
}
