use kuchiki::traits::*;
use kuchiki::NodeRef;
use std::collections::{HashSet, HashMap};
use serde::{Deserialize, Serialize};
use rust_stemmers::{Algorithm, Stemmer};
use reqwest::Client;
use std::time::Duration;
use futures::stream::{self, StreamExt};
use serde_json::Value;

const PROMPT_FILTER_CONTENT: &str = r#"Your task is to filter and convert HTML content into clean, focused markdown that's optimized for use with LLMs and information retrieval systems.

TASK DETAILS:
1. Content Selection
- DO: Keep essential information, main content, key details
- DO: Preserve hierarchical structure using markdown headers
- DO: Keep code blocks, tables, key lists
- DON'T: Include navigation menus, ads, footers, cookie notices
- DON'T: Keep social media widgets, sidebars, related content

2. Content Transformation
- DO: Use proper markdown syntax (#, ##, **, `, etc)
- DO: Convert tables to markdown tables
- DO: Preserve code formatting with ```language blocks
- DO: Maintain link texts but remove tracking parameters
- DON'T: Include HTML tags in output
- DON'T: Keep class names, ids, or other HTML attributes

3. Content Organization
- DO: Maintain logical flow of information
- DO: Group related content under appropriate headers
- DO: Use consistent header levels
- DON'T: Fragment related content
- DON'T: Duplicate information

IMPORTANT: If user specific instruction is provided, ignore above guideline and prioritize those requirements over these general guidelines.

OUTPUT FORMAT:
Wrap your response in <content> tags. Use proper markdown throughout.
<content>
[Your markdown content here]
</content>

Begin filtering now.

--------------------------------------------

<|HTML_CONTENT_START|>
{HTML}
<|HTML_CONTENT_END|>

<|USER_INSTRUCTION_START|>
{REQUEST}
<|USER_INSTRUCTION_END|>
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentFilter {
    Pruning(PruningContentFilter),
    BM25(BM25ContentFilter),
    LLM(LLMContentFilter),
}

impl Default for ContentFilter {
    fn default() -> Self {
        ContentFilter::Pruning(PruningContentFilter::default())
    }
}

impl ContentFilter {
    pub async fn filter_content(&self, html: &str) -> String {
        match self {
            ContentFilter::Pruning(f) => f.filter_content(html).await,
            ContentFilter::BM25(f) => f.filter_content(html).await,
            ContentFilter::LLM(f) => f.filter_content(html).await,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub provider: String,
    pub api_token: String,
    pub base_url: Option<String>,
    pub backoff_base_delay: u64,
    pub backoff_max_attempts: u32,
    pub backoff_exponential_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMContentFilter {
    pub config: LLMConfig,
    pub instruction: String,
    pub chunk_token_threshold: usize,
    pub overlap_rate: f32,
    pub word_token_rate: f32,
    pub ignore_cache: bool,
}

impl Default for LLMContentFilter {
    fn default() -> Self {
        // Warning: This default configuration is invalid because api_token is empty
        // It's provided to satisfy potential Default traits but should be instantiated with new()
        Self {
            config: LLMConfig {
                provider: "openai/gpt-4o-mini".to_string(),
                api_token: "".to_string(),
                base_url: None,
                backoff_base_delay: 2,
                backoff_max_attempts: 3,
                backoff_exponential_factor: 2.0,
            },
            instruction: "Convert this HTML into clean, relevant markdown, removing any noise or irrelevant content.".to_string(),
            chunk_token_threshold: 4096,
            overlap_rate: 0.1,
            word_token_rate: 0.75,
            ignore_cache: true,
        }
    }
}

impl LLMContentFilter {
    pub fn new(
        config: LLMConfig,
        instruction: Option<String>,
        chunk_token_threshold: Option<usize>,
        overlap_rate: Option<f32>,
    ) -> Self {
        Self {
            config,
            instruction: instruction.unwrap_or_else(|| {
                "Convert this HTML into clean, relevant markdown, removing any noise or irrelevant content.".to_string()
            }),
            chunk_token_threshold: chunk_token_threshold.unwrap_or(4096),
            overlap_rate: overlap_rate.unwrap_or(0.1),
            word_token_rate: 0.75,
            ignore_cache: true,
        }
    }

    pub async fn filter_content(&self, html: &str) -> String {
        // 1. Chunking
        let chunks = self.merge_chunks(html);

        let client = Client::new();

        // 2. Process chunks in parallel
        let tasks = chunks.into_iter().enumerate().map(|(i, chunk)| {
            let config = self.config.clone();
            let instruction = self.instruction.clone();
            let client = client.clone();
            async move {
                Self::process_chunk(client, i, chunk, config, instruction).await
            }
        });

        // Parallel execution with buffered stream
        // Using buffer_unordered to run 4 tasks concurrently
        let results: Vec<(usize, String)> = stream::iter(tasks)
            .buffer_unordered(4)
            .collect()
            .await;

        // 3. Sort and join
        let mut results = results;
        results.sort_by_key(|(i, _)| *i);

        results.into_iter().map(|(_, s)| s).collect::<Vec<_>>().join("\n\n")
    }

    fn merge_chunks(&self, text: &str) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return vec![];
        }

        let total_tokens_est = (words.len() as f32 * self.word_token_rate) as usize;

        // If small enough, return as one chunk
        if total_tokens_est <= self.chunk_token_threshold {
            return vec![text.to_string()];
        }

        // Calculate chunk size in words
        let chunk_size_words = (self.chunk_token_threshold as f32 / self.word_token_rate) as usize;
        let overlap_words = (chunk_size_words as f32 * self.overlap_rate) as usize;

        let mut chunks = Vec::new();
        let mut i = 0;

        while i < words.len() {
            let end = (i + chunk_size_words).min(words.len());
            let chunk = words[i..end].join(" ");
            chunks.push(chunk);

            if end == words.len() {
                break;
            }

            i += chunk_size_words - overlap_words;
        }

        chunks
    }

    async fn process_chunk(
        client: Client,
        index: usize,
        chunk: String,
        config: LLMConfig,
        instruction: String,
    ) -> (usize, String) {
        // Sanitize chunk - basic json escape handled by serde_json
        // We need to replace variables in prompt

        // Very basic sanitization of HTML for prompt injection protection could be done here
        // But assuming the prompt handles it via block delimeters

        let mut prompt = PROMPT_FILTER_CONTENT.replace("{HTML}", &chunk);
        prompt = prompt.replace("{REQUEST}", &instruction);

        match Self::perform_completion_with_backoff(client, &config, &prompt).await {
            Ok(content) => {
                // Extract content from <content> tags
                if let Some(start) = content.find("<content>") {
                    if let Some(end) = content.find("</content>") {
                        if start < end {
                             let extracted = &content[start + 9..end];
                             return (index, extracted.trim().to_string());
                        }
                    }
                }
                // Fallback: return full content if tags not found (or maybe LLM forgot tags)
                (index, content)
            },
            Err(e) => {
                eprintln!("Error processing chunk {}: {}", index, e);
                (index, String::new())
            }
        }
    }

    async fn perform_completion_with_backoff(client: Client, config: &LLMConfig, prompt: &str) -> Result<String, String> {
        let mut attempt = 0;

        // Basic OpenAI compatible request body
        let body_json = serde_json::json!({
            "model": config.provider,
            "messages": [
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.1
        });

        // If provider looks like "openai/...", strip the prefix for the model field if using standard base_url
        // Actually, usually users provide "gpt-4" etc.
        // If they use litellm style "openai/gpt-4", we might need to handle it.
        // For this implementation, we pass provider as is to model field.

        // Adjust for generic use
        let url = config.base_url.as_deref().unwrap_or("https://api.openai.com/v1/chat/completions");

        loop {
            attempt += 1;

            let res = client.post(url)
                .header("Authorization", format!("Bearer {}", config.api_token))
                .header("Content-Type", "application/json")
                .json(&body_json)
                .send()
                .await;

            match res {
                Ok(response) => {
                    if response.status().is_success() {
                        let json: Value = response.json().await.map_err(|e| e.to_string())?;
                        // Extract content
                        // Standard OpenAI response: choices[0].message.content
                        if let Some(content) = json.pointer("/choices/0/message/content") {
                             return Ok(content.as_str().unwrap_or("").to_string());
                        } else {
                            return Err("Invalid response format".to_string());
                        }
                    } else if response.status().as_u16() == 429 {
                        // Rate limit
                         if attempt >= config.backoff_max_attempts {
                            return Err(format!("Rate limit exceeded after {} attempts", attempt));
                        }
                        let delay = config.backoff_base_delay as f64 * config.backoff_exponential_factor.powi(attempt as i32 - 1);
                        tokio::time::sleep(Duration::from_secs_f64(delay)).await;
                        continue;
                    } else {
                        return Err(format!("API error: {}", response.status()));
                    }
                },
                Err(e) => {
                    if attempt >= config.backoff_max_attempts {
                        return Err(format!("Request failed: {}", e));
                    }
                    let delay = config.backoff_base_delay as f64 * config.backoff_exponential_factor.powi(attempt as i32 - 1);
                    tokio::time::sleep(Duration::from_secs_f64(delay)).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        // Expected behavior matching Python:
        // Chunk 1: "Text1 Text2" (Tag: p)
        // Chunk 2: "Text3" (Tag: div) - assuming body itself is not the parent directly but div is.
        // Wait, body contains div.
        // Traverse:
        // body start
        // div start
        // Text1 -> current=[Text1]
        // p start
        // Text2 -> current=[Text1, Text2]
        // p end -> flush "Text1 Text2" (p)
        // Text3 -> current=[Text3]
        // div end -> flush "Text3" (div)
        // body end -> current=[]

        // Note: The HTML parser puts everything in body.
        // The div is the child of body.

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

        // Traverse:
        // p start
        // Start -> current=[Start]
        // span start
        // Middle -> current=[Start, Middle]
        // span end (inline -> no break)
        // End -> current=[Start, Middle, End]
        // p end (break) -> flush "Start Middle End"

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].1, "Start Middle End");
    }
}
