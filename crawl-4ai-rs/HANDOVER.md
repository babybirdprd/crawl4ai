# crawl-4ai-rs Handover Document

## Project Overview
`crawl-4ai-rs` is a Rust port of the Python `crawl4ai` library. It aims to provide a robust, standalone web crawler and scraper with Markdown generation capabilities, suitable for LLM workflows.

## Current State
- **Core Crawler**: Implemented `AsyncWebCrawler` using `chromiumoxide`.
- **Error Handling**: Implemented `CrawlerError` and robust retry logic (embedded in `arun`).
- **Content Filtering**:
    - `PruningContentFilter` (DOM pruning).
    - `BM25ContentFilter` (Text ranking).
    - `LLMContentFilter` (LLM-based filtering/summarization).
- **Extraction Strategies**:
    - `JsonCssExtractionStrategy`: CSS selector based extraction.
    - `RegexExtractionStrategy`: Regex based extraction (with caching).
    - `JsonXPathExtractionStrategy`: XPath based extraction.
    - **CLI Support**: Added `--extraction-config` to pass strategies via JSON file.
- **Markdown Generation**: `async` implementation.
- **Session Management**: Implemented.
- **CLI**: Implemented in `src/main.rs`.
- **Documentation**: Added doc comments to `src/crawler.rs`, `src/models.rs`, and `src/extraction_strategy.rs`.

## Recent Changes
- **CLI Extraction Strategy**: Added support for running extraction strategies via the CLI using `--extraction-config`.
- **Regex Strategy Optimization**: Optimized `RegexExtractionStrategy` to cache compiled regexes.
- **Integration**: `AsyncWebCrawler` now executes the configured extraction strategy and returns the result in `extracted_content`.

## Next Steps for the Next Agent (The "Heavy" Tasks)
1.  **Refactor Retry Logic**:
    -   The retry logic is currently embedded in the `arun` loop. Extracting it into a testable, generic policy (like `retry_with_backoff`) is highly desired to allow unit testing without spinning up a full browser. A previous attempt was made but reverted due to complexity in `arun`.
2.  **Advanced Retry Testing**:
    -   Implement integration tests that simulate network failures (e.g., using a proxy or mock server that drops connections) to verify the crawler recovers.
3.  **Performance Tuning**:
    -   Benchmark `JsonXPathExtractionStrategy` vs `JsonCssExtractionStrategy` on large DOMs.
    -   Analyze memory usage during long crawls.
4.  **Wait Strategy Improvements**:
    -   The current `WaitStrategy` implementation is basic. Consider adding more sophisticated waiting conditions (e.g., network idle).

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
