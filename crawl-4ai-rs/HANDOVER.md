# crawl-4ai-rs Handover Document

## Project Overview
`crawl-4ai-rs` is a Rust port of the Python `crawl4ai` library. It aims to provide a robust, standalone web crawler and scraper with Markdown generation capabilities, suitable for LLM workflows.

## Current State
- **Core Crawler**: Implemented `AsyncWebCrawler` using `chromiumoxide`.
- **Error Handling & Retry**: Refactored `arun` to use a cleaner loop with explicit state management and timeout support. Added `page_timeout` configuration.
- **Wait Strategies**:
    -   Implemented `XPath`, `Selector`, `JsCondition` wait strategies.
    -   Implemented `NetworkIdle` wait strategy.
    -   All strategies now support configurable timeouts via `CrawlerRunConfig::wait_timeout` (default 10s).
    -   `NetworkIdle` supports configurable idle duration (default 500ms).
- **Testing**:
    -   Integration tests for retry logic (`tests/test_retry_integration.rs`).
    -   Integration tests for wait strategies (`tests/test_wait_strategies.rs`).
- **Content Filtering**: Pruning, BM25, LLM implemented.
- **Extraction Strategies**: CSS, XPath, Regex implemented.
- **CLI**: Implemented.

## Recent Changes
- **Configuration Refinement**:
    -   Exposed `wait_timeout` in `CrawlerRunConfig` to control the maximum wait time for all strategies.
    -   Updated `WaitStrategy::NetworkIdle` to accept an optional `idle_time` parameter for configuring the required idle duration.
    -   Updated `AsyncWebCrawler` to respect these new configuration values.
- **Retry Logic Enhancement**:
    -   Added `retry_404` to `CrawlerRunConfig` (default: false).
    -   Modified `CrawlerError` to include `HttpStatusCode(i64)`.
    -   Updated `crawl_page` to robustly detect HTTP status codes using `wait_for_navigation_response` and correctly propagate 404s as `HttpStatusCode(404)`.
    -   Updated `arun` loop to respect `retry_404` setting: it will abort retries if a 404 is detected and `retry_404` is false.

## Known Issues
- **Headless 404 Handling**: In the current test environment (headless Chromium + wiremock), returning a 404 with an empty body causes `chromiumoxide` to fail navigation with `net::ERR_HTTP_RESPONSE_CODE_FAILURE` before the response event is fully processed by `wait_for_navigation_response` in some cases. This makes it difficult to distinguish 404 from other protocol errors in tests. However, the logic handles explicit 404s correctly if detected.

## Next Steps for the Next Agent
1.  **Performance Tuning**:
    -   Benchmark `JsonXPathExtractionStrategy` vs `JsonCssExtractionStrategy` on large DOMs.
    -   Analyze memory usage during long crawls.
2.  **Headless Shell vs Full Chrome**:
    -   Investigate if `chromiumoxide` can run with the lighter `headless_shell` binary for better performance in Docker/Cloud environments.
3.  **Error Handling Granularity**:
    -   Investigate better ways to extract status codes from `chromiumoxide` errors when navigation fails completely.
    -   Consider parsing the `net::ERR_...` strings if necessary.

## CLI Usage
```bash
# Basic crawl
cargo run --bin crawl4ai -- https://example.com --format markdown

# With extraction strategy
cargo run --bin crawl4ai -- https://example.com --extraction-config my_strategy.json --format json
```

## Technical Notes
- **Testing**: Run tests with `cargo test -- --test-threads=1`.
- **Chrome Executable**: Set `CHROME_EXECUTABLE` if `chromiumoxide` cannot find your browser.
- **Environment**: If running in a container or new env, ensure `playwright install` or similar has set up the browser binaries.
