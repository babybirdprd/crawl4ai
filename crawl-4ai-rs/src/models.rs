use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::content_filter::ContentFilter;
use crate::extraction_strategy::{JsonCssExtractionStrategy, JsonXPathExtractionStrategy, RegexExtractionStrategy};

/// Strategy to wait for content to load before extracting it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum WaitStrategy {
    /// Wait for a fixed amount of time in milliseconds.
    Fixed(u64),
    /// Wait for a CSS selector to appear in the DOM.
    Selector(String),
    /// Wait for an XPath to appear in the DOM.
    XPath(String),
    /// Wait for a JavaScript condition to evaluate to true.
    JsCondition(String),
    /// Wait for network to be idle (no active requests for 500ms).
    NetworkIdle {
        /// Time in milliseconds for the network to be idle (default: 500ms).
        #[serde(default)]
        idle_time: Option<u64>,
    },
}

/// Configuration for extraction strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExtractionStrategyConfig {
    #[serde(rename = "css")]
    JsonCss(JsonCssExtractionStrategy),
    #[serde(rename = "xpath")]
    JsonXPath(JsonXPathExtractionStrategy),
    #[serde(rename = "regex")]
    Regex(RegexExtractionStrategy),
}

/// Configuration for a crawler run.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrawlerRunConfig {
    /// Optional session ID for persistent browser contexts.
    pub session_id: Option<String>,
    /// Strategy to wait for content loading.
    pub wait_for: Option<WaitStrategy>,
    /// Content filter to use for processing HTML.
    pub content_filter: Option<ContentFilter>,
    /// Extraction strategy to use.
    pub extraction_strategy: Option<ExtractionStrategyConfig>,
    /// Whether to take a screenshot of the page.
    #[serde(default)]
    pub screenshot: bool,
    /// Timeout for page navigation in milliseconds.
    pub page_timeout: Option<u64>,
    /// Timeout for the wait strategy in milliseconds (default: 10000ms).
    pub wait_timeout: Option<u64>,
}

/// Result of a crawl operation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrawlResult {
    /// The URL that was crawled.
    pub url: String,
    /// The raw HTML content of the page.
    pub html: String,
    /// Whether the crawl was successful.
    pub success: bool,
    /// The cleaned HTML content (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleaned_html: Option<String>,
    /// Extracted media items (images, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<HashMap<String, Vec<MediaItem>>>,
    /// Extracted links (internal and external).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<HashMap<String, Vec<Link>>>,
    /// Base64 encoded screenshot data (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    /// Generated markdown content (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<MarkdownGenerationResult>,
    /// Content extracted via strategies (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extracted_content: Option<String>,
    /// Error message if the crawl failed (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Result of markdown generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownGenerationResult {
    /// The raw markdown content.
    pub raw_markdown: String,
    /// Markdown with citations (if applicable).
    pub markdown_with_citations: String,
    /// References section of the markdown.
    pub references_markdown: String,
    /// Markdown fitted to context window (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit_markdown: Option<String>,
    /// HTML fitted to context window (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit_html: Option<String>,
}

/// Represents a media item found on the page (e.g., an image).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    /// The source URL of the media.
    pub src: Option<String>,
    /// The alt text of the media.
    pub alt: Option<String>,
    /// The description or title of the media.
    pub desc: Option<String>,
    /// Relevance score (optional).
    pub score: Option<i32>,
    /// The type of media (e.g., "image").
    #[serde(rename = "type")]
    pub type_: String, // "type" is a reserved keyword in Rust
    /// Group ID for related media items (optional).
    pub group_id: Option<i32>,
}

/// Represents a hyperlink found on the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    /// The href URL of the link.
    pub href: Option<String>,
    /// The visible text of the link.
    pub text: Option<String>,
    /// The title attribute of the link.
    pub title: Option<String>,
}
