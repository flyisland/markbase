use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanContext {
    MarkdownBody,
    FrontmatterString,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    WikiLink,
    Embed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTarget {
    pub normalized_target: String,
    pub target_text: String,
    pub anchor: Option<String>,
    pub alias_or_size: Option<String>,
    pub is_markdown_note: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkToken {
    pub kind: LinkKind,
    pub full_span: Range<usize>,
    pub inner_span: Range<usize>,
    pub raw_inner: String,
    pub parsed: ParsedTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FenceState {
    marker: u8,
    count: usize,
}

pub fn scan_link_tokens(input: &str, context: ScanContext) -> Vec<LinkToken> {
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut in_inline_code: Option<usize> = None;
    let mut in_fence: Option<FenceState> = None;
    let mut line_start = true;

    while i < bytes.len() {
        if let Some(fence) = in_fence {
            if line_start
                && let Some((marker, count, after)) = detect_fence_marker(bytes, i)
                && marker == fence.marker
                && count >= fence.count
            {
                in_fence = None;
                i = skip_to_line_end(bytes, after);
                line_start = true;
                continue;
            }

            line_start = bytes[i] == b'\n';
            i += 1;
            continue;
        }

        if context == ScanContext::MarkdownBody
            && line_start
            && let Some((marker, count, after)) = detect_fence_marker(bytes, i)
        {
            in_fence = Some(FenceState { marker, count });
            i = skip_to_line_end(bytes, after);
            line_start = true;
            continue;
        }

        if context == ScanContext::MarkdownBody && bytes[i] == b'`' {
            let tick_count = count_run(bytes, i, b'`');
            match in_inline_code {
                Some(open_count) if open_count == tick_count => {
                    in_inline_code = None;
                }
                None => {
                    in_inline_code = Some(tick_count);
                }
                _ => {}
            }
            i += tick_count;
            line_start = false;
            continue;
        }

        if in_inline_code.is_some() {
            line_start = bytes[i] == b'\n';
            i += 1;
            continue;
        }

        if let Some((kind, open_len)) = detect_link_start(bytes, i)
            && let Some(close) = find_link_end(bytes, i + open_len)
        {
            let full_start = i;
            let full_end = close + 2;
            let inner_start = i + open_len;
            let inner_end = close;
            let raw_inner = input[inner_start..inner_end].to_string();
            tokens.push(LinkToken {
                kind,
                full_span: full_start..full_end,
                inner_span: inner_start..inner_end,
                parsed: parse_link_target(&raw_inner),
                raw_inner,
            });
            i = full_end;
            line_start = false;
            continue;
        }

        line_start = bytes[i] == b'\n';
        i += 1;
    }

    tokens
}

pub fn parse_link_target(raw_inner: &str) -> ParsedTarget {
    let trimmed = raw_inner.trim();
    let (target_and_anchor, alias_or_size) = split_alias_or_size(trimmed);
    let (target_text, anchor) = split_anchor(target_and_anchor);
    let basename = target_text.rsplit('/').next().unwrap_or(target_text);

    let (normalized_target, is_markdown_note) = if let Some(name) = basename.strip_suffix(".md") {
        (name.to_string(), true)
    } else if basename.contains('.') {
        (basename.to_string(), false)
    } else {
        (basename.to_string(), true)
    };

    ParsedTarget {
        normalized_target,
        target_text: basename.to_string(),
        anchor,
        alias_or_size,
        is_markdown_note,
    }
}

pub fn rebuild_link_token(
    kind: LinkKind,
    target: &str,
    anchor: Option<&str>,
    alias: Option<&str>,
) -> String {
    let mut out = match kind {
        LinkKind::WikiLink => String::from("[["),
        LinkKind::Embed => String::from("![["),
    };
    out.push_str(target);
    if let Some(anchor) = anchor {
        out.push('#');
        out.push_str(anchor);
    }
    if let Some(alias) = alias {
        out.push('|');
        out.push_str(alias);
    }
    out.push_str("]]");
    out
}

fn split_alias_or_size(input: &str) -> (&str, Option<String>) {
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() && bytes[i + 1] == b'|' => {
                return (&input[..i], Some(input[(i + 2)..].to_string()));
            }
            b'|' => {
                return (&input[..i], Some(input[(i + 1)..].to_string()));
            }
            _ => i += 1,
        }
    }
    (input, None)
}

fn split_anchor(input: &str) -> (&str, Option<String>) {
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'#' && (i == 0 || bytes[i - 1] != b'\\') {
            return (&input[..i], Some(input[(i + 1)..].to_string()));
        }
        i += 1;
    }
    (input, None)
}

fn detect_link_start(bytes: &[u8], i: usize) -> Option<(LinkKind, usize)> {
    if bytes.get(i..i + 3) == Some(b"![[") {
        Some((LinkKind::Embed, 3))
    } else if bytes.get(i..i + 2) == Some(b"[[") {
        Some((LinkKind::WikiLink, 2))
    } else {
        None
    }
}

fn find_link_end(bytes: &[u8], mut i: usize) -> Option<usize> {
    while i + 1 < bytes.len() {
        if bytes[i] == b']' && bytes[i + 1] == b']' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn detect_fence_marker(bytes: &[u8], i: usize) -> Option<(u8, usize, usize)> {
    let mut cursor = i;
    while cursor < bytes.len() && bytes[cursor] == b' ' {
        cursor += 1;
    }

    let marker = *bytes.get(cursor)?;
    if marker != b'`' && marker != b'~' {
        return None;
    }

    let count = count_run(bytes, cursor, marker);
    if count < 3 {
        return None;
    }

    Some((marker, count, cursor + count))
}

fn count_run(bytes: &[u8], start: usize, needle: u8) -> usize {
    let mut count = 0;
    while start + count < bytes.len() && bytes[start + count] == needle {
        count += 1;
    }
    count
}

fn skip_to_line_end(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() {
        let byte = bytes[i];
        i += 1;
        if byte == b'\n' {
            break;
        }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_link_tokens_distinguishes_embed_and_wikilink() {
        let tokens = scan_link_tokens("[[note]] ![[note]]", ScanContext::MarkdownBody);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, LinkKind::WikiLink);
        assert_eq!(tokens[1].kind, LinkKind::Embed);
    }

    #[test]
    fn test_parse_link_tokens_preserves_escaped_pipe() {
        let tokens = scan_link_tokens(
            "[[Note\\|Alias]] ![[Image.png\\|200]]",
            ScanContext::MarkdownBody,
        );
        assert_eq!(tokens[0].parsed.normalized_target, "Note");
        assert_eq!(tokens[0].parsed.alias_or_size.as_deref(), Some("Alias"));
        assert_eq!(tokens[1].parsed.normalized_target, "Image.png");
        assert_eq!(tokens[1].parsed.alias_or_size.as_deref(), Some("200"));
    }

    #[test]
    fn test_normalize_target_strips_md_before_anchor_and_alias() {
        let parsed = parse_link_target("note.md#Heading|Alias");
        assert_eq!(parsed.normalized_target, "note");
        assert_eq!(parsed.target_text, "note.md");
        assert_eq!(parsed.anchor.as_deref(), Some("Heading"));
        assert_eq!(parsed.alias_or_size.as_deref(), Some("Alias"));
        assert!(parsed.is_markdown_note);
    }

    #[test]
    fn test_parse_link_tokens_skips_code_contexts() {
        let input = "See [[real]].\n`[[inline]]`\n```\n[[fenced]]\n```\n![[real-image.png]]";
        let tokens = scan_link_tokens(input, ScanContext::MarkdownBody);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].parsed.normalized_target, "real");
        assert_eq!(tokens[1].parsed.normalized_target, "real-image.png");
    }

    #[test]
    fn test_parse_link_tokens_ignores_unclosed_syntax() {
        let tokens = scan_link_tokens("[[note ![[image.png [[", ScanContext::MarkdownBody);
        assert!(tokens.is_empty());
    }
}
