use serde::{Deserialize, Serialize};
use std::time::Duration;
use futures::stream::{self, StreamExt};
use rig::{client::CompletionClient, completion::Prompt, providers};

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

        // Determine provider and model
        let (provider, model) = if let Some((p, m)) = self.config.provider.split_once('/') {
            (p, m)
        } else {
             ("openai", self.config.provider.as_str())
        };

        if provider != "openai" {
             eprintln!("Unsupported provider: {}. Falling back to OpenAI behavior.", provider);
        }

        // Initialize Client/Agent once
        let mut client_builder = providers::openai::Client::builder()
             .api_key(&self.config.api_token);

        if let Some(ref base_url) = self.config.base_url {
            client_builder = client_builder.base_url(base_url);
        }

        let client: rig::providers::openai::Client = match client_builder.build() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to build LLM client: {}", e);
                return String::new();
            }
        };

        let agent = client.agent(model).build();

        // 2. Process chunks in parallel
        let tasks = chunks.into_iter().enumerate().map(|(i, chunk)| {
            let config = self.config.clone();
            let instruction = self.instruction.clone();
            let agent = agent.clone();
            async move {
                Self::process_chunk(i, chunk, config, instruction, agent).await
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

        if total_tokens_est <= self.chunk_token_threshold {
            return vec![text.to_string()];
        }

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

    async fn process_chunk<M: rig::completion::CompletionModel>(
        index: usize,
        chunk: String,
        config: LLMConfig,
        instruction: String,
        agent: rig::agent::Agent<M>,
    ) -> (usize, String) {
        // Prepare prompt
        let mut prompt_text = PROMPT_FILTER_CONTENT.replace("{HTML}", &chunk);
        prompt_text = prompt_text.replace("{REQUEST}", &instruction);

        let result = Self::perform_completion_with_backoff(&config, &prompt_text, agent).await;

        match result {
            Ok(content) => {
                if let Some(start) = content.find("<content>") {
                    if let Some(end) = content.find("</content>") {
                        if start < end {
                             let extracted = &content[start + 9..end];
                             return (index, extracted.trim().to_string());
                        }
                    }
                }
                (index, content)
            },
            Err(e) => {
                eprintln!("Error processing chunk {}: {}", index, e);
                (index, String::new())
            }
        }
    }

    async fn perform_completion_with_backoff<M: rig::completion::CompletionModel>(
        config: &LLMConfig,
        prompt_text: &str,
        agent: rig::agent::Agent<M>
    ) -> Result<String, String> {
        let mut attempt = 0;

        loop {
            attempt += 1;

            let res = agent.prompt(prompt_text).await;

            match res {
                Ok(response) => {
                    return Ok(response);
                },
                Err(e) => {
                    let err_msg = e.to_string();
                    if err_msg.contains("429") || err_msg.contains("Too Many Requests") {
                         if attempt >= config.backoff_max_attempts {
                            return Err(format!("Rate limit exceeded after {} attempts", attempt));
                        }
                        let delay = config.backoff_base_delay as f64 * config.backoff_exponential_factor.powi(attempt as i32 - 1);
                        tokio::time::sleep(Duration::from_secs_f64(delay)).await;
                        continue;
                    }

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
