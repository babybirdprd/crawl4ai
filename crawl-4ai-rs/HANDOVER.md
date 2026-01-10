# crawl-4ai-rs Handover Document

## Project Overview
`crawl-4ai-rs` is a Rust port of the Python `crawl4ai` library. It aims to provide a robust, standalone web crawler and scraper with Markdown generation capabilities, suitable for LLM workflows.

## Current State
- **Core Crawler**: Implemented `AsyncWebCrawler` using `chromiumoxide`.
- **Error Handling & Retry**: Refactored `arun` to use a cleaner loop with explicit state management and timeout support. Added `page_timeout` configuration.
- **Wait Strategies**:
    -   Implemented `XPath` wait strategy (via `document.evaluate`).
    -   Implemented `NetworkIdle` wait strategy (via CDP event listeners). Note: `NetworkIdle` only tracks requests initiated *after* the strategy starts waiting.
    -   Refactored `Selector` and `JsCondition` strategies.
- **Testing**: Added integration tests for retry logic (`tests/test_retry_integration.rs`) simulating network delays/timeouts.
- **Content Filtering**: Pruning, BM25, LLM implemented.
- **Extraction Strategies**: CSS, XPath, Regex implemented.
- **CLI**: Implemented.

## Recent Changes
- **Wait Strategy Improvements**:
    -   Added `XPath` support to `WaitStrategy`.
    -   Added `NetworkIdle` support to `WaitStrategy`.
    -   Improved `Selector` and `JsCondition` timeout handling (though still hardcoded to 10s internally, marked with TODO).
    -   Added escaping for XPath strings to prevent injection/syntax errors.

## Next Steps for the Next Agent
1.  **Performance Tuning**:
    -   Benchmark `JsonXPathExtractionStrategy` vs `JsonCssExtractionStrategy` on large DOMs.
    -   Analyze memory usage during long crawls.
2.  **Configuration Refinement**:
    -   Expose the timeout for `WaitStrategy` in `CrawlerRunConfig` (currently hardcoded to 10s or 30s).
    -   Add configuration for `NetworkIdle` duration (currently 500ms).
3.  **Headless Shell vs Full Chrome**:
    -   Investigate if `chromiumoxide` can run with the lighter `headless_shell` binary for better performance in Docker/Cloud environments.
4.  **Error Handling Granularity**:
    -   Currently, `arun` retries on most errors. Consider categorizing errors better (e.g., 404 Not Found should probably not be retried unless configured, while 500 or Timeouts should be).

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
