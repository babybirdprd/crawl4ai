use kuchiki::traits::*;
use kuchiki::NodeRef;
use std::collections::{HashSet, HashMap};
use serde::{Deserialize, Serialize};

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

    pub async fn filter_content(&self, html: &str) -> String {
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
