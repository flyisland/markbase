use gray_matter::Matter;
use gray_matter::engine::YAML;
use serde_json::Value;
use std::sync::LazyLock;

use crate::link_syntax::{LinkKind, ScanContext, parse_link_target, scan_link_tokens};

use regex::Regex;

static TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"#[\w\-/]+").unwrap());

/// Validates and normalizes a tag according to Obsidian Tag Format specification.
///
/// # Arguments
/// * `tag` - The tag string (without # prefix)
///
/// # Returns
/// * `Some(String)` - The normalized (lowercase) tag if valid
/// * `None` - If the tag is invalid (pure numeric like "1984")
///
/// Validation rules:
/// - Must contain at least one non-numerical character
/// - Case-insensitive: normalized to lowercase
pub fn normalize_tag(tag: &str) -> Option<String> {
    // Must contain at least one non-digit character
    if tag.chars().any(|c| !c.is_ascii_digit()) {
        // Normalize to lowercase for case-insensitivity
        Some(tag.to_lowercase())
    } else {
        None
    }
}

pub struct Extractor;

impl Extractor {
    pub fn extract(content: &str) -> ExtractedContent {
        let (frontmatter, content_without_fm) = Self::parse_frontmatter(content);
        let tags = Self::extract_tags(&content_without_fm);

        let body_tokens = scan_link_tokens(&content_without_fm, ScanContext::MarkdownBody);
        let embeds: Vec<String> = body_tokens
            .iter()
            .filter(|token| token.kind == LinkKind::Embed)
            .map(|token| token.parsed.normalized_target.clone())
            .collect();
        let body_links: Vec<String> = body_tokens
            .iter()
            .map(|token| token.parsed.normalized_target.clone())
            .collect();
        let fm_links = Self::extract_frontmatter_links(&frontmatter);

        let mut all_links = body_links;
        all_links.extend(fm_links);
        all_links.extend(embeds.clone());
        all_links.sort();
        all_links.dedup();

        let mut sorted_embeds = embeds;
        sorted_embeds.sort();
        sorted_embeds.dedup();

        ExtractedContent {
            frontmatter,
            tags,
            links: all_links,
            embeds: sorted_embeds,
        }
    }

    fn parse_frontmatter(content: &str) -> (Value, String) {
        let matter = Matter::<YAML>::new();
        match matter.parse::<Value>(content) {
            Ok(result) => {
                let frontmatter = result
                    .data
                    .map(|v| serde_json::to_value(v).unwrap_or(Value::Null))
                    .unwrap_or(Value::Null);
                (frontmatter, result.content)
            }
            Err(_) => (Value::Null, content.to_string()),
        }
    }

    fn extract_tags(content: &str) -> Vec<String> {
        TAG_REGEX
            .find_iter(content)
            .filter_map(|m| {
                let tag = m.as_str();
                // Remove leading # and normalize
                let content = &tag[1..]; // Skip the # prefix
                normalize_tag(content)
            })
            .collect()
    }

    pub fn extract_frontmatter_links(frontmatter: &Value) -> Vec<String> {
        let mut links = Vec::new();

        if let Some(obj) = frontmatter.as_object() {
            Self::scan_value_for_links(&Value::Object(obj.clone()), &mut links);
        }

        links
    }

    fn scan_value_for_links(value: &Value, links: &mut Vec<String>) {
        match value {
            Value::String(s) => {
                links.extend(
                    scan_link_tokens(s, ScanContext::FrontmatterString)
                        .into_iter()
                        .filter(|token| token.kind == LinkKind::WikiLink)
                        .map(|token| token.parsed.normalized_target),
                );
            }
            Value::Array(arr) => {
                for item in arr {
                    Self::scan_value_for_links(item, links);
                }
            }
            Value::Object(obj) => {
                for (_, v) in obj {
                    Self::scan_value_for_links(v, links);
                }
            }
            _ => {}
        }
    }

    pub fn normalize_link_name(name: &str) -> String {
        parse_link_target(name).normalized_target
    }
}

pub struct ExtractedContent {
    pub frontmatter: Value,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    pub embeds: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_content() {
        let content = "# Hello World\n\nThis is a test.";
        let extracted = Extractor::extract(content);
        assert!(extracted.frontmatter.is_null());
        assert!(extracted.tags.is_empty());
        assert!(extracted.links.is_empty());
        assert!(extracted.embeds.is_empty());
    }

    #[test]
    fn test_extract_frontmatter() {
        let content = r#"---
title: Test Document
tags: [test, example]
---

# Content

This is the body."#;
        let extracted = Extractor::extract(content);
        assert!(!extracted.frontmatter.is_null());
        assert_eq!(
            extracted
                .frontmatter
                .get("title")
                .unwrap()
                .as_str()
                .unwrap(),
            "Test Document"
        );
    }

    #[test]
    fn test_extract_tags() {
        let content = "This has #tag1 and #tag-2 and #nested/tag";
        let extracted = Extractor::extract(content);
        assert_eq!(extracted.tags.len(), 3);
        assert!(extracted.tags.contains(&"tag1".to_string()));
        assert!(extracted.tags.contains(&"tag-2".to_string()));
        assert!(extracted.tags.contains(&"nested/tag".to_string()));
    }

    #[test]
    fn test_invalid_pure_numeric_tags() {
        // Pure numeric tags like #1984 are invalid per Obsidian spec
        let content = "#1984 #123 #007";
        let extracted = Extractor::extract(content);
        assert!(extracted.tags.is_empty());
    }

    #[test]
    fn test_tag_case_normalization() {
        // Obsidian treats #tag and #TAG as identical - we normalize to lowercase
        let content = "#MyTag #MYTAG #mytag";
        let extracted = Extractor::extract(content);
        assert_eq!(extracted.tags.len(), 3);
        assert!(extracted.tags.iter().all(|t| t == "mytag"));
        assert!(extracted.tags.contains(&"mytag".to_string()));
    }

    #[test]
    fn test_valid_tags_with_numbers() {
        // Tags with at least one non-digit character are valid
        let content = "#tag123 #123tag #tag-123 #123-tag #y1984";
        let extracted = Extractor::extract(content);
        assert_eq!(extracted.tags.len(), 5);
        assert!(extracted.tags.contains(&"tag123".to_string()));
        assert!(extracted.tags.contains(&"123tag".to_string()));
        assert!(extracted.tags.contains(&"tag-123".to_string()));
        assert!(extracted.tags.contains(&"123-tag".to_string()));
        assert!(extracted.tags.contains(&"y1984".to_string()));
    }

    #[test]
    fn test_extract_wikilinks() {
        let content = "See [[architecture]] and [[performance-tips]] for more info.";
        let extracted = Extractor::extract(content);
        assert_eq!(extracted.links.len(), 2);
        assert!(extracted.links.contains(&"architecture".to_string()));
        assert!(extracted.links.contains(&"performance-tips".to_string()));
    }

    #[test]
    fn test_extract_embeds() {
        let content = "Check this image: ![[mobile-app-mockup.png]] and ![[diagram.svg]]";
        let extracted = Extractor::extract(content);
        assert_eq!(extracted.embeds.len(), 2);
        assert!(
            extracted
                .embeds
                .contains(&"mobile-app-mockup.png".to_string())
        );
        assert!(extracted.embeds.contains(&"diagram.svg".to_string()));
    }

    #[test]
    fn test_extract_wikilinks_with_aliases() {
        let content = "See [[architecture|Method Architecture]] for details.";
        let extracted = Extractor::extract(content);
        assert_eq!(extracted.links.len(), 1);
        assert!(extracted.links.contains(&"architecture".to_string()));
    }

    #[test]
    fn test_extract_wikilinks_with_headers() {
        let content = "See [[architecture#Overview]] for details.";
        let extracted = Extractor::extract(content);
        assert_eq!(extracted.links.len(), 1);
        assert!(extracted.links.contains(&"architecture".to_string()));
    }

    #[test]
    fn test_extract_complex_markdown() {
        let content = r#"---
title: Mobile App
tags: [project, mobile]
---

# Mobile App

Check [[architecture]] and [[api-design]].

![[mockup.png]]

#project #mobile #ios"#;
        let extracted = Extractor::extract(content);
        assert!(extracted.links.contains(&"architecture".to_string()));
        assert!(extracted.links.contains(&"api-design".to_string()));
        assert!(extracted.links.contains(&"mockup.png".to_string()));
        assert_eq!(extracted.embeds.len(), 1);
        assert!(extracted.embeds.contains(&"mockup.png".to_string()));
        assert_eq!(extracted.tags.len(), 3);
    }

    #[test]
    fn test_extract_empty_frontmatter() {
        let content = r#"---
---

Content here."#;
        let extracted = Extractor::extract(content);
        assert!(extracted.frontmatter.is_null());
    }

    #[test]
    fn test_extract_no_frontmatter() {
        let content = "--- not frontmatter\n\nContent";
        let extracted = Extractor::extract(content);
        assert!(extracted.frontmatter.is_null());
    }

    #[test]
    fn test_extract_invalid_frontmatter() {
        let content = r#"---
invalid: yaml: [
---

Content"#;
        let extracted = Extractor::extract(content);
        assert!(extracted.frontmatter.is_null());
    }

    #[test]
    fn test_extract_multiple_same_tags() {
        let content = "#tag #tag #tag";
        let extracted = Extractor::extract(content);
        assert_eq!(extracted.tags.len(), 3);
        assert_eq!(extracted.tags.iter().filter(|t| *t == "tag").count(), 3);
    }

    #[test]
    fn test_extract_nested_frontmatter() {
        let content = r#"---
author:
  name: John Doe
  email: john@example.com
tags: [test]
---

Content"#;
        let extracted = Extractor::extract(content);
        let author = extracted.frontmatter.get("author").unwrap();
        assert_eq!(author.get("name").unwrap().as_str().unwrap(), "John Doe");
    }

    #[test]
    fn test_extract_frontmatter_with_numbers() {
        let content = r#"---
count: 42
price: 19.99
active: true
---

Content"#;
        let extracted = Extractor::extract(content);
        assert_eq!(
            extracted
                .frontmatter
                .get("count")
                .unwrap()
                .as_i64()
                .unwrap(),
            42
        );
        assert_eq!(
            extracted
                .frontmatter
                .get("price")
                .unwrap()
                .as_f64()
                .unwrap(),
            19.99
        );
        assert_eq!(
            extracted
                .frontmatter
                .get("active")
                .unwrap()
                .as_bool()
                .unwrap(),
            true
        );
    }

    #[test]
    fn test_extract_frontmatter_with_dates() {
        let content = r#"---
created: 2024-01-15
modified: 2024-06-20T10:30:00Z
---

Content"#;
        let extracted = Extractor::extract(content);
        assert!(extracted.frontmatter.get("created").is_some());
        assert!(extracted.frontmatter.get("modified").is_some());
    }

    #[test]
    fn test_extract_frontmatter_with_array() {
        let content = r#"---
tags: [todo, in-progress, done]
categories:
  - tech
  - personal
---

Content"#;
        let extracted = Extractor::extract(content);
        let tags = extracted.frontmatter.get("tags").unwrap();
        assert!(tags.is_array());
        assert_eq!(tags.as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_extract_frontmatter_with_null() {
        let content = r#"---
title: Test
description: null
---

Content"#;
        let extracted = Extractor::extract(content);
        assert!(extracted.frontmatter.get("description").unwrap().is_null());
    }

    #[test]
    fn test_extract_frontmatter_with_special_chars() {
        let content = r#"---
title: "Test: with colon, and 'quotes'"
url: https://example.com/path?a=1&b=2
regex: ^\d{3}-\d{4}$
---

Content"#;
        let extracted = Extractor::extract(content);
        assert!(extracted.frontmatter.get("title").is_some());
        assert!(extracted.frontmatter.get("url").is_some());
        assert!(extracted.frontmatter.get("regex").is_some());
    }

    #[test]
    fn test_embeds_added_to_links() {
        let content = "See ![[diagram.png]] for details.";
        let extracted = Extractor::extract(content);
        assert!(extracted.links.contains(&"diagram.png".to_string()));
        assert!(extracted.embeds.contains(&"diagram.png".to_string()));
    }

    #[test]
    fn test_frontmatter_links_extracted() {
        let content = r#"---
related: "[[api-spec]]"
---

Content"#;
        let extracted = Extractor::extract(content);
        assert!(extracted.links.contains(&"api-spec".to_string()));
    }

    #[test]
    fn test_links_deduplicated() {
        let content = r#"---
links: "[[same-note]]"
---

See [[same-note]] and ![[same-note]]."#;
        let extracted = Extractor::extract(content);
        let count = extracted.links.iter().filter(|l| *l == "same-note").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_code_blocks_excluded() {
        let content = r#"---
related: "[[linked-note]]"
---

```json
{
  "link": "[[should-not-match]]"
}
```

See [[linked-note]] for details."#;
        let extracted = Extractor::extract(content);
        assert!(extracted.links.contains(&"linked-note".to_string()));
        assert!(!extracted.links.contains(&"should-not-match".to_string()));
    }

    #[test]
    fn test_path_prefix_stripped() {
        let content = "See [[notes/design]] for details.";
        let extracted = Extractor::extract(content);
        assert!(extracted.links.contains(&"design".to_string()));
    }

    #[test]
    fn test_md_extension_stripped() {
        let content = "See [[note.md]] for details.";
        let extracted = Extractor::extract(content);
        assert!(extracted.links.contains(&"note".to_string()));
    }

    #[test]
    fn test_embed_with_section_stripped() {
        let content = "![[architecture#Background]]";
        let extracted = Extractor::extract(content);
        assert!(extracted.embeds.contains(&"architecture".to_string()));
        assert!(extracted.links.contains(&"architecture".to_string()));
    }

    #[test]
    fn test_embed_with_alias() {
        let content = "![[diagram.png|Image]]";
        let extracted = Extractor::extract(content);
        assert!(extracted.embeds.contains(&"diagram.png".to_string()));
        assert!(extracted.links.contains(&"diagram.png".to_string()));
    }

    #[test]
    fn test_wikilink_with_section_and_alias() {
        let content = "See [[old-note#Heading|alias]] for details.";
        let extracted = Extractor::extract(content);
        assert!(extracted.links.contains(&"old-note".to_string()));
    }

    #[test]
    fn test_frontmatter_links_multiple_properties() {
        let content = r#"---
related: "[[note1]]"
seeAlso: "[[note2]]"
---

Content"#;
        let extracted = Extractor::extract(content);
        assert!(extracted.links.contains(&"note1".to_string()));
        assert!(extracted.links.contains(&"note2".to_string()));
    }

    #[test]
    fn test_embed_in_frontmatter_not_extracted() {
        let content = r#"---
embed: "![[embedded-note]]"
---

Content"#;
        let extracted = Extractor::extract(content);
        assert!(!extracted.links.contains(&"embedded-note".to_string()));
    }

    #[test]
    fn test_extract_links_with_escaped_pipe() {
        let content = "| ref |\n| --- |\n| [[Note\\|Alias]] |\n\n![[Image.png\\|200]]";
        let extracted = Extractor::extract(content);
        assert!(extracted.links.contains(&"Note".to_string()));
        assert!(extracted.links.contains(&"Image.png".to_string()));
        assert!(extracted.embeds.contains(&"Image.png".to_string()));
    }

    #[test]
    fn test_extract_frontmatter_text_with_links_but_not_embeds() {
        let content = r#"---
related: "see [[real-note]] and ![[ignored-note]]"
---

Content"#;
        let extracted = Extractor::extract(content);
        assert!(extracted.links.contains(&"real-note".to_string()));
        assert!(!extracted.links.contains(&"ignored-note".to_string()));
        assert!(extracted.embeds.is_empty());
    }

    #[test]
    fn test_design_example() {
        let content = r#"---
tags: [architecture]
related: "[[api-spec]]"
---
See [[system-overview]] and [[api-spec#Authentication|Auth Docs]].
![[diagram.png]]
"#;
        let extracted = Extractor::extract(content);

        assert!(extracted.links.contains(&"system-overview".to_string()));
        assert!(extracted.links.contains(&"api-spec".to_string()));
        assert!(extracted.links.contains(&"diagram.png".to_string()));

        assert!(extracted.embeds.contains(&"diagram.png".to_string()));
    }

    #[test]
    fn test_link_to_block() {
        let content = "See [[note#^block123]] for details.";
        let extracted = Extractor::extract(content);
        assert!(extracted.links.contains(&"note".to_string()));
    }

    #[test]
    fn test_embed_block() {
        let content = "![[note#^blockid]]";
        let extracted = Extractor::extract(content);
        assert!(extracted.embeds.contains(&"note".to_string()));
        assert!(extracted.links.contains(&"note".to_string()));
    }

    #[test]
    fn test_embed_audio() {
        let content = "Listen to ![[podcast.mp3]]";
        let extracted = Extractor::extract(content);
        assert!(extracted.embeds.contains(&"podcast.mp3".to_string()));
    }

    #[test]
    fn test_embed_pdf() {
        let content = "Read ![[report.pdf]]";
        let extracted = Extractor::extract(content);
        assert!(extracted.embeds.contains(&"report.pdf".to_string()));
    }

    #[test]
    fn test_normalize_link_name() {
        assert_eq!(
            Extractor::normalize_link_name("architecture"),
            "architecture"
        );
        assert_eq!(
            Extractor::normalize_link_name("architecture#Background"),
            "architecture"
        );
        assert_eq!(
            Extractor::normalize_link_name("architecture|Alias"),
            "architecture"
        );
        assert_eq!(
            Extractor::normalize_link_name("architecture#Heading|Alias"),
            "architecture"
        );
        assert_eq!(Extractor::normalize_link_name("notes/design"), "design");
        assert_eq!(Extractor::normalize_link_name("notes/design.md"), "design");
        assert_eq!(
            Extractor::normalize_link_name("notes/design#Section|Alias"),
            "design"
        );
        assert_eq!(Extractor::normalize_link_name("diagram.png"), "diagram.png");
        assert_eq!(
            Extractor::normalize_link_name("diagram.png#^block"),
            "diagram.png"
        );
    }
}
