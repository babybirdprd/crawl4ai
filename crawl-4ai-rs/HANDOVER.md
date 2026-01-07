# crawl-4ai-rs Handover Document

## Project Overview
`crawl-4ai-rs` is a Rust port of the Python `crawl4ai` library. It aims to provide a robust, standalone web crawler and scraper with Markdown generation capabilities, suitable for LLM workflows.

## Current State
- **Core Crawler**: Implemented `AsyncWebCrawler` using `chromiumoxide` for headless browser automation.
- **Models**: Ported `CrawlResult` and `MarkdownGenerationResult` structs. Added `CrawlerRunConfig` for passing configuration options.
- **Session Management**: Implemented basic session management. `AsyncWebCrawler` now maintains a map of session IDs to `BrowserContextId`, allowing reuse of browser contexts (cookies, local storage, etc.) across crawls.
- **Media & Link Extraction**: Implemented logic in `arun` to extract images and links from the DOM using JavaScript injection.
- **Content Filtering**: Implemented `PruningContentFilter` (using `kuchiki`) to prune the DOM before markdown conversion.
- **Markdown Generation**: Updated `DefaultMarkdownGenerator` to use `PruningContentFilter`.
- **Browser Configuration**: Configurable via `CHROME_EXECUTABLE` environment variable. Defaults to auto-discovery or specific paths (/usr/bin/google-chrome-stable, /usr/bin/chromium).
- **Testing**:
    - `tests/basic_crawl.rs`: Integration tests for basic crawling and markdown generation.
    - `tests/session_test.rs`: Verification of session reuse logic.

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
    *Note: In some environments (like the development sandbox), you might need to point to a specific chromium binary if auto-discovery fails.*
3.  **Running Tests**:
    ```bash
    cargo test
    ```

## Changes Made (Session Management)
- Added `CrawlerRunConfig` struct to `src/models.rs` with `session_id`.
- Modified `AsyncWebCrawler` in `src/crawler.rs`:
    - Added `sessions: HashMap<String, BrowserContextId>`.
    - Updated `arun` to accept `Option<CrawlerRunConfig>`.
    - Implemented logic to create a new `BrowserContext` if a `session_id` is provided and doesn't exist, or reuse an existing one.
- Updated `tests/basic_crawl.rs` to match the new `arun` signature.
- Added `tests/session_test.rs` to test session ID usage.

## Next Steps for the Next Agent
1.  **Smart Waiting**: Improve the `wait_for_navigation` logic. The current implementation uses a simple `wait_for_navigation`, but handling dynamic content loading (waiting for network idle, specific selectors, or JavaScript conditions) is needed for more robust crawling.
2.  **Error Handling**: Enhance error mapping and retries. The "oneshot canceled" error from `chromiumoxide` can still happen if the browser crashes or disconnects. Robust retry logic and clearer error messages are needed.
3.  **Docker Support**: Add a Dockerfile for easy deployment.
4.  **BM25 Content Filter**: Implement the `BM25ContentFilter` strategy.
5.  **Full Feature Parity**: Continue porting features from Python (e.g., specific extraction strategies, proxies, more detailed config options).

## Technical Notes
- **Browser Sandbox**: The crawler is currently configured with `--no-sandbox` to run in containerized environments.
- **Flakiness**: Browser automation tests can be flaky. If you encounter "oneshot canceled", retry the test. Ensuring the correct `CHROME_EXECUTABLE` path is set is crucial.
