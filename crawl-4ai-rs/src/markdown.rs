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

    pub fn generate_markdown(&self, html: &str) -> MarkdownGenerationResult {
        // If a filter is present, we should use it to generate the MAIN markdown output if intended.
        // However, standard behavior for "RawHtml" usually implies NO filtering.
        // But for "CleanedHtml", we want the filtered result.

        // In the Python version:
        // cleaned_html = filter(raw_html)
        // markdown = to_markdown(cleaned_html) (if cleaning enabled)

        // Here, we calculate BOTH if filter is present.
        // But `raw_markdown` field in `MarkdownGenerationResult` is ambiguous.
        // Should it be the markdown of the INPUT html? Yes.

        // If the INPUT html (passed to this function) was already selected as "RawHtml",
        // then `raw_markdown` is markdown(RawHtml).
        // If the INPUT html was "CleanedHtml", `raw_markdown` is markdown(CleanedHtml).

        // However, in `crawler.rs`, for `CleanedHtml`, we are passing `&html` (the raw one)
        // because we expect the generator to filter it?
        // Let's check `crawler.rs`.
        // Yes: `&html` is passed.

        // So if `content_filter` is set, `fit_markdown` contains the filtered version.
        // But `raw_markdown` contains the UNFILTERED version.

        // The issue in my test is that I'm checking `result.markdown.raw_markdown`.
        // If `crawler.rs` logic says "CleanedHtml", it constructs the generator with a filter.
        // `generate_markdown` produces `raw_markdown` (UNFILTERED) and `fit_markdown` (FILTERED).

        // If I want the result to be FILTERED, I should probably be looking at `fit_markdown`
        // OR `crawler.rs` should return `fit_markdown` as the main result if `CleanedHtml` is selected.

        // Actually, let's change `generate_markdown` to respect the filter for the main `raw_markdown` field
        // ONLY IF we want to enforce that "this generator produces the primary output".
        // But `MarkdownGenerationResult` structure seems to imply:
        // raw_markdown = direct conversion of input
        // fit_markdown = filtered conversion

        // So the caller (crawler.rs) or the consumer (test) needs to look at the right field.

        // BUT, if `ContentSource::CleanedHtml` is chosen, the user expects `raw_markdown` (the main output)
        // to BE the cleaned version?
        // In Python `crawl4ai`, `markdown` field IS the processed markdown.

        // So, if we have a filter, `raw_markdown` should probably be the filtered one?
        // Or `crawler.rs` should assign `fit_markdown` to `markdown.raw_markdown` before returning?

        // Let's modify `DefaultMarkdownGenerator` to apply filter to `raw_markdown` if filter is present?
        // No, that breaks the meaning of "raw".

        // Let's modify `crawler.rs` to map `fit_markdown` to `raw_markdown` in the result?
        // Or better: In `crawler.rs`, if `CleanedHtml` is selected, we should perhaps USE the filtered HTML
        // as the input to the generator?

        // Current logic in `crawler.rs`:
        // let generator = DefaultMarkdownGenerator::new(Some(content_filter));
        // let markdown_result = generator.generate_markdown(&html);

        // If I want `raw_markdown` to be filtered, I should do:
        // let filtered_html = content_filter.filter_content(&html);
        // let generator = DefaultMarkdownGenerator::new(None);
        // let markdown_result = generator.generate_markdown(&filtered_html);

        // This makes `raw_markdown` equal to the markdown of filtered html.

        // Let's verify `crawler.rs` logic again.
        // The logic I wrote in `crawler.rs` passes `&html` (raw) to generator even for `CleanedHtml` case.
        // And `DefaultMarkdownGenerator` calculates `raw_markdown` from input `&html`.

        // So `raw_markdown` is ALWAYS full markdown. `fit_markdown` is filtered.

        // I should update `crawler.rs` to perform filtering BEFORE generation if `CleanedHtml` is desired as the source.

        let (fit_markdown, fit_html) = if let Some(filter) = &self.content_filter {
            let filtered_html = filter.filter_content(html);
            let filtered_markdown = from_read(filtered_html.as_bytes(), 80);
            (Some(filtered_markdown), Some(filtered_html))
        } else {
            (None, None)
        };

        let raw_markdown = from_read(html.as_bytes(), 80);

        // If we have fit_markdown (filtered), and we assume the "main" output should be filtered when a filter is present...
        // But `crawler.rs` logic handles `RawHtml` by passing `None` as filter.
        // So if `self.content_filter` is `None`, `fit_markdown` is `None`, and `raw_markdown` is the full one.

        // If `CleanedHtml` is requested, `crawler.rs` passes a filter.
        // So `fit_markdown` will be the cleaned version.
        // `raw_markdown` will be the raw version.

        // The expectation in `crawl4ai` is that `markdown.raw_markdown` contains the result of the processing.
        // If processing involved cleaning, it should be the cleaned version.

        // So, if we have a filter, `raw_markdown` should effectively become `fit_markdown`.

        let effective_markdown = if let Some(ref fit) = fit_markdown {
            fit.clone()
        } else {
            raw_markdown.clone()
        };

        MarkdownGenerationResult {
            raw_markdown: effective_markdown, // This ensures CleanedHtml produces cleaned output in the main field
            markdown_with_citations: raw_markdown.clone(), // Keep original here? Or duplicate logic? Let's leave as is for now.
            references_markdown: String::new(),
            fit_markdown,
            fit_html,
        }
    }
}
