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
- **Content Filtering**: Pruning, BM25, basic LLM (placeholder) implemented.
- **Extraction Strategies**: CSS, XPath, Regex implemented.
- **CLI**: Implemented.

## Recent Changes
- **Configuration Refinement**:
    -   Exposed `wait_timeout` in `CrawlerRunConfig`.
    -   Updated `WaitStrategy::NetworkIdle` to accept an optional `idle_time`.
- **Retry Logic Enhancement**:
    -   Added `retry_404` to `CrawlerRunConfig`.
    -   Improved 404 detection and propagation.
- **Performance Benchmarking**:
    -   Benchmarked `JsonCssExtractionStrategy` vs `JsonXPathExtractionStrategy`.
    -   **Result**: CSS extraction is ~2.65x faster than XPath extraction on large DOMs (3.5MB, 5000 items).
    -   *Recommendation*: Prefer CSS selectors for performance-critical extraction where possible.

## Known Issues
- **Headless 404 Handling**: In the current test environment (headless Chromium + wiremock), returning a 404 with an empty body causes `chromiumoxide` to fail navigation with `net::ERR_HTTP_RESPONSE_CODE_FAILURE`.

## Next Steps for the Agent (CRITICAL)
1.  **Implement `rig` for LLM Support**:
    -   The current `LLMContentFilter` is a basic placeholder. You must **implement `rig`** (likely the `rig-core` crate or similar Rust LLM framework) to properly architecture the LLM functionality.
    -   **Support Multiple Providers**: The implementation must support multiple LLM providers (OpenAI, Anthropic, local models, etc.) via `rig`.
    -   **Port Actual Functionality**: Start porting the robust LLM features from the original Python project.
    -   *Note*: This is a heavy workload. Do not cut corners. The goal is a production-ready LLM integration.

2.  **Headless Shell vs Full Chrome**:
    -   Investigate if `chromiumoxide` can run with the lighter `headless_shell` binary.

3.  **Error Handling Granularity**:
    -   Investigate better ways to extract status codes from `chromiumoxide` errors when navigation fails completely.

4. **Documentation**:
    -   Create extensive end user documentation for using the port.

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
