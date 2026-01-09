use crawl_4ai_rs::crawler::AsyncWebCrawler;
use crawl_4ai_rs::models::CrawlerRunConfig;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use std::time::Duration;

#[tokio::test]
async fn test_retry_on_timeout_recovery() {
    // 1. Start mock server
    let mock_server = MockServer::start().await;

    // 2. Define delays
    let delay_duration = Duration::from_millis(2000);
    let timeout_duration = 500; // ms

    // Mock 1: Fails with timeout (slow response) - First 1 attempt
    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(ResponseTemplate::new(200).set_delay(delay_duration))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Mock 2: Succeeds immediately - Subsequent attempts
    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html><body>Success</body></html>"))
        .mount(&mock_server)
        .await;

    // 3. Configure Crawler
    let mut crawler = AsyncWebCrawler::new();
    let config = CrawlerRunConfig {
        page_timeout: Some(timeout_duration),
        ..Default::default()
    };

    let url = format!("{}/slow", mock_server.uri());

    // 4. Run Crawler
    // It should:
    // Attempt 1: Timeout (Mock 1)
    // Attempt 2: Timeout (Mock 1)
    // Attempt 3: Success (Mock 2)
    let result = crawler.arun(&url, Some(config)).await;

    assert!(result.is_ok(), "Crawler should succeed after retries");
    let crawl_result = result.unwrap();
    assert!(crawl_result.html.contains("Success"));
}

#[tokio::test]
async fn test_retry_exhaustion() {
    // 1. Start mock server
    let mock_server = MockServer::start().await;

    // 2. Define delays
    let delay_duration = Duration::from_millis(2000);
    let timeout_duration = 500; // ms

    // Mock 1: Fails with timeout always
    Mock::given(method("GET"))
        .and(path("/always_slow"))
        .respond_with(ResponseTemplate::new(200).set_delay(delay_duration))
        .mount(&mock_server)
        .await;

    // 3. Configure Crawler
    let mut crawler = AsyncWebCrawler::new();
    let config = CrawlerRunConfig {
        page_timeout: Some(timeout_duration),
        ..Default::default()
    };

    let url = format!("{}/always_slow", mock_server.uri());

    // 4. Run Crawler
    // It should retry 3 times then fail
    let result = crawler.arun(&url, Some(config)).await;

    assert!(result.is_err(), "Crawler should fail after exhausting retries");
}
