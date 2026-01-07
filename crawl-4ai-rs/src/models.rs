use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::content_filter::ContentFilter;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum WaitStrategy {
    Fixed(u64), // Milliseconds
    Selector(String),
    JsCondition(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentSource {
    #[serde(rename = "raw_html")]
    RawHtml,
    #[serde(rename = "cleaned_html")]
    CleanedHtml,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrawlerRunConfig {
    pub session_id: Option<String>,
    pub wait_for: Option<WaitStrategy>,
    pub content_filter: Option<ContentFilter>,
    pub capture_mhtml: Option<bool>,
    pub capture_network_requests: Option<bool>,
    pub capture_console_messages: Option<bool>,
    pub content_source: Option<ContentSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrawlResult {
    pub url: String,
    pub html: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleaned_html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mhtml: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<HashMap<String, Vec<MediaItem>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_requests: Option<Vec<NetworkRequest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console_messages: Option<Vec<ConsoleMessage>>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    pub url: String,
    pub method: String,
    pub headers: Option<HashMap<String, String>>,
    pub response_status: Option<i32>,
    pub response_headers: Option<HashMap<String, String>>,
    pub request_body: Option<String>,
    pub response_body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleMessage {
    #[serde(rename = "type")]
    pub type_: String,
    pub text: String,
    pub source: Option<String>,
    pub line: Option<i32>,
    pub column: Option<i32>,
    pub url: Option<String>,
}
