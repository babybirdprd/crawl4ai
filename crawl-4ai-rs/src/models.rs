use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum WaitStrategy {
    Fixed(u64), // Milliseconds
    Selector(String),
    JsCondition(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrawlerRunConfig {
    pub session_id: Option<String>,
    pub wait_for: Option<WaitStrategy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrawlResult {
    pub url: String,
    pub html: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleaned_html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<HashMap<String, Vec<MediaItem>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<HashMap<String, Vec<Link>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<MarkdownGenerationResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extracted_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownGenerationResult {
    pub raw_markdown: String,
    pub markdown_with_citations: String,
    pub references_markdown: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit_markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit_html: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub src: Option<String>,
    pub alt: Option<String>,
    pub desc: Option<String>,
    pub score: Option<i32>,
    #[serde(rename = "type")]
    pub type_: String, // "type" is a reserved keyword in Rust
    pub group_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub href: Option<String>,
    pub text: Option<String>,
    pub title: Option<String>,
}
