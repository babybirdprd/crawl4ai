# crawl-4ai-rs Handover Document

## Project Overview
`crawl-4ai-rs` is a Rust port of the Python `crawl4ai` library. It aims to provide a robust, standalone web crawler and scraper with Markdown generation capabilities, suitable for LLM workflows.

## Current State
- **Core Crawler**: Implemented `AsyncWebCrawler` using `chromiumoxide` for headless browser automation.
- **Error Handling**: Implemented robust error handling with retries (3 attempts) and browser restart logic upon critical failures (e.g. channel closed, transport error). Refactored `start()` to be idempotent.
- **Docker Support**: Added `Dockerfile` for containerized deployment, including all dependencies for Chrome/Chromium.
- **Models**: Ported `CrawlResult` and `MarkdownGenerationResult` structs. Added `CrawlerRunConfig` for passing configuration options.
- **Session Management**: Implemented basic session management. `AsyncWebCrawler` now maintains a map of session IDs to `BrowserContextId`, allowing reuse of browser contexts (cookies, local storage, etc.) across crawls.
- **Smart Waiting**: Implemented `WaitStrategy` enum (`Fixed`, `Selector`, `JsCondition`) and updated `AsyncWebCrawler` to support waiting for specific conditions before extracting content. This improves robustness for dynamic content.
- **Media & Link Extraction**: Implemented logic in `arun` to extract images and links from the DOM using JavaScript injection.
- **Content Filtering**:
    - Implemented `PruningContentFilter` (using `kuchiki`) to prune the DOM before markdown conversion.
    - Implemented `BM25ContentFilter` (using `rust-stemmers` and custom BM25 logic) to rank and filter text chunks based on relevance to a query.
    - Introduced `ContentFilter` enum to allow selecting different filtering strategies.
    - **Refined Text Extraction**: Improved `extract_text_chunks` in `BM25ContentFilter` to correctly handle mixed content (inline text inside block containers with other block children) by creating synthetic nodes, preventing duplication and concatenation issues.
- **Markdown Generation**: Updated `DefaultMarkdownGenerator` to use the `ContentFilter` enum.
- **Browser Configuration**: Configurable via `CHROME_EXECUTABLE` environment variable. Defaults to auto-discovery or specific paths.
- **Testing**:
    - `tests/basic_crawl.rs`: Integration tests for basic crawling and markdown generation.
    - `tests/session_test.rs`: Verification of session reuse logic.
    - `tests/wait_test.rs`: Verification of smart waiting strategies.
    - `tests/content_filter_test.rs`: Unit tests for `BM25ContentFilter` verification.
    - `tests/nesting_test.rs`: Unit tests for verifying correct handling of nested block elements and mixed content in `BM25ContentFilter`.

## Architecture
- **Language**: Rust (2021 edition).
- **Async Runtime**: `tokio`.
- **Browser Automation**: `chromiumoxide` (Chrome DevTools Protocol).
- **DOM Manipulation**: `kuchiki` (based on `html5ever`).
- **HTTP Client**: `reqwest` (currently unused in core logic but added as dependency).
- **Serialization**: `serde`, `serde_json`.
- **HTML Processing**: `html2text`.
- **Stemming**: `rust-stemmers`.

## Setup & Usage
1.  **Dependencies**: Ensure Google Chrome or Chromium is installed.
2.  **Environment**: Set `CHROME_EXECUTABLE` if Chrome is in a non-standard location.
    ```bash
    export CHROME_EXECUTABLE=/usr/bin/google-chrome
    ```
    *Note: In the development sandbox, use `python -m playwright install --with-deps chromium` to install. The executable is typically at `~/.cache/ms-playwright/...`.*
3.  **Running Tests**:
    ```bash
    cargo test
    ```
    *Note: Browser tests should often be run sequentially to avoid resource contention:*
    ```bash
    cargo test -- --test-threads=1
    ```
4.  **Docker**:
    ```bash
    docker build -t crawl-4ai-rs .
    docker run crawl-4ai-rs
    ```

## Changes Made (This Session)
- **Error Handling**: Modified `crawler.rs` to include a retry loop in `arun`. Added logic to restart the browser if a critical error occurs. Refactored `start()` to handle restarts.
- **Docker**: Created `Dockerfile`.
- **Text Extraction**: Refined `BM25ContentFilter`'s `extract_text_chunks` to create new text nodes for chunks derived from mixed-content containers, fixing an issue where the entire container (including unrelated children) was being serialized.
- **Testing**: Added `tests/nesting_test.rs` to verify the fix.

## Next Steps for the Next Agent
1.  **Full Feature Parity**:
    - Implement CSS selector based extraction strategies (like `CssExtractionStrategy` in Python).
    - Add proxy support.
    - Add user-agent rotation or configuration.
2.  **Performance Tuning**: Review BM25 calculation performance for large pages.
3.  **API Polish**: The `arun` method signature and `CrawlResult` structure are stabilizing, but consider if `ExtractionResult` should be part of the public API in `models.rs` instead of internal to `crawler.rs`.
4.  **Integration**: If this is to be used as a service, consider adding a simple HTTP server (e.g. `axum` or `actix-web`) wrapping the crawler.

## Technical Notes
- **Browser Sandbox**: The crawler is configured with `--no-sandbox`, `--disable-gpu`, etc.
- **Chromium Compatibility**: `chromiumoxide` 0.5.7 is used.
