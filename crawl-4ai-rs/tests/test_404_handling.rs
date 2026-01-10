use crawl_4ai_rs::crawler::{AsyncWebCrawler, CrawlerError};
use crawl_4ai_rs::models::CrawlerRunConfig;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_404_no_retry_default() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/not_found"))
        .respond_with(ResponseTemplate::new(404))
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
        // Allow flaky wiremock/chrome behavior if we can't detect 404 perfectly,
        // but ideally we should.
        // For now, let's print what we got.
        println!("Got error: {:?}", err);
        // Fail if it's not the expected error, unless we decide to handle generic errors too.
        panic!("Expected HttpStatusCode error, got: {:?}", err);
    }
}
