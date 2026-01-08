# crawl-4ai-rs Handover Document

## Project Overview
`crawl-4ai-rs` is a Rust port of the Python `crawl4ai` library. It aims to provide a robust, standalone web crawler and scraper with Markdown generation capabilities, suitable for LLM workflows.

## Current State
- **Core Crawler**: Implemented `AsyncWebCrawler` using `chromiumoxide`.
- **Error Handling**: Implemented `CrawlerError` and robust retry logic.
- **Content Filtering**:
    - `PruningContentFilter` (DOM pruning).
    - `BM25ContentFilter` (Text ranking).
    - `LLMContentFilter` (LLM-based filtering/summarization).
    - **Refactored**: `src/content_filter.rs` has been split into a module `src/content_filter/` with `pruning.rs`, `bm25.rs`, and `llm.rs`.
- **Extraction Strategies**:
    - `JsonCssExtractionStrategy`: Supports extracting structured data (text, attributes, html, regex) using CSS selectors.
    - `RegexExtractionStrategy`: Supports extracting entities (emails, URLs, phones, etc.) using regex patterns.
    - **New**: `JsonXPathExtractionStrategy`: Implemented using `sxd-xpath` and `sxd-document`. It currently supports standard XPath queries on XHTML-compatible content.
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
- **New**: `sxd-xpath` and `sxd-document` for XPath support.

## Recent Changes
- **Refactored `content_filter.rs`**: Split the large file into manageable submodules.
- **Ported `JsonXPathExtractionStrategy`**: Added XPath support for structured extraction. Note that `sxd-document` requires valid XML/XHTML, so the HTML is serialized before parsing.
- **Retry Logic**: Attempted to add unit tests for retry logic, but encountered difficulties mocking browser connection failures reliably.

## Next Steps for the Next Agent (The "Heavy" Tasks)
1.  **Robust XPath Support**:
    -   The current `JsonXPathExtractionStrategy` relies on `kuchiki` to serialize HTML to text, then `sxd-document` to parse it. This might be brittle for malformed HTML. Investigate using `libxml` or a more forgiving XML parser if `sxd-document` proves too strict for real-world scraping.
    -   Optimize the implementation to avoid re-compiling XPath queries for every field.
2.  **Advanced Retry Testing**:
    -   Implement a robust integration test for `AsyncWebCrawler`'s retry logic. This might require a custom proxy or a more sophisticated mock server setup to simulate connection drops/resets that trigger the browser's retry mechanism.
3.  **Performance Tuning**:
    -   Analyze `BM25ContentFilter` and extraction strategies for performance on large documents.
4.  **Documentation**:
    -   Add Rust documentation (doc comments) to the new modules and public APIs.

## Technical Notes
- **Testing**: Run tests with `cargo test -- --test-threads=1` to avoid browser contention during integration tests.
- **Chrome Executable**: When running tests locally, if `chromiumoxide` fails to find Chrome, use `playwright install chromium` and set `CHROME_EXECUTABLE` to the path.
- **XPath Limitation**: `sxd-xpath` supports XPath 1.0. Newer XPath features are not available.
