use serde::{Deserialize, Serialize};
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
