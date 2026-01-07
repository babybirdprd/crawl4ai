use crawl_4ai_rs::content_filter::{BM25ContentFilter, ContentFilter};

#[test]
fn test_nested_blocks_text_extraction() {
    let html = r#"
        <html>
            <head><title>Test Page</title></head>
            <body>
                <div>
                    <p>Outer Paragraph.</p>
                    <div id="nested">
                        <p>Inner Paragraph 1.</p>
                        <span>Inline text in nested div.</span>
                        <p>Inner Paragraph 2.</p>
                    </div>
                    <p>Footer Paragraph.</p>
                </div>
            </body>
        </html>
    "#;

    // Provide a query to ensure we don't return empty due to missing query extraction
    // Use low threshold to get everything
    let filter = BM25ContentFilter::new(Some("paragraph text".to_string()), 0.0);

    let result = filter.filter_content(html);

    println!("Result: {}", result);

    assert!(result.contains("Outer Paragraph."));
    assert!(result.contains("Inner Paragraph 1."));
    // "Inline text in nested div."
    assert!(result.contains("Inline text in nested div."));
    assert!(result.contains("Inner Paragraph 2."));

    // Verify no duplication
    let count = result.matches("Inner Paragraph 1.").count();
    assert_eq!(count, 1, "Should appear exactly once, but found {}", count);
}

#[test]
fn test_mixed_content_nesting() {
    let html = r#"
        <html>
        <head><title>Test Page</title></head>
        <body>
        <div>
            Direct text 1.
            <p>Paragraph 1.</p>
            Direct text 2.
            <div>
                 Nested Direct text.
            </div>
        </div>
        </body>
        </html>
    "#;

    let filter = BM25ContentFilter::new(Some("text paragraph".to_string()), 0.0);
    let result = filter.filter_content(html);

    println!("Result: {}", result);

    assert!(result.contains("Direct text 1."));
    assert!(result.contains("Paragraph 1."));
    assert!(result.contains("Direct text 2."));
    assert!(result.contains("Nested Direct text."));
}
