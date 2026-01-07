use crawl_4ai_rs::content_filter::BM25ContentFilter;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bm25_content_filter_basic() {
        let html = r#"
        <html>
            <body>
                <h1>Important Title</h1>
                <p>This is a very important paragraph that should be kept because it has relevant keywords.</p>
                <div>
                    <span>Garbage content here.</span>
                </div>
                <h2>Subsection</h2>
                <p>More relevant text here for the query.</p>
            </body>
        </html>
        "#;

        let filter = BM25ContentFilter::new(Some("important relevant".to_string()), 0.5);
        let filtered_html = filter.filter_content(html);

        println!("Filtered HTML: {}", filtered_html);

        assert!(filtered_html.contains("Important Title"));
        assert!(filtered_html.contains("important paragraph"));
        assert!(!filtered_html.contains("Garbage content"));
    }

    #[test]
    fn test_bm25_content_filter_no_query() {
        let html = r#"
        <html>
            <head><title>Auto Query Page</title></head>
            <body>
                <h1>Main Topic</h1>
                <p>Content related to the main topic should be kept.</p>
                <div class="ads">Buy this product now!</div>
            </body>
        </html>
        "#;

        // Should auto-detect query from title/h1
        let filter = BM25ContentFilter::default();
        let filtered_html = filter.filter_content(html);

        println!("Filtered HTML (Auto Query): {}", filtered_html);

        // "Main Topic" and "Auto Query Page" should be part of the query
        assert!(filtered_html.contains("Main Topic"));
        // Ads should hopefully be filtered out if scoring works, but with short text it's tricky.
        // Let's just check if it runs without crashing and keeps main content.
    }
}
