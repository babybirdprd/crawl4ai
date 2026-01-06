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

#[tokio::test]
async fn test_extraction() {
    let mut crawler = AsyncWebCrawler::new();
    // Use a page with known content or mock it if possible
    // For now, we will test with example.com
    let url = "https://example.com";
    let result = crawler.arun(url).await;

    assert!(result.is_ok());
    let crawl_result = result.unwrap();

    // Check media extraction
    // example.com might not have images, but the map should be present
    assert!(crawl_result.media.is_some());
    let media = crawl_result.media.as_ref().unwrap();
    assert!(media.contains_key("images"));

    // Check link extraction
    assert!(crawl_result.links.is_some());
    let links = crawl_result.links.as_ref().unwrap();
    assert!(links.contains_key("internal"));

    // example.com has a link "More information..."
    let internal_links = links.get("internal").unwrap();
    assert!(!internal_links.is_empty());

    // example.com has a link "More information..."
    // The text might differ slightly depending on browser rendering (whitespace etc)
    // "More information..."
    // example.com has a link "More information..."
    // The text might differ slightly depending on browser rendering (whitespace etc)
    // "More information..."
    // In the panic message, we see: [Link { href: Some("https://iana.org/domains/example"), text: Some("Learn more"), title: Some("") }]
    // It seems "More information..." is not the text, but "Learn more" is.
    let learn_more_link = internal_links.iter().find(|l|
        l.text.as_ref().map(|t| t.contains("Learn more")).unwrap_or(false)
    );
    assert!(learn_more_link.is_some(), "Could not find 'Learn more' link. Found: {:?}", internal_links);
}
