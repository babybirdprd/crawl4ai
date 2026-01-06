use crawl_4ai_rs::crawler::AsyncWebCrawler;

#[tokio::test]
async fn test_crawler_initialization() {
    let _crawler = AsyncWebCrawler::new();
    assert!(true);
}

#[tokio::test]
async fn test_crawl_html() {
    let mut crawler = AsyncWebCrawler::new();
    let url = "https://example.com";
    let result = crawler.arun(url).await;

    match &result {
        Ok(r) => println!("Success: {}", r.url),
        Err(e) => println!("Error: {:?}", e),
    }

    assert!(result.is_ok());
    let crawl_result = result.unwrap();
    assert_eq!(crawl_result.url, url);
    assert!(crawl_result.success);
    assert!(!crawl_result.html.is_empty());

    // Check markdown generation
    assert!(crawl_result.markdown.is_some());
    let md = crawl_result.markdown.unwrap();
    assert!(!md.raw_markdown.is_empty());
}
