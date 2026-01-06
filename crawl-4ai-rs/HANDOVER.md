# crawl-4ai-rs Handover Document

## Project Overview
`crawl-4ai-rs` is a Rust port of the Python `crawl4ai` library. It aims to provide a robust, standalone web crawler and scraper with Markdown generation capabilities, suitable for LLM workflows.

## Current State
- **Core Crawler**: Implemented `AsyncWebCrawler` using `chromiumoxide` for headless browser automation.
- **Models**: Ported `CrawlResult` and `MarkdownGenerationResult` structs from Python to Rust (using `serde`).
- **Media & Link Extraction**: Implemented logic in `arun` to extract images and links from the DOM using JavaScript injection.
- **Content Filtering**: Implemented `PruningContentFilter` (using `kuchiki`) to prune the DOM before markdown conversion, improving relevance.
- **Markdown Generation**: Updated `DefaultMarkdownGenerator` to use `PruningContentFilter`.
- **Browser Configuration**: Configurable via `CHROME_EXECUTABLE` environment variable. Defaults to auto-discovery or specific paths (/usr/bin/google-chrome-stable, /usr/bin/chromium).
- **Testing**: Integration tests (`tests/basic_crawl.rs`) are implemented and passing. `tests/extraction_test.rs` was used for verification and can be re-created if needed.

## Architecture
- **Language**: Rust (2021 edition).
- **Async Runtime**: `tokio`.
- **Browser Automation**: `chromiumoxide` (Chrome DevTools Protocol).
- **DOM Manipulation**: `kuchiki` (based on `html5ever`).
- **HTTP Client**: `reqwest` (currently unused in core logic but added as dependency).
- **Serialization**: `serde`, `serde_json`.
- **HTML Processing**: `html2text`.

## Project Structure
```
crawl-4ai-rs/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Module exports
│   ├── crawler.rs      # AsyncWebCrawler implementation
│   ├── models.rs       # Data models (CrawlResult, etc.)
│   ├── markdown.rs     # Markdown generation logic
│   └── content_filter.rs # PruningContentFilter implementation
└── tests/
    └── basic_crawl.rs  # Integration tests
```

## Setup & Usage
1.  **Dependencies**: Ensure Google Chrome or Chromium is installed.
2.  **Environment**: Set `CHROME_EXECUTABLE` if Chrome is in a non-standard location.
    ```bash
    export CHROME_EXECUTABLE=/usr/bin/google-chrome
    ```
3.  **Running Tests**:
    ```bash
    cargo test
    ```

## Next Steps for the Next Agent
1.  **Session Management**: Add support for reusing browser contexts/sessions.
2.  **Smart Waiting**: Improve the `wait_for_navigation` logic to handle dynamic content loading (wait for network idle, selectors, etc.).
3.  **Error Handling**: Enhance error mapping and retries (currently flaky "oneshot canceled" errors happen).
4.  **Docker Support**: Add a Dockerfile for easy deployment.
5.  **BM25 Content Filter**: Implement the `BM25ContentFilter` strategy.
6.  **Full Feature Parity**: Continue porting features from Python (e.g., specific extraction strategies, proxies).

## Technical Notes
- **Browser Sandbox**: The crawler is currently configured with `--no-sandbox` to run in containerized environments.
- **Flakiness**: Browser automation tests are flaky ("oneshot canceled"). This is likely due to the ephemeral nature of the browser process or resource constraints. Retry logic or better lifecycle management is needed.
