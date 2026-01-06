# crawl-4ai-rs Handover Document

## Project Overview
`crawl-4ai-rs` is a Rust port of the Python `crawl4ai` library. It aims to provide a robust, standalone web crawler and scraper with Markdown generation capabilities, suitable for LLM workflows.

## Current State
- **Core Crawler**: Implemented `AsyncWebCrawler` using `chromiumoxide` for headless browser automation.
- **Models**: Ported `CrawlResult` and `MarkdownGenerationResult` structs from Python to Rust (using `serde`).
- **Markdown Generation**: Implemented a basic `DefaultMarkdownGenerator` using `html2text`.
- **Browser Configuration**: Configurable via `CHROME_EXECUTABLE` environment variable. Defaults to auto-discovery or a specific path if known.
- **Testing**: Basic integration tests (`tests/basic_crawl.rs`) are implemented and passing. Browser is configured with `--no-sandbox` for CI/sandbox environments.

## Architecture
- **Language**: Rust (2021 edition).
- **Async Runtime**: `tokio`.
- **Browser Automation**: `chromiumoxide` (Chrome DevTools Protocol).
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
│   └── markdown.rs     # Markdown generation logic
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
1.  **Media Extraction**: The `CrawlResult` struct has a `media` field, but it is currently always `None`. Implement logic in `arun` to extract images and other media from the page (using `chromiumoxide`'s DOM access).
2.  **Link Extraction**: Similarly, extract internal and external links to populate the `links` field.
3.  **Advanced Content Filtering**: Implement the `PruningContentFilter` or `BM25ContentFilter` strategies from the Python version to improve Markdown quality (fit markdown).
4.  **Session Management**: Add support for reusing browser contexts/sessions.
5.  **Smart Waiting**: Improve the `wait_for_navigation` logic to handle dynamic content loading (wait for network idle, selectors, etc.).
6.  **Error Handling**: Enhance error mapping and retries.
7.  **Docker Support**: Add a Dockerfile for easy deployment.

## Technical Notes
- **Browser Sandbox**: The crawler is currently configured with `--no-sandbox` to run in containerized environments. This should be made configurable for production use.
- **Flakiness**: Browser automation tests can be flaky ("oneshot canceled" errors). Consider adding retry logic or better process management if this persists.
