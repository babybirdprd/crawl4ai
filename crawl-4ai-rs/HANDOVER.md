# crawl-4ai-rs Handover Document

## Project Overview
`crawl-4ai-rs` is a Rust port of the Python `crawl4ai` library. It aims to provide a robust, standalone web crawler and scraper with Markdown generation capabilities, suitable for LLM workflows.

## Current State
- **Core Crawler**: Implemented `AsyncWebCrawler` using `chromiumoxide`.
- **Error Handling & Retry**: Refactored `arun` to use a cleaner loop with explicit state management and timeout support. Added `page_timeout` configuration.
- **Testing**: Added integration tests for retry logic (`tests/test_retry_integration.rs`) simulating network delays/timeouts.
- **Content Filtering**: Pruning, BM25, LLM implemented.
- **Extraction Strategies**: CSS, XPath, Regex implemented.
- **CLI**: Implemented.

## Recent Changes
- **Refactored `arun`**: Extracted `crawl_page` and `prepare_session` methods to separate concerns and handle borrow checker constraints cleanly.
- **Retry Logic**: Verified and tested retry logic with `wiremock` integration tests. The crawler now correctly retries on timeouts and navigates safely using `page.goto()` instead of relying on implicit `wait_for_navigation`.
- **Timeout Configuration**: Added `page_timeout` to `CrawlerRunConfig`.

## Next Steps for the Next Agent (The "Heavy" Tasks)
1.  **Performance Tuning**:
    -   Benchmark `JsonXPathExtractionStrategy` vs `JsonCssExtractionStrategy` on large DOMs.
    -   Analyze memory usage during long crawls.
2.  **Wait Strategy Improvements**:
    -   The current `WaitStrategy` implementation is basic. Consider adding more sophisticated waiting conditions (e.g., network idle, specific network request completion).
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
