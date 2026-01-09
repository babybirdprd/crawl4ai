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
- **Extraction Strategies**:
    - `JsonCssExtractionStrategy`: CSS selector based extraction.
    - `RegexExtractionStrategy`: Regex based extraction.
    - `JsonXPathExtractionStrategy`: XPath based extraction.
- **Markdown Generation**: `async` implementation.
- **Session Management**: Implemented.
- **CLI**: Implemented in `src/main.rs`.
- **Documentation**: Added doc comments to `src/crawler.rs`, `src/models.rs`, and `src/extraction_strategy.rs`.

## Recent Changes
- **Documentation**: Added comprehensive Rust doc comments to the core modules (`crawler`, `models`, `extraction_strategy`). This should make it much easier for new contributors to understand the codebase.
- **Refactored `content_filter.rs`**: Split into submodules (done by previous agent).
- **Fixed `JsonXPathExtractionStrategy`**: Direct DOM-to-DOM conversion (done by previous agent).

## Next Steps for the Next Agent (The "Heavy" Tasks)
1.  **Advanced Retry Testing**:
    -   Implement a robust integration test for `AsyncWebCrawler`'s retry logic. This involves simulating connection failures.
2.  **Performance Tuning**:
    -   Analyze `BM25ContentFilter` and extraction strategies for performance on large documents.
    -   Benchmark `JsonXPathExtractionStrategy`.
3.  **Strategy Configuration via CLI**:
    -   Extend the CLI (`src/main.rs`) to support passing extraction strategies (CSS/XPath/Regex) via JSON config or command-line flags. Currently it only does basic crawling.
4.  **Unit Tests for Retry Logic**:
    -   While integration tests are hard, unit tests for the retry logic *logic* (independent of the browser) could be added if the logic is extracted to a helper function.

## CLI Usage
```bash
cargo run --bin crawl4ai -- https://example.com --format markdown
```

## Technical Notes
- **Testing**: Run tests with `cargo test -- --test-threads=1`.
- **Chrome Executable**: Set `CHROME_EXECUTABLE` if `chromiumoxide` cannot find your browser.
