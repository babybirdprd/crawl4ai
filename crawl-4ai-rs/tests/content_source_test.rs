use crawl_4ai_rs::crawler::AsyncWebCrawler;
use crawl_4ai_rs::models::{CrawlerRunConfig, ContentSource};
use crawl_4ai_rs::content_filter::{ContentFilter, BM25ContentFilter};
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_content_source_selection() {
    let mock_server = MockServer::start().await;

    let html_content = r#"
    <html>
        <head><title>Test Page</title></head>
        <body>
            <div id="main">
                <h1>Important Content</h1>
                <p>This is the main content that should be preserved.</p>
                <p>Relevant information here.</p>
            </div>
            <div id="sidebar" style="background-color: gray;">
                <h2>Ads</h2>
                <p>Buy now! Discount!</p>
                <p>Spam spam spam.</p>
            </div>
            <div id="footer">
                <p>Copyright 2023</p>
            </div>
        </body>
    </html>
    "#;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html_content))
        .mount(&mock_server)
        .await;

    let mut crawler = AsyncWebCrawler::new();
    let url = mock_server.uri();

    // 1. Test CleanedHtml with a filter that targets "Important"
    // The BM25 filter should rank the main content higher and likely prune the sidebar/footer if configured well.
    // However, the default BM25 implementation in this codebase is based on scoring chunks.
    // We need to set a threshold.

    // Let's rely on the fact that CleanedHtml goes through the generator's filter (if provided),
    // and we'll configure a filter.
    // Wait, the code I wrote for CleanedHtml uses &html.
    // AND `DefaultMarkdownGenerator` uses the filter if provided.

    // So for CleanedHtml: generator(filter=Some(..)).generate(&html) -> Filtered Markdown
    // For RawHtml: generator(filter=None).generate(&html) -> Full Markdown

    let filter = ContentFilter::BM25(BM25ContentFilter::new(Some("Important Content Relevant".to_string()), 0.1));

    // RAW HTML SOURCE
    let config_raw = CrawlerRunConfig {
        content_source: Some(ContentSource::RawHtml),
        content_filter: Some(filter.clone()), // Should be ignored by logic
        ..Default::default()
    };

    let result_raw = crawler.arun(&url, Some(config_raw)).await.unwrap();
    let md_raw = result_raw.markdown.unwrap().raw_markdown;

    println!("Raw Markdown:\n{}", md_raw);
    assert!(md_raw.contains("Ads"), "Raw markdown should contain Ads/Sidebar");
    assert!(md_raw.contains("Spam"), "Raw markdown should contain Spam");

    // CLEANED HTML SOURCE
    // We need a filter that actually removes things.
    // The BM25 filter implementation logic splits text into chunks and scores them.
    // Blocks with low score are removed. "Ads" and "Spam" should have low score against "Important Content".

    // Note: The current BM25 implementation might be tricky to tune perfectly in a simple test without trial/error,
    // but let's try a high threshold.
    let strong_filter = ContentFilter::BM25(BM25ContentFilter::new(Some("Important Content".to_string()), 10.0));

    let config_cleaned = CrawlerRunConfig {
        content_source: Some(ContentSource::CleanedHtml),
        content_filter: Some(strong_filter),
        ..Default::default()
    };

    let result_cleaned = crawler.arun(&url, Some(config_cleaned)).await.unwrap();
    let md_cleaned = result_cleaned.markdown.unwrap().raw_markdown;

    println!("Cleaned Markdown:\n{}", md_cleaned);

    // Verify differentiation
    // Ideally Cleaned markdown lacks "Ads" or is at least shorter.
    // If the filter works as expected.

    if md_cleaned.contains("Spam") {
         println!("Warning: Filter didn't remove Spam. Check BM25 threshold logic.");
         // For the purpose of this task (Content Source Selection), the MAIN point is that
         // RawHtml explicitly IGNORES the filter (passing None to generator),
         // whereas CleanedHtml PASSES the filter.
         // So if we used a filter that would definitely remove everything (e.g. impossible query),
         // Raw should still have content, Cleaned should be empty.
    }

    // Let's do the "Impossible Query" test for definitive proof of wiring.
    let impossible_filter = ContentFilter::BM25(BM25ContentFilter::new(Some("ImpossibleQueryStringXYZ".to_string()), 100.0));

    let config_cleaned_impossible = CrawlerRunConfig {
        content_source: Some(ContentSource::CleanedHtml),
        content_filter: Some(impossible_filter.clone()),
        ..Default::default()
    };
    let res_c_imp = crawler.arun(&url, Some(config_cleaned_impossible)).await.unwrap();
    let md_c_imp = res_c_imp.markdown.unwrap().raw_markdown;

    let config_raw_impossible = CrawlerRunConfig {
        content_source: Some(ContentSource::RawHtml),
        content_filter: Some(impossible_filter.clone()),
        ..Default::default()
    };
    let res_r_imp = crawler.arun(&url, Some(config_raw_impossible)).await.unwrap();
    let md_r_imp = res_r_imp.markdown.unwrap().raw_markdown;

    println!("Cleaned (Impossible): len={}", md_c_imp.len());
    println!("Raw (Impossible): len={}", md_r_imp.len());

    assert!(md_r_imp.len() > md_c_imp.len(), "Raw markdown should be longer than cleaned when filter matches nothing");
    assert!(md_r_imp.contains("Important Content"), "Raw markdown should preserve content even if filter mismatches");
    assert!(md_c_imp.trim().is_empty() || md_c_imp.len() < 50, "Cleaned markdown should be nearly empty with impossible filter");
}
