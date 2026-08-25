use std::path::PathBuf;

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};

use crate::{FreshnessState, MemoryFact};

pub const LOCAL_MODEL_ID: &str = "fastembed/all-MiniLM-L6-v2";
pub const LOCAL_DIMENSION: usize = 384;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmbeddingAvailability {
    NeedsModel,
    Loading,
    Ready,
    Failed(String),
}

pub trait EmbeddingProvider {
    fn model_id(&self) -> &str;
    fn dimension(&self) -> usize;
    fn availability(&self) -> EmbeddingAvailability;
    fn embed_batch(&mut self, texts: &[String]) -> AcResult<Vec<Vec<f32>>>;
}

pub struct LocalEmbeddingProvider {
    model: Option<TextEmbedding>,
    availability: EmbeddingAvailability,
}

impl Default for LocalEmbeddingProvider {
    fn default() -> Self {
        Self {
            model: None,
            availability: EmbeddingAvailability::NeedsModel,
        }
    }
}

impl LocalEmbeddingProvider {
    pub fn load(&mut self, cache_dir: PathBuf) -> AcResult<()> {
        self.availability = EmbeddingAvailability::Loading;
        let options = TextInitOptions::new(EmbeddingModel::AllMiniLML6V2)
            .with_cache_dir(cache_dir)
            .with_show_download_progress(false)
            .with_intra_threads(2);
        match TextEmbedding::try_new(options) {
            Ok(model) => {
                self.model = Some(model);
                self.availability = EmbeddingAvailability::Ready;
                Ok(())
            }
            Err(error) => {
                let message = error.to_string();
                self.availability = EmbeddingAvailability::Failed(message.clone());
                Err(AcError::new(
                    "EMBEDDING-MODEL_LOAD",
                    message,
                    ac_common::ErrorKind::Unavailable,
                    ac_common::Retryability::Retryable,
                ))
            }
        }
    }
}

impl EmbeddingProvider for LocalEmbeddingProvider {
    fn model_id(&self) -> &str {
        LOCAL_MODEL_ID
    }
    fn dimension(&self) -> usize {
        LOCAL_DIMENSION
    }
    fn availability(&self) -> EmbeddingAvailability {
        self.availability.clone()
    }
    fn embed_batch(&mut self, texts: &[String]) -> AcResult<Vec<Vec<f32>>> {
        let model = self.model.as_mut().ok_or_else(|| {
            AcError::new(
                "EMBEDDING-MODEL_UNAVAILABLE",
                "local embedding model has not been explicitly loaded",
                ac_common::ErrorKind::Unavailable,
                ac_common::Retryability::Retryable,
            )
        })?;
        let vectors = model
            .embed(texts, None)
            .map_err(|error| AcError::validation("EMBEDDING-INFERENCE", error.to_string()))?;
        if vectors.iter().any(|vector| vector.len() != LOCAL_DIMENSION) {
            return Err(AcError::validation(
                "EMBEDDING-DIMENSION",
                "model returned an unexpected embedding dimension",
            ));
        }
        Ok(vectors)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticChunk {
    pub id: StableId,
    pub repository_id: StableId,
    pub fact_id: StableId,
    pub content: String,
    pub content_hash: String,
    pub model_id: String,
    pub dimension: usize,
    pub vector: Vec<f32>,
    pub freshness: FreshnessState,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticMatch {
    pub chunk: SemanticChunk,
    pub similarity: f32,
}

#[derive(Default)]
pub struct SemanticMemoryIndex {
    provider: LocalEmbeddingProvider,
    chunks: Vec<SemanticChunk>,
}

impl SemanticMemoryIndex {
    pub fn availability(&self) -> EmbeddingAvailability {
        self.provider.availability()
    }
    pub fn load_local(&mut self, cache_dir: impl Into<PathBuf>) -> AcResult<()> {
        self.provider.load(cache_dir.into())
    }
    pub fn chunks(&self) -> Vec<SemanticChunk> {
        self.chunks.clone()
    }
    pub fn restore(&mut self, chunks: Vec<SemanticChunk>) {
        self.chunks = chunks
            .into_iter()
            .filter(|chunk| {
                chunk.model_id == LOCAL_MODEL_ID
                    && chunk.dimension == LOCAL_DIMENSION
                    && chunk.vector.len() == LOCAL_DIMENSION
            })
            .collect();
    }
    pub fn index_fact(&mut self, fact: &MemoryFact) -> AcResult<bool> {
        if contains_secret(&fact.statement) {
            return Ok(false);
        }
        let content_hash = stable_hash(&fact.statement);
        if self.chunks.iter().any(|chunk| {
            chunk.fact_id == fact.id
                && chunk.content_hash == content_hash
                && chunk.model_id == LOCAL_MODEL_ID
        }) {
            return Ok(false);
        }
        let vector = self
            .provider
            .embed_batch(std::slice::from_ref(&fact.statement))?
            .pop()
            .ok_or_else(|| {
                AcError::validation(
                    "EMBEDDING-INFERENCE",
                    "embedding provider returned no vector",
                )
            })?;
        self.chunks.retain(|chunk| chunk.fact_id != fact.id);
        self.chunks.push(SemanticChunk {
            id: StableId::new("semchunk"),
            repository_id: fact.scope.repository_id.clone(),
            fact_id: fact.id.clone(),
            content: fact.statement.clone(),
            content_hash,
            model_id: LOCAL_MODEL_ID.to_string(),
            dimension: LOCAL_DIMENSION,
            vector,
            freshness: fact.freshness,
            created_at: TimestampMillis::now(),
        });
        Ok(true)
    }
    pub fn search(&mut self, query: &str, limit: usize) -> AcResult<Vec<SemanticMatch>> {
        let query = self
            .provider
            .embed_batch(&[query.to_string()])?
            .pop()
            .ok_or_else(|| {
                AcError::validation(
                    "EMBEDDING-INFERENCE",
                    "embedding provider returned no query vector",
                )
            })?;
        let mut matches = self
            .chunks
            .iter()
            .filter(|chunk| {
                chunk.freshness != FreshnessState::Invalid && chunk.dimension == query.len()
            })
            .cloned()
            .map(|chunk| SemanticMatch {
                similarity: cosine(&query, &chunk.vector),
                chunk,
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| right.similarity.total_cmp(&left.similarity));
        matches.truncate(limit);
        Ok(matches)
    }
}

fn cosine(left: &[f32], right: &[f32]) -> f32 {
    let (mut dot, mut left_norm, mut right_norm) = (0.0, 0.0, 0.0);
    for (a, b) in left.iter().zip(right) {
        dot += a * b;
        left_norm += a * a;
        right_norm += b * b;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        dot / (left_norm.sqrt() * right_norm.sqrt())
    }
}
fn stable_hash(value: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
fn contains_secret(value: &str) -> bool {
    let upper = value.to_ascii_uppercase();
    upper.contains("BEGIN PRIVATE KEY")
        || upper.contains("API_KEY=")
        || upper.contains("PASSWORD=")
        || upper.contains("TOKEN=")
        || value.contains("AKIA")
}
