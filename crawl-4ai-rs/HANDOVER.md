# crawl-4ai-rs Handover Document

## Project Overview
`crawl-4ai-rs` is a Rust port of the Python `crawl4ai` library. It aims to provide a robust, standalone web crawler and scraper with Markdown generation capabilities, suitable for LLM workflows.

## Current State
- **Core Crawler**: Implemented `AsyncWebCrawler` using `chromiumoxide` for headless browser automation.
- **Models**: Ported `CrawlResult` and `MarkdownGenerationResult` structs. Added `CrawlerRunConfig` for passing configuration options.
- **Session Management**: Implemented basic session management. `AsyncWebCrawler` now maintains a map of session IDs to `BrowserContextId`, allowing reuse of browser contexts (cookies, local storage, etc.) across crawls.
- **Smart Waiting**: Implemented `WaitStrategy` enum (`Fixed`, `Selector`, `JsCondition`) and updated `AsyncWebCrawler` to support waiting for specific conditions before extracting content. This improves robustness for dynamic content.
- **Media & Link Extraction**: Implemented logic in `arun` to extract images and links from the DOM using JavaScript injection.
- **Content Filtering**: Implemented `PruningContentFilter` (using `kuchiki`) to prune the DOM before markdown conversion.
- **Markdown Generation**: Updated `DefaultMarkdownGenerator` to use `PruningContentFilter`.
- **Browser Configuration**: Configurable via `CHROME_EXECUTABLE` environment variable. Defaults to auto-discovery or specific paths (/usr/bin/google-chrome-stable, /usr/bin/chromium).
- **Testing**:
    - `tests/basic_crawl.rs`: Integration tests for basic crawling and markdown generation.
    - `tests/session_test.rs`: Verification of session reuse logic.
    - `tests/wait_test.rs`: Verification of smart waiting strategies (fixed delay, selector, JS condition).

## Architecture
- **Language**: Rust (2021 edition).
- **Async Runtime**: `tokio`.
- **Browser Automation**: `chromiumoxide` (Chrome DevTools Protocol).
- **DOM Manipulation**: `kuchiki` (based on `html5ever`).
- **HTTP Client**: `reqwest` (currently unused in core logic but added as dependency).
- **Serialization**: `serde`, `serde_json`.
- **HTML Processing**: `html2text`.

## Setup & Usage
1.  **Dependencies**: Ensure Google Chrome or Chromium is installed.
2.  **Environment**: Set `CHROME_EXECUTABLE` if Chrome is in a non-standard location.
    ```bash
    export CHROME_EXECUTABLE=/usr/bin/google-chrome
    ```
    *Note: In the development sandbox, use `python -m playwright install --with-deps chromium` if needed.*
3.  **Running Tests**:
    ```bash
    cargo test
    ```
    *Note: If encountering "oneshot canceled" errors, try running tests sequentially:*
    ```bash
    cargo test -- --test-threads=1
    ```

## Changes Made (Smart Waiting)
- Added `WaitStrategy` enum to `src/models.rs`.
- Updated `CrawlerRunConfig` to include `wait_for: Option<WaitStrategy>`.
- Modified `AsyncWebCrawler::arun` in `src/crawler.rs` to handle waiting strategies after navigation.
- Improved browser handler loop in `src/crawler.rs` to be more resilient to non-fatal errors (e.g. Serde errors on unknown messages).
- Added `tests/wait_test.rs` to verify new functionality.

## Next Steps for the Next Agent
1.  **Error Handling**: Enhance error mapping and retries. The "oneshot canceled" error from `chromiumoxide` can still happen if the browser crashes or disconnects. Robust retry logic and clearer error messages are needed.
2.  **Docker Support**: Add a Dockerfile for easy deployment.
3.  **BM25 Content Filter**: Implement the `BM25ContentFilter` strategy.
4.  **Full Feature Parity**: Continue porting features from Python (e.g., specific extraction strategies, proxies, more detailed config options).

## Technical Notes
- **Browser Sandbox**: The crawler is currently configured with `--no-sandbox` to run in containerized environments. Added `--disable-gpu` and `--disable-setuid-sandbox` for better stability.
- **Flakiness**: Browser automation tests can be flaky due to resource contention. Running sequentially helps.
- **Chromium Compatibility**: `chromiumoxide` 0.5.7 might have some issues with very new Chromium versions (causing Serde errors on unknown events), but basic functionality works if these errors are ignored in the handler loop.
