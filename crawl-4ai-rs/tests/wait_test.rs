use crawl_4ai_rs::crawler::AsyncWebCrawler;
use crawl_4ai_rs::models::{CrawlerRunConfig, WaitStrategy};
use tokio;
use std::time::Instant;

#[tokio::test]
async fn test_smart_waiting_fixed() {
    let mut crawler = AsyncWebCrawler::new();
    let config = CrawlerRunConfig {
        wait_for: Some(WaitStrategy::Fixed(1000)),
        ..Default::default()
    };

    // Use a blank page or simple page
    let url = "data:text/html,<html><body><h1>Hello</h1></body></html>";

    let start = Instant::now();
    let result = crawler.arun(url, Some(config)).await;
    let duration = start.elapsed();

    assert!(result.is_ok());
    assert!(duration.as_millis() >= 1000, "Should have waited at least 1000ms");
}

#[tokio::test]
async fn test_smart_waiting_selector() {
    let mut crawler = AsyncWebCrawler::new();
    let config = CrawlerRunConfig {
        wait_for: Some(WaitStrategy::Selector("#dynamic".to_string())),
        ..Default::default()
    };

    // Script adds element after 1 second
    let url = r#"data:text/html,
    <html>
        <body>
            <h1>Wait Test</h1>
            <script>
                setTimeout(() => {
                    const div = document.createElement('div');
                    div.id = 'dynamic';
                    div.innerText = 'Appeared!';
                    document.body.appendChild(div);
                }, 1000);
            </script>
        </body>
    </html>"#;

    let start = Instant::now();
    let result = crawler.arun(url, Some(config)).await;
    let duration = start.elapsed();

    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(res.html.contains("Appeared!"));
    assert!(duration.as_millis() >= 1000, "Should have waited for element");
}

#[tokio::test]
async fn test_smart_waiting_js() {
     let mut crawler = AsyncWebCrawler::new();
     let config = CrawlerRunConfig {
         wait_for: Some(WaitStrategy::JsCondition("document.body.getAttribute('data-loaded') === 'true'".to_string())),
         ..Default::default()
     };

     // Script sets attribute after 1 second
     let url = r#"data:text/html,
     <html>
         <body>
             <h1>Wait Test JS</h1>
             <script>
                 setTimeout(() => {
                     document.body.setAttribute('data-loaded', 'true');
                 }, 1000);
             </script>
         </body>
     </html>"#;

     let start = Instant::now();
     let result = crawler.arun(url, Some(config)).await;
     let duration = start.elapsed();

     assert!(result.is_ok());
     let res = result.unwrap();
     assert!(res.html.contains("data-loaded"));
     assert!(duration.as_millis() >= 1000, "Should have waited for JS condition");
}
