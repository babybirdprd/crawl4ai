# crawl-4ai-rs Handover Document

## Project Overview
`crawl-4ai-rs` is a Rust port of the Python `crawl4ai` library. It aims to provide a robust, standalone web crawler and scraper with Markdown generation capabilities, suitable for LLM workflows.

## Current State
- **Core Crawler**: Implemented `AsyncWebCrawler` using `chromiumoxide`.
- **Error Handling**: Implemented `CrawlerError` and robust retry logic.
- **Content Filtering**:
    - `PruningContentFilter` (DOM pruning).
    - `BM25ContentFilter` (Text ranking) - **Refined** to use stack-based traversal matching Python's logic.
- **Extraction Strategies**:
    - `JsonCssExtractionStrategy`: Supports extracting structured data (text, attributes, html, regex) using CSS selectors.
    - **New**: `RegexExtractionStrategy`: Implemented in `src/extraction_strategy.rs`. Supports extracting entities (emails, URLs, phones, etc.) using regex patterns.
- **Markdown Generation**: Basic implementation.
- **Session Management**: Implemented.

## Dependencies
- `chromiumoxide` for browser automation.
- `kuchiki` for HTML parsing and CSS selectors.
- `serde`, `serde_json` for data handling.
- `reqwest` for HTTP.
- `rust-stemmers` for BM25.
- `regex` for pattern matching.

## Recent Changes
- **Ported `RegexExtractionStrategy`**: Added `RegexExtractionStrategy` struct and implementation in `src/extraction_strategy.rs`. It includes default patterns for common entities (email, url, phone, etc.) matching the Python implementation.
- **Updated `JsonCssExtractionStrategy`**: Added support for `type: "regex"` in fields, allowing regex extraction on top of CSS selection.
- **Testing**: Added unit tests for `RegexExtractionStrategy` and `JsonCssExtractionStrategy` regex support in `src/extraction_strategy.rs`. Verified all tests pass.

## Next Steps for the Next Agent (The "Heavy" Tasks)
1.  **Port `LLMContentFilter`**: This is a major missing feature. It requires:
    -   Implementing a client for LLM providers (OpenAI, etc.) using `reqwest`.
    -   Porting the logic that chunks HTML and sends it to the LLM for cleaning/markdown generation.
    -   Implementing the `perform_completion_with_backoff` utility (retry logic for API calls).
2.  **Port `JsonXPathExtractionStrategy`**:
    -   Currently `kuchiki` only supports CSS. To support XPath, you might need `libxml` bindings (like `libxml` crate) or another library like `sxd-xpath`. This is a non-trivial dependency decision.
3.  **Unit Tests for Retry Logic**:
    -   Create strict unit tests that mock the browser or network failures to verify the retry mechanism in `AsyncWebCrawler`.
4.  **Performance Tuning**:
    -   The BM25 calculation in Rust is naive. For very large pages, optimize the tokenization and scoring loops.

## Technical Notes
- **Testing**: Run tests with `cargo test -- --test-threads=1` to avoid browser contention during integration tests.
- **Chrome Executable**: When running tests locally, if `chromiumoxide` fails to find Chrome, use `playwright install chromium` and set `CHROME_EXECUTABLE` to the path (e.g., `~/.cache/ms-playwright/.../chrome`).
- **Schema**: The `JsonCssExtractionStrategy` uses a `serde_json::Value` schema. The structure matches the Python version (`baseSelector`, `fields`, `type`, etc.).
