# crawl-4ai-rs Handover Document

## Project Overview
`crawl-4ai-rs` is a Rust port of the Python `crawl4ai` library. It aims to provide a robust, standalone web crawler and scraper with Markdown generation capabilities, suitable for LLM workflows.

## Current State
- **Core Crawler**: Implemented `AsyncWebCrawler` using `chromiumoxide`.
- **Error Handling**: Implemented `CrawlerError` and robust retry logic.
- **Content Filtering**:
    - `PruningContentFilter` (DOM pruning).
    - `BM25ContentFilter` (Text ranking).
    - **New**: `LLMContentFilter` (LLM-based filtering/summarization) implemented in `src/content_filter.rs`.
- **Extraction Strategies**:
    - `JsonCssExtractionStrategy`: Supports extracting structured data (text, attributes, html, regex) using CSS selectors.
    - `RegexExtractionStrategy`: Supports extracting entities (emails, URLs, phones, etc.) using regex patterns.
- **Markdown Generation**: Implementation updated to be `async` to support LLM filtering.
- **Session Management**: Implemented.

## Dependencies
- `chromiumoxide` for browser automation.
- `kuchiki` for HTML parsing and CSS selectors.
- `serde`, `serde_json` for data handling.
- `reqwest` for HTTP (used by `LLMContentFilter`).
- `rust-stemmers` for BM25.
- `regex` for pattern matching.
- `wiremock` (dev-dependency) for testing API calls.

## Recent Changes
- **Ported `LLMContentFilter`**:
    - Added `LLMConfig` and `LLMContentFilter` structs to `src/content_filter.rs`.
    - Refactored `ContentFilter::filter_content` and `DefaultMarkdownGenerator::generate_markdown` to be `async`.
    - Implemented chunking logic (`merge_chunks`) matching Python's approach.
    - Implemented parallel async API calls with backoff retry logic using `reqwest` and `tokio`.
    - Added `tests/test_llm_filter.rs` with `wiremock` tests.
- **Updated `AsyncWebCrawler`**:
    - `arun` method now awaits `generate_markdown`.

## Next Steps for the Next Agent (The "Heavy" Tasks)
1.  **Port `JsonXPathExtractionStrategy`**:
    -   Currently `kuchiki` only supports CSS. To support XPath, you might need `libxml` bindings (like `libxml` crate) or another library like `sxd-xpath`. This is a non-trivial dependency decision.
2.  **Unit Tests for Retry Logic**:
    -   Create strict unit tests that mock the browser or network failures to verify the retry mechanism in `AsyncWebCrawler`.
3.  **Performance Tuning**:
    -   The BM25 calculation in Rust is naive. For very large pages, optimize the tokenization and scoring loops.
4.  **Refactor `content_filter.rs`**:
    -   The file is growing large. Consider splitting `Pruning`, `BM25`, and `LLM` into separate files under a `content_filter` module directory.

## Technical Notes
- **Testing**: Run tests with `cargo test -- --test-threads=1` to avoid browser contention during integration tests.
- **Chrome Executable**: When running tests locally, if `chromiumoxide` fails to find Chrome, use `playwright install chromium` and set `CHROME_EXECUTABLE` to the path (e.g., `~/.cache/ms-playwright/.../chrome`).
- **Async Trait Methods**: `ContentFilter` methods are now `async`. If adding new filters, ensure they follow this pattern.
