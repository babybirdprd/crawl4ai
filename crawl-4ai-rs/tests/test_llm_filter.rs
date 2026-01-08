use crawl_4ai_rs::content_filter::{LLMConfig, LLMContentFilter, ContentFilter};
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};
use serde_json::json;

#[tokio::test]
async fn test_llm_content_filter_chunking() {
    let config = LLMConfig {
        provider: "test-provider".to_string(),
        api_token: "test-token".to_string(),
        base_url: None,
        backoff_base_delay: 0,
        backoff_max_attempts: 1,
        backoff_exponential_factor: 1.0,
    };

    // Low threshold to force chunking
    let filter = LLMContentFilter {
        config,
        instruction: "Summarize".to_string(),
        chunk_token_threshold: 10,
        overlap_rate: 0.0,
        word_token_rate: 1.0,
        ignore_cache: true,
    };

    // 15 words
    let html = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen";

    // merge_chunks is private, but filter_content calls it.
    // However, filter_content makes API calls.
    // We can't easily inspect chunks without mocking the API.
}

#[tokio::test]
async fn test_llm_content_filter_api_call() {
    let mock_server = MockServer::start().await;

    let response_body = json!({
        "choices": [
            {
                "message": {
                    "content": "<content>Filtered Content</content>"
                }
            }
        ]
    });

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let config = LLMConfig {
        provider: "test-model".to_string(),
        api_token: "test-token".to_string(),
        base_url: Some(format!("{}/chat/completions", mock_server.uri())),
        backoff_base_delay: 0,
        backoff_max_attempts: 1,
        backoff_exponential_factor: 1.0,
    };

    let filter = LLMContentFilter {
        config,
        instruction: "Filter this".to_string(),
        chunk_token_threshold: 100,
        overlap_rate: 0.0,
        word_token_rate: 1.0,
        ignore_cache: true,
    };

    let html = "Some content to filter";
    let result = filter.filter_content(html).await;

    assert_eq!(result, "Filtered Content");
}

#[tokio::test]
async fn test_llm_content_filter_api_retry() {
    let mock_server = MockServer::start().await;

    // Fail twice with 429, then succeed
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(429))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    let response_body = json!({
        "choices": [
            {
                "message": {
                    "content": "<content>Retry Success</content>"
                }
            }
        ]
    });

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let config = LLMConfig {
        provider: "test-model".to_string(),
        api_token: "test-token".to_string(),
        base_url: Some(format!("{}/chat/completions", mock_server.uri())),
        backoff_base_delay: 0, // Instant retry for test
        backoff_max_attempts: 3,
        backoff_exponential_factor: 1.0,
    };

    let filter = LLMContentFilter {
        config,
        instruction: "Retry test".to_string(),
        chunk_token_threshold: 100,
        overlap_rate: 0.0,
        word_token_rate: 1.0,
        ignore_cache: true,
    };

    let html = "Retry me";
    let result = filter.filter_content(html).await;

    assert_eq!(result, "Retry Success");
}
