use kuchiki::traits::*;
use kuchiki::NodeRef;
use std::collections::{HashSet, HashMap};
use serde::{Deserialize, Serialize};
use rust_stemmers::{Algorithm, Stemmer};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BM25ContentFilter {
    pub user_query: Option<String>,
    pub bm25_threshold: f32,
    pub language: String,
    pub use_stemming: bool,
    pub min_word_threshold: Option<usize>,
}

impl Default for BM25ContentFilter {
    fn default() -> Self {
        Self {
            user_query: None,
            bm25_threshold: 1.0,
            language: "english".to_string(),
            use_stemming: true,
            min_word_threshold: None,
        }
    }
}

impl BM25ContentFilter {
    pub fn new(user_query: Option<String>, bm25_threshold: f32) -> Self {
        Self {
            user_query,
            bm25_threshold,
            ..Default::default()
        }
    }

    pub async fn filter_content(&self, html: &str) -> String {
        let document = kuchiki::parse_html().one(html);

        let body = if let Ok(b) = document.select_first("body") {
            b.as_node().clone()
        } else {
            document.clone()
        };

        // Extract query if missing
        let query = if let Some(q) = &self.user_query {
            q.clone()
        } else {
            self.extract_page_query(&document, &body)
        };

        if query.is_empty() {
             return "".to_string();
        }

        let candidates = self.extract_text_chunks(&body);
        if candidates.is_empty() {
            return "".to_string();
        }

        let stemmer = if self.use_stemming {
            Some(Stemmer::create(Algorithm::English))
        } else {
            None
        };

        let tokenize = |text: &str| -> Vec<String> {
            let tokens = text.to_lowercase()
                .split(|c: char| !c.is_alphanumeric())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect::<Vec<_>>();

            if let Some(s) = &stemmer {
                tokens.into_iter().map(|t| s.stem(&t).to_string()).collect()
            } else {
                tokens
            }
        };

        let tokenized_query = tokenize(&query);
        let tokenized_corpus: Vec<Vec<String>> = candidates.iter()
            .map(|(_, text, _, _)| tokenize(text))
            .collect();

        // Calculate BM25 Scores
        let scores = self.calculate_bm25(&tokenized_corpus, &tokenized_query);

        // Adjust scores with tag weights
        let priority_tags: HashMap<&str, f32> = [
            ("h1", 5.0), ("h2", 4.0), ("h3", 3.0),
            ("title", 4.0), ("strong", 2.0), ("b", 1.5),
            ("em", 1.5), ("blockquote", 2.0), ("code", 2.0),
            ("pre", 1.5), ("th", 1.5)
        ].iter().cloned().collect();

        let mut adjusted_candidates = Vec::new();
        for (i, score) in scores.iter().enumerate() {
            let (_, _, tag_name, node) = &candidates[i];
            let weight = *priority_tags.get(tag_name.as_str()).unwrap_or(&1.0);
            let adjusted_score = score * weight;

            if adjusted_score >= self.bm25_threshold {
                adjusted_candidates.push((i, adjusted_score, node));
            }
        }

        // Sort by original index to preserve order
        adjusted_candidates.sort_by_key(|(i, _, _)| *i);

        let mut result_html = String::new();
        for (_, _, node) in adjusted_candidates {
            let mut bytes = vec![];
            let _ = node.serialize(&mut bytes);
            result_html.push_str(&String::from_utf8_lossy(&bytes));
        }

        result_html
    }

    fn extract_page_query(&self, document: &NodeRef, body: &NodeRef) -> String {
        let mut parts = Vec::new();

        // Title
        if let Ok(title) = document.select_first("title") {
            parts.push(title.text_contents());
        }

        // H1
        if let Ok(h1) = document.select_first("h1") {
            parts.push(h1.text_contents());
        }

        // Meta description/keywords
        if let Ok(metas) = document.select("meta") {
            for meta in metas {
                let attrs = meta.attributes.borrow();
                if let Some(name) = attrs.get("name") {
                    if name == "description" || name == "keywords" {
                        if let Some(content) = attrs.get("content") {
                            parts.push(content.to_string());
                        }
                    }
                }
            }
        }

        // Fallback: first long paragraph
        if parts.is_empty() {
             if let Ok(ps) = body.select("p") {
                 for p in ps {
                     let text = p.text_contents();
                     if text.len() > 150 {
                         parts.push(text.chars().take(150).collect());
                         break;
                     }
                 }
             }
        }

        parts.join(" ")
    }

    fn extract_text_chunks(&self, body: &NodeRef) -> Vec<(usize, String, String, NodeRef)> {
        let mut chunks = Vec::new();
        let mut index = 0;
        let mut current_text = Vec::new();

        let inline_tags: HashSet<&str> = [
            "a", "abbr", "acronym", "b", "bdo", "big", "br", "button", "cite", "code", "dfn", "em", "i", "img", "input", "kbd", "label", "map", "object", "q", "samp", "script", "select", "small", "span", "strong", "sub", "sup", "textarea", "time", "tt", "var"
        ].iter().cloned().collect();

        let header_tags: HashSet<&str> = ["h1", "h2", "h3", "h4", "h5", "h6", "header"].iter().cloned().collect();

        for edge in body.traverse() {
             match edge {
                 kuchiki::iter::NodeEdge::Start(node) => {
                     if let Some(text) = node.as_text() {
                         let t = text.borrow();
                         if !t.trim().is_empty() {
                             current_text.push(t.trim().to_string());
                         }
                     }
                 },
                 kuchiki::iter::NodeEdge::End(node) => {
                     if let Some(elem) = node.as_element() {
                         let tag_name = elem.name.local.to_string();

                         let is_inline = inline_tags.contains(tag_name.as_str());
                         // Python: not (tag.name == "p" and len(current_text) == 0)
                         let should_break = !is_inline && !(tag_name == "p" && current_text.is_empty());

                         if should_break {
                             let text = current_text.join(" ");
                             let text = text.trim();

                             if !text.is_empty() {
                                 let tag_type = if header_tags.contains(tag_name.as_str()) {
                                     "header".to_string()
                                 } else {
                                     "content".to_string()
                                 };

                                 chunks.push((index, text.to_string(), tag_type, node.clone()));
                                 index += 1;
                                 current_text.clear();
                             }
                         }
                     }
                 }
             }
        }

        // Handle remaining text
        if !current_text.is_empty() {
             let text = current_text.join(" ");
             let text = text.trim();
             if !text.is_empty() {
                 chunks.push((index, text.to_string(), "content".to_string(), body.clone()));
             }
        }

         if let Some(min_words) = self.min_word_threshold {
            chunks.retain(|(_, text, _, _)| {
                text.split_whitespace().count() >= min_words
            });
         }

        chunks
    }

    fn calculate_bm25(&self, corpus: &[Vec<String>], query: &[String]) -> Vec<f32> {
        let n = corpus.len() as f32;
        if n == 0.0 { return vec![]; }
        let avgdl: f32 = corpus.iter().map(|d| d.len()).sum::<usize>() as f32 / n;

        let k1 = 1.5;
        let b = 0.75;

        let mut scores = vec![0.0; corpus.len()];

        for term in query {
            // Calculate IDF for term
            let doc_freq = corpus.iter().filter(|d| d.contains(term)).count() as f32;
            let idf = ((n - doc_freq + 0.5) / (doc_freq + 0.5) + 1.0).ln();

            for (i, doc) in corpus.iter().enumerate() {
                let term_freq = doc.iter().filter(|&t| t == term).count() as f32;
                let doc_len = doc.len() as f32;

                if term_freq > 0.0 {
                    let numerator = term_freq * (k1 + 1.0);
                    let denominator = term_freq + k1 * (1.0 - b + b * (doc_len / avgdl));
                    scores[i] += idf * (numerator / denominator);
                }
            }
        }

        scores
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kuchiki::traits::*;

    #[test]
    fn test_extract_text_chunks_nested() {
        let html = r#"
            <div>
                Text1
                <p>Text2</p>
                Text3
            </div>
        "#;
        let document = kuchiki::parse_html().one(html);
        let body = document.select_first("body").unwrap();

        let filter = BM25ContentFilter::default();
        let chunks = filter.extract_text_chunks(body.as_node());

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].1, "Text1 Text2");
        assert_eq!(chunks[0].2, "content"); // p is content

        assert_eq!(chunks[1].1, "Text3");
        assert_eq!(chunks[1].2, "content"); // div is content
    }

    #[test]
    fn test_extract_text_chunks_inline() {
        let html = r#"
            <p>Start <span>Middle</span> End</p>
        "#;
        let document = kuchiki::parse_html().one(html);
        let body = document.select_first("body").unwrap();

        let filter = BM25ContentFilter::default();
        let chunks = filter.extract_text_chunks(body.as_node());

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].1, "Start Middle End");
    }
}
