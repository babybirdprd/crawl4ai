use crawl_4ai_rs::crawler::AsyncWebCrawler;
use crawl_4ai_rs::models::CrawlerRunConfig;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_network_console_capture() {
    let mock_server = MockServer::start().await;

    let html_content = r#"
    <html>
        <body>
            <h1>Network Test</h1>
            <script>
                console.log("Test console message");
                // Trigger a network request
                fetch('/api/data');
            </script>
        </body>
    </html>
    "#;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html_content))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/data"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"foo": "bar"})))
        .mount(&mock_server)
        .await;

    let mut crawler = AsyncWebCrawler::new();
    let url = mock_server.uri();

    let config = CrawlerRunConfig {
        capture_network_requests: Some(true),
        capture_console_messages: Some(true),
        // Wait specifically for the JS to run
        wait_for: Some(crawl_4ai_rs::models::WaitStrategy::Fixed(1000)),
        ..Default::default()
    };

    let result = crawler.arun(&url, Some(config)).await;

    match &result {
        Ok(r) => {
            println!("Success: {}", r.url);

            // Verify Network Requests
            assert!(r.network_requests.is_some(), "Network requests should be captured");
            let requests = r.network_requests.as_ref().unwrap();
            println!("Captured {} network requests", requests.len());

            let found_api = requests.iter().any(|req| req.url.contains("/api/data"));
            assert!(found_api, "Should capture fetch request to /api/data");

            // Verify Console Messages
            assert!(r.console_messages.is_some(), "Console messages should be captured");
            let messages = r.console_messages.as_ref().unwrap();
            println!("Captured {} console messages", messages.len());

            let found_msg = messages.iter().any(|msg| msg.text.contains("Test console message"));
            assert!(found_msg, "Should capture specific console message");
        },
        Err(e) => {
            panic!("Crawl failed: {:?}", e);
        }
    }
}
