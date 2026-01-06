use crate::models::MarkdownGenerationResult;
use html2text::from_read;

pub struct DefaultMarkdownGenerator;

impl Default for DefaultMarkdownGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultMarkdownGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_markdown(&self, html: &str) -> MarkdownGenerationResult {
        let raw_markdown = from_read(html.as_bytes(), 80); // 80 cols width

        // Simplified implementation:
        // In the real one, we'd handle citations, fit_markdown, etc.
        // For now, we populate raw_markdown and copy it to others.

        MarkdownGenerationResult {
            raw_markdown: raw_markdown.clone(),
            markdown_with_citations: raw_markdown.clone(),
            references_markdown: String::new(),
            fit_markdown: None,
            fit_html: None,
        }
    }
}
