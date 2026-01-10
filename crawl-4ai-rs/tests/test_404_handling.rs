use crawl_4ai_rs::crawler::{AsyncWebCrawler, CrawlerError};
use crawl_4ai_rs::models::CrawlerRunConfig;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_404_no_retry_default() {
    let mock_server = MockServer::start().await;

    // We set a body string because headless chrome might fail with a generic protocol error
    // if it encounters a 404 with no content, preventing us from inspecting the status code.
    // With a body, it should load the "error page" and let us see the 404 status.
    Mock::given(method("GET"))
        .and(path("/not_found"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found Page"))
        .expect(1) // Should be called exactly once
        .mount(&mock_server)
        .await;

    let mut crawler = AsyncWebCrawler::new();
    let url = format!("{}/not_found", mock_server.uri());

    let result = crawler.arun(&url, None).await;

    assert!(result.is_err(), "Crawler should fail on 404");
    let err = result.unwrap_err();

    // Check if error is HttpStatusCode(404)
    if let Some(CrawlerError::HttpStatusCode(code)) = err.downcast_ref::<CrawlerError>() {
        assert_eq!(*code, 404);
    } else {
        println!("Got error: {:?}", err);
        panic!("Expected HttpStatusCode error, got: {:?}", err);
    }
}

#[tokio::test]
async fn test_404_retry_enabled_exhaustion() {
    let mock_server = MockServer::start().await;

    // Mock always returns 404 with body
    Mock::given(method("GET"))
        .and(path("/not_found"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .expect(3) // Should be called 3 times (1 initial + 2 retries loop)
        .mount(&mock_server)
        .await;

    let mut crawler = AsyncWebCrawler::new();
    let config = CrawlerRunConfig {
        retry_404: true,
        ..Default::default()
    };
    let url = format!("{}/not_found", mock_server.uri());

    let result = crawler.arun(&url, Some(config)).await;

    assert!(result.is_err(), "Crawler should fail on 404 even with retry (eventually)");
}

#[tokio::test]
async fn test_404_retry_recovery() {
    let mock_server = MockServer::start().await;

    // First response 404
    Mock::given(method("GET"))
        .and(path("/flaky"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&mock_server)
        .await;

    // Second response 200
    Mock::given(method("GET"))
        .and(path("/flaky"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Success"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let mut crawler = AsyncWebCrawler::new();
    let config = CrawlerRunConfig {
        retry_404: true,
        ..Default::default()
    };
    let url = format!("{}/flaky", mock_server.uri());

    let result = crawler.arun(&url, Some(config)).await;

    assert!(result.is_ok(), "Crawler should recover from 404 if retry enabled");
    assert!(result.unwrap().html.contains("Success"));
}
