# crawl-4ai-rs Handover Document

## Project Overview
`crawl-4ai-rs` is a Rust port of the Python `crawl4ai` library. It aims to provide a robust, standalone web crawler and scraper with Markdown generation capabilities, suitable for LLM workflows.

## Current State
- **Core Crawler**: Implemented `AsyncWebCrawler` using `chromiumoxide`.
- **LLM Support**:
    -   Integrated `rig-core` crate for LLM interaction.
    -   Refactored `LLMContentFilter` to use `rig` providers (specifically OpenAI).
    -   Optimized `LLMContentFilter` to reuse the `rig` Agent across parallel chunks.
- **Error Handling & Retry**: Refactored `arun` to use a cleaner loop with explicit state management and timeout support.
- **Wait Strategies**:
    -   Implemented `XPath`, `Selector`, `JsCondition`, `NetworkIdle` wait strategies.
- **Testing**:
    -   Integration tests for retry logic (`tests/test_retry_integration.rs`).
    -   Integration tests for wait strategies (`tests/test_wait_strategies.rs`).
    -   **Unit Tests for LLM**: `tests/test_llm_filter.rs` exists but currently fails with 404 errors due to URL path mismatch between `rig` client defaults and `wiremock` expectations.

## Recent Changes
- **Implemented `rig` Integration**:
    -   Added `rig-core` to `Cargo.toml`.
    -   Refactored `src/content_filter/llm.rs` to use `rig::providers::openai::Client` and `rig::agent::Agent`.
    -   Ensured connection pooling/reuse by moving agent creation out of the parallel chunk processing loop.
    -   Made `process_chunk` and `perform_completion_with_backoff` generic over `CompletionModel` to support future providers easily.

## Known Issues
- **LLM Test Failures**: `tests/test_llm_filter.rs` fails because the `rig` OpenAI client appends `/chat/completions` (or similar) to the base URL, and `wiremock` is not matching the resulting path correctly against the mock. Debugging requires inspecting the exact URL `rig` generates.
- **Headless 404 Handling**: In the current test environment (headless Chromium + wiremock), returning a 404 with an empty body causes `chromiumoxide` to fail navigation with `net::ERR_HTTP_RESPONSE_CODE_FAILURE`.

## Next Steps for the Agent (CRITICAL)
1.  **Fix LLM Integration Tests**:
    -   Investigate the exact URL structure `rig` uses when a custom `base_url` is provided.
    -   Update `tests/test_llm_filter.rs` to match this expectation so tests pass.
    -   Consider enabling logging/tracing in tests to see the actual request URL.

2.  **Expand LLM Provider Support**:
    -   Currently, the code defaults to OpenAI and prints a warning for others.
    -   Implement proper switching logic in `LLMContentFilter::filter_content` to support other providers supported by `rig` (e.g., Anthropic, Cohere) based on the `provider` string.

3.  **Headless Shell vs Full Chrome**:
    -   Investigate if `chromiumoxide` can run with the lighter `headless_shell` binary.

4.  **Documentation**:
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
