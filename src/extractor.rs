use gray_matter::Matter;
use gray_matter::engine::YAML;
use regex::Regex;
use serde_json::Value;

static WIKILINK_REGEX: &str = r"\[\[([^\]]+)\]\]";
static EMBED_REGEX: &str = r"!\[\[([^\]]+)\]\]";
static TAG_REGEX: &str = r"#[\w\-/]+";

pub struct Extractor;

impl Extractor {
    pub fn extract(content: &str) -> ExtractedContent {
        let (frontmatter, content_without_fm) = Self::parse_frontmatter(content);
        let tags = Self::extract_tags(&content_without_fm);
        let links = Self::extract_wikilinks(&content_without_fm);
        let embeds = Self::extract_embeds(&content_without_fm);

        ExtractedContent {
            full_content: content.to_string(),
            frontmatter,
            tags,
            links,
            embeds,
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
        let re = Regex::new(TAG_REGEX).unwrap();
        re.find_iter(content)
            .map(|m| m.as_str().trim_start_matches('#').to_string())
            .collect()
    }

    fn extract_wikilinks(content: &str) -> Vec<String> {
        let re = Regex::new(WIKILINK_REGEX).unwrap();
        re.captures_iter(content)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect()
    }

    fn extract_embeds(content: &str) -> Vec<String> {
        let re = Regex::new(EMBED_REGEX).unwrap();
        re.captures_iter(content)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect()
    }
}

pub struct ExtractedContent {
    pub full_content: String,
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
        assert_eq!(extracted.full_content, content);
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
        assert_eq!(extracted.full_content, content);
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
        let content = "See [[architecture|System Architecture]] for details.";
        let extracted = Extractor::extract(content);
        assert_eq!(extracted.links.len(), 1);
        assert!(
            extracted
                .links
                .contains(&"architecture|System Architecture".to_string())
        );
    }

    #[test]
    fn test_extract_wikilinks_with_headers() {
        let content = "See [[architecture#Overview]] for details.";
        let extracted = Extractor::extract(content);
        assert_eq!(extracted.links.len(), 1);
        assert!(
            extracted
                .links
                .contains(&"architecture#Overview".to_string())
        );
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
        assert_eq!(extracted.links.len(), 3); // [[architecture]], [[api-design]], [[mockup.png]] from embed
        assert_eq!(extracted.embeds.len(), 1);
        assert_eq!(extracted.tags.len(), 3);
        assert_eq!(
            extracted
                .frontmatter
                .get("title")
                .unwrap()
                .as_str()
                .unwrap(),
            "Mobile App"
        );
    }

    #[test]
    fn test_extract_empty_frontmatter() {
        let content = r#"---
---

Content here."#;
        let extracted = Extractor::extract(content);
        assert!(extracted.frontmatter.is_null());
        assert_eq!(extracted.full_content, content);
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
}
