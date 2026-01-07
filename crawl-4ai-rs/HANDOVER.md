# crawl-4ai-rs Handover Document

## Project Overview
`crawl-4ai-rs` is a Rust port of the Python `crawl4ai` library. It aims to provide a robust, standalone web crawler and scraper with Markdown generation capabilities, suitable for LLM workflows.

## Current State
- **Core Crawler**: Implemented `AsyncWebCrawler` using `chromiumoxide` for headless browser automation.
- **Error Handling & Retry**:
    - Implemented `CrawlerError` enum using `thiserror`.
    - Implemented robust retry logic in `arun` (max 3 retries) to handle "oneshot canceled" and other browser communication errors.
    - Automatic browser restart and session invalidation on fatal errors.
- **Models**: Ported `CrawlResult` and `MarkdownGenerationResult` structs. Added `CrawlerRunConfig` for passing configuration options.
- **Session Management**: Implemented basic session management. `AsyncWebCrawler` now maintains a map of session IDs to `BrowserContextId`.
- **Smart Waiting**: Implemented `WaitStrategy` enum (`Fixed`, `Selector`, `JsCondition`) and updated `AsyncWebCrawler` to support waiting for specific conditions.
- **Media & Link Extraction**: Implemented logic in `arun` to extract images and links from the DOM using JavaScript injection.
- **Content Filtering**:
    - `PruningContentFilter` (DOM pruning).
    - `BM25ContentFilter` (Text ranking and filtering).
- **Markdown Generation**: Updated `DefaultMarkdownGenerator` to use the `ContentFilter` enum.
- **Browser Configuration**: Configurable via `CHROME_EXECUTABLE` environment variable.

## Setup & Usage
1.  **Dependencies**: Ensure Google Chrome or Chromium is installed.
2.  **Environment**: Set `CHROME_EXECUTABLE` if Chrome is in a non-standard location.
    ```bash
    export CHROME_EXECUTABLE=/usr/bin/google-chrome
    ```
3.  **Running Tests**:
    ```bash
    cargo test -- --test-threads=1
    ```
    *Note: Sequential execution is recommended to avoid browser resource contention.*

## Recent Changes (Error Handling)
- Refactored `src/crawler.rs` to include `CrawlerError`.
- Updated `arun` to wrap crawl logic in a retry loop.
- Added logic to clone browser handle (to avoid `&mut self` conflicts) and handle session creation within the loop.
- Ensured `self.sessions` is cleared if the browser crashes and restarts, as `BrowserContextId`s become invalid.

## Next Steps for the Next Agent
1.  **Refine Text Chunk Extraction**: The current `extract_text_chunks` implementation in `BM25ContentFilter` works but could be optimized to better handle nested block elements and spacing, matching Python's `deque` based approach more closely if edge cases arise.
2.  **Full Feature Parity**: Continue porting features from Python (e.g., specific extraction strategies, proxies, more detailed config options).
3.  **Performance Tuning**: Review BM25 calculation performance for large pages.
4.  **Unit Tests for Retry Logic**: While manual verification and basic tests pass, adding specific unit tests that mock browser failures (if possible with `chromiumoxide` or via a facade) would ensure long-term stability of the retry mechanism.

## Technical Notes
- **Borrow Checker**: The `arun` method uses a pattern where `self.browser` is accessed immutably (via a reference) while `self.sessions` is accessed mutably. This works because of disjoint field borrowing, but care must be taken when modifying this code.
- **Chromiumoxide**: `Browser` is not `Clone`, so we pass references.
