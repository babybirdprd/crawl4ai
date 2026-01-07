# crawl-4ai-rs Handover Document

## Project Overview
`crawl-4ai-rs` is a Rust port of the Python `crawl4ai` library. It aims to provide a robust, standalone web crawler and scraper with Markdown generation capabilities, suitable for LLM workflows.

## Current State
- **Core Crawler**: Implemented `AsyncWebCrawler` using `chromiumoxide`.
- **Error Handling**: Implemented `CrawlerError` and robust retry logic.
- **Content Filtering**:
    - `PruningContentFilter` (DOM pruning).
    - `BM25ContentFilter` (Text ranking) - **Refined** to use stack-based traversal matching Python's logic (handling nested blocks and inline tags correctly).
- **Extraction Strategies**:
    - **New**: `JsonCssExtractionStrategy` implemented in `src/extraction_strategy.rs`. Supports extracting structured data (text, attributes, html) using CSS selectors and JSON schema.
- **Markdown Generation**: Basic implementation.
- **Session Management**: Implemented.

## Dependencies
- `chromiumoxide` for browser automation.
- `kuchiki` for HTML parsing and CSS selectors.
- `serde`, `serde_json` for data handling.
- `reqwest` for HTTP.
- `rust-stemmers` for BM25.

## Recent Changes
- **Refined Text Chunking**: The `BM25ContentFilter` now uses a stack-based traversal algorithm that closely mirrors the Python `crawl4ai` implementation. It correctly handles block vs inline elements and whitespace trimming. Unit tests added in `src/content_filter.rs`.
- **Added CSS Extraction**: Implemented `JsonCssExtractionStrategy` to allow extracting structured data defined by a JSON schema using CSS selectors. Unit tests added in `src/extraction_strategy.rs`.

## Next Steps for the Next Agent (The "Heavy" Tasks)
1.  **Port `LLMContentFilter`**: This is a major missing feature. It requires:
    -   Implementing a client for LLM providers (OpenAI, etc.) using `reqwest`.
    -   Porting the logic that chunks HTML and sends it to the LLM for cleaning/markdown generation.
    -   Implementing the `perform_completion_with_backoff` utility (retry logic for API calls).
2.  **Port `RegexExtractionStrategy`**:
    -   Add `regex` crate to `Cargo.toml`.
    -   Implement the strategy in `src/extraction_strategy.rs`.
    -   Port the default patterns (email, phone, etc.) from Python.
3.  **Port `JsonXPathExtractionStrategy`**:
    -   Currently `kuchiki` only supports CSS. To support XPath, you might need `libxml` bindings (like `libxml` crate) or another library like `sxd-xpath`. This is a non-trivial dependency decision.
4.  **Unit Tests for Retry Logic**:
    -   Create strict unit tests that mock the browser or network failures to verify the retry mechanism in `AsyncWebCrawler`.
5.  **Performance Tuning**:
    -   The BM25 calculation in Rust is naive. For very large pages, optimize the tokenization and scoring loops.

## Technical Notes
- **Testing**: Run tests with `cargo test -- --test-threads=1` to avoid browser contention during integration tests.
- **Schema**: The `JsonCssExtractionStrategy` uses a `serde_json::Value` schema. The structure matches the Python version (`baseSelector`, `fields`, `type`, etc.).
- **Missing Regex**: The `JsonCssExtractionStrategy` has a placeholder for `type: "regex"` which returns `None`. Implement this when adding `RegexExtractionStrategy`.
