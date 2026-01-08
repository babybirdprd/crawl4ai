use crawl_4ai_rs::extraction_strategy::JsonXPathExtractionStrategy;
use serde_json::{json, Value};

#[test]
fn test_xpath_extraction_with_malformed_html() {
    // Note: <br> and <img> are not closed, and <input> is not closed.
    // Also missing <html> and <body> tags, just a fragment.
    let html = r#"
        <div class="product">
            <h2>Product 1</h2>
            <br>
            <img src="img1.jpg">
            <span class="price">$10</span>
            <input type="hidden" value="123">
        </div>
        <div class="product">
            <h2>Product 2</h2>
            <br>
            <img src="img2.jpg">
            <span class="price">$20</span>
        </div>
    "#;

    let schema = json!({
        "baseSelector": "//div[@class='product']",
        "fields": [
            {"name": "name", "selector": "h2", "type": "text"},
            {"name": "price", "selector": "span[@class='price']", "type": "text"}
        ]
    });

    let strategy = JsonXPathExtractionStrategy::new(schema);
    let results = strategy.extract(html);

    assert_eq!(results.len(), 2, "Should find 2 products");

    assert_eq!(results[0]["name"], "Product 1");
    assert_eq!(results[0]["price"], "$10");

    assert_eq!(results[1]["name"], "Product 2");
    assert_eq!(results[1]["price"], "$20");
}
