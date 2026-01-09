use serde::{Deserialize, Serialize};
use serde_json::Value;
use kuchiki::traits::*;
use kuchiki::NodeRef;
use kuchiki::NodeData;
use regex::Regex;
use std::collections::HashMap;
use sxd_document::Package;
use sxd_xpath::{evaluate_xpath, Value as XPathValue, Factory, Context};
use sxd_xpath::nodeset::Node as XPathNode;

/// A strategy for extracting structured data using CSS selectors.
///
/// This strategy accepts a JSON schema defining the base selector and fields to extract.
/// It supports nested fields, lists, and various data types (text, attribute, html, regex).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonCssExtractionStrategy {
    /// The extraction schema.
    pub schema: Value,
}

/// The schema used for defining extraction rules.
#[derive(Debug, Clone, Deserialize)]
pub struct ExtractionSchema {
    /// Optional name for the schema.
    pub name: Option<String>,
    /// The base CSS selector to identify the root elements for extraction.
    #[serde(rename = "baseSelector")]
    pub base_selector: String,
    /// Common fields to extract from the root element itself (optional).
    #[serde(rename = "baseFields")]
    pub base_fields: Option<Vec<Field>>,
    /// Fields to extract from within the base element.
    pub fields: Vec<Field>,
}

/// A single field to be extracted.
#[derive(Debug, Clone, Deserialize)]
pub struct Field {
    /// The name of the field in the output JSON.
    pub name: String,
    /// The CSS selector (or XPath) relative to the current context (optional).
    pub selector: Option<String>,
    /// The type of extraction ("text", "attribute", "html", "regex", "nested", "list").
    #[serde(rename = "type")]
    pub type_: String,
    /// The attribute name to extract if type is "attribute".
    pub attribute: Option<String>,
    /// Transformation to apply to the extracted value (e.g., "lowercase").
    pub transform: Option<String>,
    /// Nested fields if type is "nested" or "list".
    pub fields: Option<Vec<Field>>,
    /// Default value if extraction fails.
    pub default: Option<Value>,
    /// Regex pattern if type is "regex".
    pub pattern: Option<String>,
}

impl JsonCssExtractionStrategy {
    /// Creates a new `JsonCssExtractionStrategy` with the given schema.
    pub fn new(schema: Value) -> Self {
        Self { schema }
    }

    /// Extracts data from the provided HTML string.
    pub fn extract(&self, html: &str) -> Vec<Value> {
        let schema: ExtractionSchema = match serde_json::from_value(self.schema.clone()) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let document = kuchiki::parse_html().one(html);
        let mut results = Vec::new();

        if let Ok(base_elements) = document.select(&schema.base_selector) {
            for element in base_elements {
                let node = element.as_node();
                let mut item = serde_json::Map::new();

                if let Some(base_fields) = &schema.base_fields {
                    for field in base_fields {
                        if let Some(val) = self.extract_single_field(node, field) {
                            item.insert(field.name.clone(), val);
                        }
                    }
                }

                let field_data = self.extract_item(node, &schema.fields);
                if let Some(obj) = field_data.as_object() {
                    for (k, v) in obj {
                        item.insert(k.clone(), v.clone());
                    }
                }

                results.push(Value::Object(item));
            }
        }

        results
    }

    fn extract_item(&self, node: &NodeRef, fields: &[Field]) -> Value {
        let mut item = serde_json::Map::new();
        for field in fields {
            let value = self.extract_field(node, field);
             if !value.is_null() {
                item.insert(field.name.clone(), value);
            }
        }
        Value::Object(item)
    }

    fn extract_field(&self, node: &NodeRef, field: &Field) -> Value {
        match field.type_.as_str() {
             "nested" => {
                 if let Some(selector) = &field.selector {
                      if let Ok(mut selection) = node.select(selector) {
                          if let Some(child) = selection.next() {
                               if let Some(nested_fields) = &field.fields {
                                   return self.extract_item(child.as_node(), nested_fields);
                               }
                          }
                      }
                 }
                 Value::Null
             },
             "list" | "nested_list" => {
                  if let Some(selector) = &field.selector {
                      let mut list = Vec::new();
                      if let Ok(selection) = node.select(selector) {
                          for child in selection {
                              if let Some(nested_fields) = &field.fields {
                                  list.push(self.extract_item(child.as_node(), nested_fields));
                              }
                          }
                      }
                      return Value::Array(list);
                  }
                  Value::Null
             },
             _ => self.extract_single_field(node, field).unwrap_or(Value::Null)
        }
    }

    fn extract_single_field(&self, node: &NodeRef, field: &Field) -> Option<Value> {
        let target_node = if let Some(selector) = &field.selector {
            if let Ok(mut selection) = node.select(selector) {
                selection.next().map(|n| n.as_node().clone())
            } else {
                None
            }
        } else {
            Some(node.clone())
        };

        if let Some(n) = target_node {
             let val = match field.type_.as_str() {
                 "text" => Some(Value::String(n.text_contents().trim().to_string())),
                 "attribute" => {
                     if let Some(attr_name) = &field.attribute {
                         if let Some(element) = n.as_element() {
                             let attrs = element.attributes.borrow();
                             attrs.get(attr_name.as_str()).map(|v| Value::String(v.to_string()))
                         } else { None }
                     } else { None }
                 },
                 "html" => {
                     let mut bytes = vec![];
                     let _ = n.serialize(&mut bytes);
                     Some(Value::String(String::from_utf8_lossy(&bytes).to_string()))
                 },
                 "regex" => {
                    if let Some(pattern) = &field.pattern {
                        if let Ok(re) = Regex::new(pattern) {
                            let text = n.text_contents();
                            if let Some(caps) = re.captures(&text) {
                                // Return the first group if available, otherwise the match
                                let m = caps.get(1).or_else(|| caps.get(0)).map(|m| m.as_str().to_string());
                                m.map(Value::String)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                 },
                 _ => None
             };

             if let Some(transform) = &field.transform {
                 if let Some(Value::String(s)) = val {
                     match transform.as_str() {
                         "lowercase" => return Some(Value::String(s.to_lowercase())),
                         "uppercase" => return Some(Value::String(s.to_uppercase())),
                         _ => return Some(Value::String(s))
                     }
                 }
             }
             return val;
        }

        field.default.clone()
    }
}

/// A strategy for extracting structured data using XPath.
///
/// This strategy converts the HTML DOM to an XML-compatible DOM to allow
/// XPath queries. It is robust against malformed HTML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonXPathExtractionStrategy {
    /// The extraction schema.
    pub schema: Value,
}

impl JsonXPathExtractionStrategy {
    /// Creates a new `JsonXPathExtractionStrategy` with the given schema.
    pub fn new(schema: Value) -> Self {
        Self { schema }
    }

    /// Extracts data from the provided HTML string.
    pub fn extract(&self, html: &str) -> Vec<Value> {
        let document = kuchiki::parse_html().one(html);
        let package = Package::new();
        let doc = package.as_document();

        // Convert kuchiki DOM to sxd_document DOM
        convert_kuchiki_to_sxd(&document, &doc, None);

        let schema: ExtractionSchema = match serde_json::from_value(self.schema.clone()) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let mut results = Vec::new();

        if let Ok(val) = evaluate_xpath(&doc, &schema.base_selector) {
            if let XPathValue::Nodeset(nodes) = val {
                 for node in nodes.document_order() {
                      let mut item = serde_json::Map::new();

                      if let Some(base_fields) = &schema.base_fields {
                          for field in base_fields {
                              if let Some(val) = self.extract_single_field(node, field) {
                                  item.insert(field.name.clone(), val);
                              }
                          }
                      }

                      let field_data = self.extract_item(node, &schema.fields);
                      if let Some(obj) = field_data.as_object() {
                          for (k, v) in obj {
                              item.insert(k.clone(), v.clone());
                          }
                      }

                      results.push(Value::Object(item));
                 }
            }
        }

        results
    }

     fn extract_item<'d>(&self, node: XPathNode<'d>, fields: &[Field]) -> Value {
        let mut item = serde_json::Map::new();
        for field in fields {
            let value = self.extract_field(node, field);
             if !value.is_null() {
                item.insert(field.name.clone(), value);
            }
        }
        Value::Object(item)
    }

    fn extract_field<'d>(&self, node: XPathNode<'d>, field: &Field) -> Value {
        match field.type_.as_str() {
             "nested" => {
                 if let Some(selector) = &field.selector {
                      let factory = Factory::new();
                      let xpath = match factory.build(selector) {
                          Ok(Some(x)) => x,
                          _ => return Value::Null,
                      };
                      let context = Context::new();

                      if let Ok(val) = xpath.evaluate(&context, node) {
                          if let XPathValue::Nodeset(nodes) = val {
                              if let Some(child) = nodes.document_order().first() {
                                   if let Some(nested_fields) = &field.fields {
                                       return self.extract_item(*child, nested_fields);
                                   }
                              }
                          }
                      }
                 }
                 Value::Null
             },
             "list" | "nested_list" => {
                  if let Some(selector) = &field.selector {
                      let factory = Factory::new();
                      let xpath = match factory.build(selector) {
                          Ok(Some(x)) => x,
                          _ => return Value::Null,
                      };
                      let context = Context::new();

                      let mut list = Vec::new();
                       if let Ok(val) = xpath.evaluate(&context, node) {
                          if let XPathValue::Nodeset(nodes) = val {
                              for child in nodes.document_order() {
                                  if let Some(nested_fields) = &field.fields {
                                      list.push(self.extract_item(child, nested_fields));
                                  }
                              }
                          }
                      }
                      return Value::Array(list);
                  }
                  Value::Null
             },
             _ => self.extract_single_field(node, field).unwrap_or(Value::Null)
        }
    }

     fn extract_single_field<'d>(&self, node: XPathNode<'d>, field: &Field) -> Option<Value> {
        let target_node = if let Some(selector) = &field.selector {
            let factory = Factory::new();
            if let Ok(Some(xpath)) = factory.build(selector) {
                let context = Context::new();
                if let Ok(val) = xpath.evaluate(&context, node) {
                     if let XPathValue::Nodeset(nodes) = val {
                         nodes.document_order().first().cloned()
                     } else {
                         None
                     }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            Some(node)
        };

        if let Some(n) = target_node {
             let val = match field.type_.as_str() {
                 "text" => Some(Value::String(n.string_value())),
                 "attribute" => {
                     if let Some(attr_name) = &field.attribute {
                         if let Some(elem) = n.element() {
                             elem.attribute_value(attr_name.as_str()).map(|v| Value::String(v.to_string()))
                         } else { None }
                     } else {
                         Some(Value::String(n.string_value()))
                     }
                 },
                 "html" => {
                     Some(Value::String(n.string_value()))
                 },
                 "regex" => {
                    if let Some(pattern) = &field.pattern {
                        if let Ok(re) = Regex::new(pattern) {
                            let text = n.string_value();
                            if let Some(caps) = re.captures(&text) {
                                let m = caps.get(1).or_else(|| caps.get(0)).map(|m| m.as_str().to_string());
                                m.map(Value::String)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                 },
                 _ => None
             };

             if let Some(transform) = &field.transform {
                 if let Some(Value::String(s)) = val {
                     match transform.as_str() {
                         "lowercase" => return Some(Value::String(s.to_lowercase())),
                         "uppercase" => return Some(Value::String(s.to_uppercase())),
                         _ => return Some(Value::String(s))
                     }
                 }
             }
             return val;
        }

        field.default.clone()
    }
}

fn convert_kuchiki_to_sxd(k_node: &NodeRef, s_doc: &sxd_document::dom::Document, s_parent: Option<sxd_document::dom::Element>) {
    for child in k_node.children() {
        match child.data() {
            NodeData::Element(data) => {
                let name = &data.name.local;
                let s_element = s_doc.create_element(name.as_ref());

                // Add attributes
                for (key, value) in &data.attributes.borrow().map {
                    s_element.set_attribute_value(key.local.as_ref(), &value.value);
                }

                if let Some(parent) = s_parent {
                    parent.append_child(s_element);
                } else {
                     s_doc.root().append_child(s_element);
                }

                convert_kuchiki_to_sxd(&child, s_doc, Some(s_element));
            },
            NodeData::Text(text) => {
                 if let Some(parent) = s_parent {
                     let s_text = s_doc.create_text(&text.borrow());
                     parent.append_child(s_text);
                 }
            },
            _ => {
                // For other node types (Comment, Doctype, etc.), we recurse but don't create sxd nodes
                // This ensures we traverse through Document node and others to find Elements
                convert_kuchiki_to_sxd(&child, s_doc, s_parent);
            }
        }
    }
}

/// A strategy for extracting entities using Regex patterns.
///
/// This strategy scans the text content of the page for common patterns
/// like emails, phone numbers, URLs, etc. It can be configured with custom patterns.
#[derive(Debug, Clone)]
pub struct RegexExtractionStrategy {
    patterns: HashMap<String, Regex>,
}

impl RegexExtractionStrategy {
    /// Creates a new `RegexExtractionStrategy` with default patterns.
    pub fn new() -> Self {
        Self::with_patterns(Self::default_patterns())
    }

    /// Creates a new `RegexExtractionStrategy` with custom patterns.
    pub fn with_patterns(patterns: Vec<(&str, &str)>) -> Self {
        let mut map = HashMap::new();
        for (name, pat) in patterns {
            if let Ok(re) = Regex::new(pat) {
                map.insert(name.to_string(), re);
            }
        }
        Self { patterns: map }
    }

    /// Returns a list of default regex patterns (email, url, phone, etc.).
    pub fn default_patterns() -> Vec<(&'static str, &'static str)> {
        vec![
            ("email", r"[\w.+-]+@[\w-]+\.[\w.-]+"),
            ("phone_intl", r"\+?\d[\d .()-]{7,}\d"),
            ("phone_us", r"\(?\d{3}\)?[ -. ]?\d{3}[ -. ]?\d{4}"),
            ("url", r"https?://[^\s\x22'<>]+"),
            ("ipv4", r"(?:\d{1,3}\.){3}\d{1,3}"),
            ("ipv6", r"[A-F0-9]{1,4}(?::[A-F0-9]{1,4}){7}"),
            ("uuid", r"[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}"),
            ("currency", r"(?:USD|EUR|RM|\$|€|£)\s?\d+(?:[.,]\d{2})?"),
            ("percentage", r"\d+(?:\.\d+)?%"),
            ("number", r"\b\d{1,3}(?:[,.\s]\d{3})*(?:\.\d+)?\b"),
            ("date_iso", r"\d{4}-\d{2}-\d{2}"),
            ("date_us", r"\d{1,2}/\d{1,2}/\d{2,4}"),
            ("time_24h", r"\b(?:[01]?\d|2[0-3]):[0-5]\d(?:[:.][0-5]\d)?\b"),
        ]
    }

    /// Extracts entities from the provided content string.
    pub fn extract(&self, url: &str, content: &str) -> Vec<Value> {
        let mut results = Vec::new();
        for (label, re) in &self.patterns {
            for cap in re.captures_iter(content) {
                 if let Some(m) = cap.get(0) {
                     results.push(serde_json::json!({
                         "url": url,
                         "label": label,
                         "value": m.as_str(),
                         "span": [m.start(), m.end()]
                     }));
                 }
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_css_extraction() {
        let html = r#"
        <html>
            <body>
                <div class="product">
                    <h2>Product 1</h2>
                    <span class="price">$10</span>
                </div>
                <div class="product">
                    <h2>Product 2</h2>
                    <span class="price">$20</span>
                </div>
            </body>
        </html>
        "#;

        let schema = json!({
            "baseSelector": ".product",
            "fields": [
                {"name": "name", "selector": "h2", "type": "text"},
                {"name": "price", "selector": ".price", "type": "text"}
            ]
        });

        let strategy = JsonCssExtractionStrategy::new(schema);
        let results = strategy.extract(html);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["name"], "Product 1");
        assert_eq!(results[0]["price"], "$10");
        assert_eq!(results[1]["name"], "Product 2");
        assert_eq!(results[1]["price"], "$20");
    }

    #[test]
    fn test_nested_extraction() {
        let html = r#"
        <div class="item">
            <div class="details">
                 <span class="info">Info</span>
            </div>
        </div>
        "#;
         let schema = json!({
            "baseSelector": ".item",
            "fields": [
                {
                    "name": "details",
                    "selector": ".details",
                    "type": "nested",
                    "fields": [
                        {"name": "info", "selector": ".info", "type": "text"}
                    ]
                }
            ]
        });
        let strategy = JsonCssExtractionStrategy::new(schema);
        let results = strategy.extract(html);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["details"]["info"], "Info");
    }

    #[test]
    fn test_regex_in_css_extraction() {
        let html = r#"
        <div class="content">
            <p>Order ID: #12345</p>
        </div>
        "#;
        let schema = json!({
            "baseSelector": ".content",
            "fields": [
                {
                    "name": "order_id",
                    "selector": "p",
                    "type": "regex",
                    "pattern": r"#(\d+)"
                }
            ]
        });
        let strategy = JsonCssExtractionStrategy::new(schema);
        let results = strategy.extract(html);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["order_id"], "12345");
    }

    #[test]
    fn test_regex_extraction_strategy() {
        let content = "Contact us at support@example.com or call 123-456-7890. Visit https://example.com";
        let strategy = RegexExtractionStrategy::new();
        let results = strategy.extract("http://page.com", content);

        let emails: Vec<&Value> = results.iter().filter(|v| v["label"] == "email").collect();
        let urls: Vec<&Value> = results.iter().filter(|v| v["label"] == "url").collect();

        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0]["value"], "support@example.com");

        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0]["value"], "https://example.com");
    }

    #[test]
    fn test_json_xpath_extraction() {
        let html = r#"
        <html>
            <body>
                <div class="product">
                    <h2>Product 1</h2>
                    <span class="price">$10</span>
                </div>
                <div class="product">
                    <h2>Product 2</h2>
                    <span class="price">$20</span>
                </div>
            </body>
        </html>
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

        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["name"], "Product 1");
        assert_eq!(results[0]["price"], "$10");
        assert_eq!(results[1]["name"], "Product 2");
        assert_eq!(results[1]["price"], "$20");
    }
}
