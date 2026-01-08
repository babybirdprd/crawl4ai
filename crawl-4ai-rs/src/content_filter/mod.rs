use serde::{Deserialize, Serialize};

pub mod pruning;
pub mod bm25;
pub mod llm;

pub use pruning::PruningContentFilter;
pub use bm25::BM25ContentFilter;
pub use llm::{LLMContentFilter, LLMConfig};

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
