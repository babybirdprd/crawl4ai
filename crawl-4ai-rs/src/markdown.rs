use crate::models::MarkdownGenerationResult;
use crate::content_filter::ContentFilter;
use html2text::from_read;

pub struct DefaultMarkdownGenerator {
    content_filter: Option<ContentFilter>,
}

impl Default for DefaultMarkdownGenerator {
    fn default() -> Self {
        Self::new(None)
    }
}

impl DefaultMarkdownGenerator {
    pub fn new(content_filter: Option<ContentFilter>) -> Self {
        Self { content_filter }
    }

    pub async fn generate_markdown(&self, html: &str) -> MarkdownGenerationResult {
        let raw_markdown = from_read(html.as_bytes(), 80); // 80 cols width

        let (fit_markdown, fit_html) = if let Some(filter) = &self.content_filter {
            let filtered_html = filter.filter_content(html).await;
            let filtered_markdown = from_read(filtered_html.as_bytes(), 80);
            (Some(filtered_markdown), Some(filtered_html))
        } else {
            (None, None)
        };

        MarkdownGenerationResult {
            raw_markdown: raw_markdown.clone(),
            markdown_with_citations: raw_markdown.clone(),
            references_markdown: String::new(),
            fit_markdown,
            fit_html,
        }
    }
}
