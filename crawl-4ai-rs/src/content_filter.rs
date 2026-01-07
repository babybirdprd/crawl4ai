use kuchiki::traits::*;
use kuchiki::NodeRef;
use std::collections::{HashSet, HashMap};
use serde::{Deserialize, Serialize};
use rust_stemmers::{Algorithm, Stemmer};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentFilter {
    Pruning(PruningContentFilter),
    BM25(BM25ContentFilter),
}

impl Default for ContentFilter {
    fn default() -> Self {
        ContentFilter::Pruning(PruningContentFilter::default())
    }
}

impl ContentFilter {
    pub fn filter_content(&self, html: &str) -> String {
        match self {
            ContentFilter::Pruning(f) => f.filter_content(html),
            ContentFilter::BM25(f) => f.filter_content(html),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruningContentFilter {
    pub threshold: f32,
    pub threshold_type: String,
    pub min_word_threshold: Option<usize>,
    pub excluded_tags: HashSet<String>,
    pub tag_weights: HashMap<String, f32>,
}

impl Default for PruningContentFilter {
    fn default() -> Self {
        Self::new(None, "fixed", 0.48)
    }
}

impl PruningContentFilter {
    pub fn new(min_word_threshold: Option<usize>, threshold_type: &str, threshold: f32) -> Self {
        let excluded_tags: HashSet<String> = [
            "nav", "footer", "header", "aside", "script", "style",
            "form", "iframe", "noscript"
        ].iter().map(|s| s.to_string()).collect();

        let mut tag_weights = HashMap::new();
        tag_weights.insert("div".to_string(), 0.5);
        tag_weights.insert("p".to_string(), 1.0);
        tag_weights.insert("article".to_string(), 1.5);
        tag_weights.insert("section".to_string(), 1.0);
        tag_weights.insert("span".to_string(), 0.3);
        tag_weights.insert("li".to_string(), 0.5);
        tag_weights.insert("ul".to_string(), 0.5);
        tag_weights.insert("ol".to_string(), 0.5);
        tag_weights.insert("h1".to_string(), 1.2);
        tag_weights.insert("h2".to_string(), 1.1);
        tag_weights.insert("h3".to_string(), 1.0);
        tag_weights.insert("h4".to_string(), 0.9);
        tag_weights.insert("h5".to_string(), 0.8);
        tag_weights.insert("h6".to_string(), 0.7);

        Self {
            threshold,
            threshold_type: threshold_type.to_string(),
            min_word_threshold,
            excluded_tags,
            tag_weights,
        }
    }

    pub fn filter_content(&self, html: &str) -> String {
        let document = kuchiki::parse_html().one(html);

        // Remove comments
        self.remove_comments(&document);

        // Remove unwanted tags
        self.remove_unwanted_tags(&document);

        // Prune tree
        if let Ok(body) = document.select_first("body") {
            self.prune_tree(body.as_node());
        } else {
             // Fallback if no body tag, prune root
            self.prune_tree(&document);
        }

        // Serialize back to HTML string
        let mut bytes = vec![];
        if let Ok(body) = document.select_first("body") {
             // If we found body, serialize its children
             for child in body.as_node().children() {
                 let _ = child.serialize(&mut bytes);
             }
        } else {
            let _ = document.serialize(&mut bytes);
        }

        String::from_utf8_lossy(&bytes).to_string()
    }

    fn remove_comments(&self, node: &NodeRef) {
        let comments: Vec<_> = node.descendants().filter(|n| n.as_comment().is_some()).collect();
        for child in comments {
             child.detach();
        }
    }

    fn remove_unwanted_tags(&self, node: &NodeRef) {
        for tag in &self.excluded_tags {
            if let Ok(selection) = node.select(tag) {
                for element in selection {
                    element.as_node().detach();
                }
            }
        }
    }

    fn prune_tree(&self, node: &NodeRef) {
        let children: Vec<NodeRef> = node.children().collect();
        for child in children {
            if let Some(element) = child.as_element() {
                let tag_name = element.name.local.to_string();

                let text_content = child.text_contents();
                let text_len = text_content.trim().len();

                let mut bytes = vec![];
                let _ = child.serialize(&mut bytes);
                let tag_len = bytes.len();

                let link_text_len = self.calculate_link_text_len(&child);

                let score = self.compute_score(&tag_name, text_len, tag_len, link_text_len);

                let should_remove = score < self.threshold;

                if let Some(min_word) = self.min_word_threshold {
                    let word_count = text_content.split_whitespace().count();
                    if word_count < min_word {
                        child.detach();
                        continue;
                    }
                }

                if should_remove {
                    child.detach();
                } else {
                    self.prune_tree(&child);
                }
            } else if child.as_text().is_some() {
                // Keep text nodes usually
            } else {
                 self.prune_tree(&child);
            }
        }
    }

    fn calculate_link_text_len(&self, node: &NodeRef) -> usize {
        let mut len = 0;
        if let Ok(links) = node.select("a") {
            for link in links {
                 len += link.text_contents().trim().len();
            }
        }
        len
    }

    fn compute_score(&self, tag_name: &str, text_len: usize, tag_len: usize, link_text_len: usize) -> f32 {
        let mut score = 0.0;
        let mut total_weight = 0.0;

        let w_text_density = 0.4;
        let w_link_density = 0.2;
        let w_tag_weight = 0.2;
        let w_text_length = 0.1;

        let density = if tag_len > 0 { text_len as f32 / tag_len as f32 } else { 0.0 };
        score += w_text_density * density;
        total_weight += w_text_density;

        let link_density = if text_len > 0 {
            1.0 - (link_text_len as f32 / text_len as f32)
        } else {
            0.0
        };
        score += w_link_density * link_density;
        total_weight += w_link_density;

        let tag_score = *self.tag_weights.get(tag_name).unwrap_or(&0.5);
        score += w_tag_weight * tag_score;
        total_weight += w_tag_weight;

        let len_score = ((text_len + 1) as f32).ln();
        score += w_text_length * len_score;
        total_weight += w_text_length;

        if total_weight > 0.0 {
            score / total_weight
        } else {
            0.0
        }
    }
}

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

    pub fn filter_content(&self, html: &str) -> String {
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
        let mut block_stack: Vec<(String, NodeRef)> = Vec::new();

        let inline_tags: HashSet<&str> = [
            "a", "abbr", "acronym", "b", "bdo", "big", "br", "button", "cite", "code", "dfn", "em", "i", "img", "input", "kbd", "label", "map", "object", "q", "samp", "script", "select", "small", "span", "strong", "sub", "sup", "textarea", "time", "tt", "var"
        ].iter().cloned().collect();

        for edge in body.traverse() {
             match edge {
                 kuchiki::iter::NodeEdge::Start(node) => {
                     if let Some(elem) = node.as_element() {
                         let tag_name = elem.name.local.to_string();
                         if !inline_tags.contains(tag_name.as_str()) {
                             // Block element start
                             // Flush text for PREVIOUS block
                             if !current_text.is_empty() {
                                 if let Some((prev_tag, prev_node)) = block_stack.last() {
                                     self.flush_chunk(&mut chunks, &mut index, &mut current_text, prev_tag, prev_node, &inline_tags);
                                 }
                             }
                             block_stack.push((tag_name, node.clone()));
                         }
                     } else if let Some(text) = node.as_text() {
                         let t = text.borrow();
                         if !t.trim().is_empty() {
                             current_text.push(t.to_string());
                         }
                     }
                 },
                 kuchiki::iter::NodeEdge::End(node) => {
                     if let Some(elem) = node.as_element() {
                         let tag_name = elem.name.local.to_string();
                         if !inline_tags.contains(tag_name.as_str()) {
                             // Block element end.
                             // Flush text for THIS block
                             if !current_text.is_empty() {
                                  if let Some((stack_tag, stack_node)) = block_stack.last() {
                                     self.flush_chunk(&mut chunks, &mut index, &mut current_text, stack_tag, stack_node, &inline_tags);
                                 }
                             }
                             block_stack.pop();
                         }
                     }
                 }
             }
        }

        chunks
    }

    fn flush_chunk(&self, chunks: &mut Vec<(usize, String, String, NodeRef)>, index: &mut usize, current_text: &mut Vec<String>, tag_name: &str, node: &NodeRef, inline_tags: &HashSet<&str>) {
        let text = current_text.join(" ");
        let text = text.trim();
        if !text.is_empty() {
            let word_count = text.split_whitespace().count();
            let min_words = self.min_word_threshold.unwrap_or(1);
            if word_count >= min_words {
                 // Check if node has block children
                 let mut has_block_children = false;
                 for child in node.children() {
                     if let Some(elem) = child.as_element() {
                         let child_tag = elem.name.local.to_string();
                         if !inline_tags.contains(child_tag.as_str()) {
                             has_block_children = true;
                             break;
                         }
                     }
                 }

                 let chunk_node = if has_block_children {
                     // Create a text node to represent this chunk, avoiding duplication of children
                     NodeRef::new_text(text)
                 } else {
                     node.clone()
                 };

                 chunks.push((*index, text.to_string(), tag_name.to_string(), chunk_node));
                 *index += 1;
            }
        }
        current_text.clear();
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
