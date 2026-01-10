use crawl_4ai_rs::crawler::AsyncWebCrawler;
use crawl_4ai_rs::models::{CrawlerRunConfig, WaitStrategy};
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_wait_strategy_configuration() {
    let crawler = AsyncWebCrawler::new();
    // This test primarily checks that the configuration compiles and is accepted.
    // Functional verification of timing requires more complex setup (mock server delaying response).

    let config = CrawlerRunConfig {
        wait_for: Some(WaitStrategy::Fixed(10)),
        wait_timeout: Some(5000),
        ..Default::default()
    };

    // Just asserting the config structure is correct as per our changes
    if let Some(WaitStrategy::Fixed(ms)) = config.wait_for {
        assert_eq!(ms, 10);
    } else {
        panic!("Wait strategy should be Fixed");
    }

    assert_eq!(config.wait_timeout, Some(5000));
}

#[tokio::test]
async fn test_network_idle_configuration() {
    let config = CrawlerRunConfig {
        wait_for: Some(WaitStrategy::NetworkIdle { idle_time: Some(1000) }),
        wait_timeout: Some(30000),
        ..Default::default()
    };

    if let Some(WaitStrategy::NetworkIdle { idle_time }) = config.wait_for {
        assert_eq!(idle_time, Some(1000));
    } else {
        panic!("Wait strategy should be NetworkIdle");
    }
}

// Integration test with wiremock to verify timeout logic is respected (at least for Fixed/Selector)
#[tokio::test]
async fn test_wait_timeout_logic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html><body><div id='content'>Hello</div></body></html>"))
        .mount(&mock_server)
        .await;

    let mut crawler = AsyncWebCrawler::new();

    // 1. Test Fixed wait
    let start = std::time::Instant::now();
    let config = CrawlerRunConfig {
        wait_for: Some(WaitStrategy::Fixed(500)),
        wait_timeout: Some(2000),
        ..Default::default()
    };

    let result = crawler.arun(&mock_server.uri(), Some(config)).await;
    assert!(result.is_ok());
    assert!(start.elapsed() >= Duration::from_millis(500));

    // 2. Test Selector wait (success)
    let config = CrawlerRunConfig {
        wait_for: Some(WaitStrategy::Selector("#content".to_string())),
        wait_timeout: Some(2000),
        ..Default::default()
    };
    let result = crawler.arun(&mock_server.uri(), Some(config)).await;
    assert!(result.is_ok());

    // 3. Test Selector wait (timeout)
    // We expect this to fail or at least log timeout.
    // The current implementation of crawl_page logs timeout and breaks loop, but proceeds to extract content.
    // So the crawl itself should be successful, but the loop should exit after timeout.
    // To verify this strictly, we'd need to check logs or check if the element was supposedly "found" if it wasn't there.
    // But since `crawl_page` continues after timeout, we just check that it runs and returns.
    let config = CrawlerRunConfig {
        wait_for: Some(WaitStrategy::Selector("#nonexistent".to_string())),
        wait_timeout: Some(1000), // Short timeout
        ..Default::default()
    };
    let start_timeout = std::time::Instant::now();
    let result = crawler.arun(&mock_server.uri(), Some(config)).await;
    let elapsed = start_timeout.elapsed();

    assert!(result.is_ok());
    // It should have waited at least 1s (timeout)
    assert!(elapsed >= Duration::from_millis(1000));
    // And ideally not much longer (allowing for overhead)
    assert!(elapsed < Duration::from_millis(5000));
}
