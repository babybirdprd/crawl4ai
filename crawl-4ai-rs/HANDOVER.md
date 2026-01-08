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
    - **JsonXPathExtractionStrategy**: Fully implemented and tested. It converts the `kuchiki` (HTML) DOM directly to an `sxd-document` (XPath) DOM, allowing robust querying even on malformed HTML (e.g., unclosed tags).
- **Markdown Generation**: Implementation updated to be `async` to support LLM filtering.
- **Session Management**: Implemented.
- **CLI**: Implemented a command-line interface in `src/main.rs` (binary `crawl4ai`) to crawl URLs, generate Markdown/JSON, and take screenshots.
- **Screenshots**: Added `screenshot` boolean to `CrawlerRunConfig` and `screenshot` field to `CrawlResult`.

## Dependencies
- `chromiumoxide` for browser automation.
- `kuchiki` for HTML parsing and CSS selectors.
- `serde`, `serde_json` for data handling.
- `reqwest` for HTTP (used by `LLMContentFilter`).
- `rust-stemmers` for BM25.
- `regex` for pattern matching.
- `wiremock` (dev-dependency) for testing API calls.
- `sxd-xpath` and `sxd-document` for XPath support.

## Recent Changes
- **Fixed `JsonXPathExtractionStrategy`**: Replaced the brittle "serialize to text -> parse as XML" workflow with a direct DOM-to-DOM conversion (`kuchiki` -> `sxd-document`). This fixes issues with `sxd-document` failing on valid HTML5 (e.g., `<br>`) that isn't strict XML.
- **Refactored `content_filter.rs`**: Split the large file into manageable submodules.
- **Retry Logic**: Attempted to add unit tests for retry logic, but encountered difficulties mocking browser connection failures reliably.

## Next Steps for the Next Agent (The "Heavy" Tasks)
1.  **Advanced Retry Testing**:
    -   Implement a robust integration test for `AsyncWebCrawler`'s retry logic. This might require a custom proxy or a more sophisticated mock server setup to simulate connection drops/resets that trigger the browser's retry mechanism.
2.  **Performance Tuning**:
    -   Analyze `BM25ContentFilter` and extraction strategies for performance on large documents. The current DOM traversal for `BM25` could potentially be optimized.
    -   Benchmark the new `JsonXPathExtractionStrategy` conversion overhead.
3.  **Documentation**:
    -   Add Rust documentation (doc comments) to the new modules and public APIs.
4.  **Strategy Configuration via CLI**:
    -   Currently, the CLI (`src/main.rs`) only performs a basic crawl. Extend it to support passing extraction strategies (CSS/XPath/Regex) via JSON config or command-line flags.

## CLI Usage
The project now includes a CLI binary. You can run it via `cargo run --bin crawl4ai`.
```bash
cargo run --bin crawl4ai -- https://example.com --format markdown
cargo run --bin crawl4ai -- https://example.com --output result.json --format json
cargo run --bin crawl4ai -- https://example.com --screenshot --output page.md
```

## Technical Notes
- **Testing**: Run tests with `cargo test -- --test-threads=1` to avoid browser contention during integration tests.
- **Chrome Executable**: When running tests locally, if `chromiumoxide` fails to find Chrome, use `playwright install chromium` and set `CHROME_EXECUTABLE` to the path (e.g., `export CHROME_EXECUTABLE=...`).
- **XPath Limitation**: `sxd-xpath` supports XPath 1.0. Newer XPath features are not available.
