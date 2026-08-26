use std::fs;
use std::path::{Path, PathBuf};

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use sha2::{Digest, Sha256};

use crate::{FreshnessState, MemoryFact};

pub const LOCAL_MODEL_ID: &str = "fastembed/all-MiniLM-L6-v2";
pub const LOCAL_DIMENSION: usize = 384;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmbeddingAvailability {
    NeedsModel,
    Loading,
    Ready,
    Failed(String),
    Unavailable(String),
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
    manifest: Option<ModelArtifactManifest>,
}

impl Default for LocalEmbeddingProvider {
    fn default() -> Self {
        Self {
            model: None,
            availability: EmbeddingAvailability::NeedsModel,
            manifest: None,
        }
    }
}

impl LocalEmbeddingProvider {
    pub fn load(&mut self, cache_dir: PathBuf) -> AcResult<()> {
        self.availability = EmbeddingAvailability::Loading;
        verify_manifest_if_present(&cache_dir)?;
        let options = TextInitOptions::new(EmbeddingModel::AllMiniLML6V2)
            .with_cache_dir(cache_dir.clone())
            .with_show_download_progress(false)
            .with_intra_threads(2);
        match TextEmbedding::try_new(options) {
            Ok(model) => {
                let manifest = record_or_load_manifest(&cache_dir)?;
                self.model = Some(model);
                self.manifest = Some(manifest);
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelTrustState {
    TrustOnFirstUse,
    VerifiedAgainstLocalManifest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelArtifactManifest {
    pub model_id: String,
    pub revision: Option<String>,
    pub dimension: usize,
    pub artifact_root: PathBuf,
    pub sha256: String,
    pub trust_state: ModelTrustState,
    pub created_at: TimestampMillis,
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
        self.chunks = chunks.into_iter().filter(vector_chunk_is_usable).collect();
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
        if !vector_values_are_usable(&vector) {
            return Err(AcError::validation(
                "EMBEDDING-VECTOR_INVALID",
                "embedding provider returned an unusable vector",
            ));
        }
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
        if !vector_values_are_usable(&query) {
            return Err(AcError::validation(
                "EMBEDDING-VECTOR_INVALID",
                "embedding provider returned an unusable query vector",
            ));
        }
        let mut matches = self
            .chunks
            .iter()
            .filter(|chunk| {
                chunk.freshness != FreshnessState::Invalid
                    && chunk.dimension == query.len()
                    && vector_values_are_usable(&chunk.vector)
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

fn vector_chunk_is_usable(chunk: &SemanticChunk) -> bool {
    chunk.model_id == LOCAL_MODEL_ID
        && chunk.dimension == LOCAL_DIMENSION
        && chunk.vector.len() == LOCAL_DIMENSION
        && vector_values_are_usable(&chunk.vector)
}

fn vector_values_are_usable(vector: &[f32]) -> bool {
    !vector.is_empty()
        && vector.iter().all(|value| value.is_finite())
        && vector.iter().any(|value| *value != 0.0)
}

const MANIFEST_FILE: &str = "agentcode-fastembed-manifest.txt";

fn verify_manifest_if_present(cache_dir: &Path) -> AcResult<()> {
    let manifest_path = cache_dir.join(MANIFEST_FILE);
    if !manifest_path.exists() {
        return Ok(());
    }
    let manifest = read_manifest(&manifest_path)?;
    let actual = hash_artifact_root(cache_dir)?;
    if manifest.sha256 != actual {
        return Err(AcError::new(
            "EMBEDDING-MODEL_INTEGRITY",
            "FastEmbed artifact digest does not match AgentCode local manifest",
            ac_common::ErrorKind::Unavailable,
            ac_common::Retryability::NotRetryable,
        ));
    }
    Ok(())
}

fn record_or_load_manifest(cache_dir: &Path) -> AcResult<ModelArtifactManifest> {
    fs::create_dir_all(cache_dir)
        .map_err(|error| AcError::validation("EMBEDDING-MODEL_CACHE", error.to_string()))?;
    let manifest_path = cache_dir.join(MANIFEST_FILE);
    if manifest_path.exists() {
        let mut manifest = read_manifest(&manifest_path)?;
        manifest.trust_state = ModelTrustState::VerifiedAgainstLocalManifest;
        return Ok(manifest);
    }
    let manifest = ModelArtifactManifest {
        model_id: LOCAL_MODEL_ID.to_string(),
        revision: None,
        dimension: LOCAL_DIMENSION,
        artifact_root: cache_dir.to_path_buf(),
        sha256: hash_artifact_root(cache_dir)?,
        trust_state: ModelTrustState::TrustOnFirstUse,
        created_at: TimestampMillis::now(),
    };
    write_manifest(&manifest_path, &manifest)?;
    Ok(manifest)
}

fn read_manifest(path: &Path) -> AcResult<ModelArtifactManifest> {
    let content = fs::read_to_string(path)
        .map_err(|error| AcError::validation("EMBEDDING-MODEL_MANIFEST", error.to_string()))?;
    let mut model_id = String::new();
    let mut dimension = 0;
    let mut sha256 = String::new();
    let mut created_at = 0;
    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            match key {
                "model_id" => model_id = value.to_string(),
                "dimension" => dimension = value.parse().unwrap_or(0),
                "sha256" => sha256 = value.to_string(),
                "created_at" => created_at = value.parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    if model_id != LOCAL_MODEL_ID || dimension != LOCAL_DIMENSION || sha256.len() != 64 {
        return Err(AcError::new(
            "EMBEDDING-MODEL_MANIFEST_INVALID",
            "FastEmbed artifact manifest does not match the configured local model",
            ac_common::ErrorKind::Unavailable,
            ac_common::Retryability::NotRetryable,
        ));
    }
    Ok(ModelArtifactManifest {
        model_id,
        revision: None,
        dimension,
        artifact_root: path.parent().unwrap_or_else(|| Path::new("")).to_path_buf(),
        sha256,
        trust_state: ModelTrustState::VerifiedAgainstLocalManifest,
        created_at: TimestampMillis::from_millis(created_at),
    })
}

fn write_manifest(path: &Path, manifest: &ModelArtifactManifest) -> AcResult<()> {
    let content = format!(
        "model_id={}\ndimension={}\nsha256={}\ncreated_at={}\ntrust_state=TOFU\n",
        manifest.model_id,
        manifest.dimension,
        manifest.sha256,
        manifest.created_at.as_millis()
    );
    fs::write(path, content)
        .map_err(|error| AcError::validation("EMBEDDING-MODEL_MANIFEST_WRITE", error.to_string()))
}

fn hash_artifact_root(root: &Path) -> AcResult<String> {
    let mut files = Vec::new();
    collect_artifact_files(root, &mut files)?;
    files.sort();
    let mut hasher = Sha256::new();
    for path in files {
        let relative = path.strip_prefix(root).unwrap_or(&path);
        hasher.update(relative.display().to_string().as_bytes());
        hasher.update([0]);
        let bytes = fs::read(&path)
            .map_err(|error| AcError::validation("EMBEDDING-MODEL_HASH", error.to_string()))?;
        hasher.update(bytes);
        hasher.update([0]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_artifact_files(root: &Path, files: &mut Vec<PathBuf>) -> AcResult<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)
        .map_err(|error| AcError::validation("EMBEDDING-MODEL_CACHE_READ", error.to_string()))?
    {
        let entry = entry.map_err(|error| {
            AcError::validation("EMBEDDING-MODEL_CACHE_READ", error.to_string())
        })?;
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some(MANIFEST_FILE) {
            continue;
        }
        if path.is_dir() {
            collect_artifact_files(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_drops_model_mismatch_corrupt_nonfinite_and_zero_vectors() {
        let mut index = SemanticMemoryIndex::default();
        let usable = chunk(
            "fact-good",
            LOCAL_MODEL_ID,
            LOCAL_DIMENSION,
            vec![0.1; LOCAL_DIMENSION],
        );
        let wrong_model = chunk(
            "fact-model",
            "other-model",
            LOCAL_DIMENSION,
            vec![0.1; LOCAL_DIMENSION],
        );
        let wrong_dimension = chunk("fact-dim", LOCAL_MODEL_ID, 3, vec![0.1; 3]);
        let corrupt = chunk(
            "fact-corrupt",
            LOCAL_MODEL_ID,
            LOCAL_DIMENSION,
            vec![0.1; 4],
        );
        let nonfinite = chunk("fact-nan", LOCAL_MODEL_ID, LOCAL_DIMENSION, {
            let mut vector = vec![0.1; LOCAL_DIMENSION];
            vector[0] = f32::NAN;
            vector
        });
        let zero = chunk(
            "fact-zero",
            LOCAL_MODEL_ID,
            LOCAL_DIMENSION,
            vec![0.0; LOCAL_DIMENSION],
        );

        index.restore(vec![
            usable,
            wrong_model,
            wrong_dimension,
            corrupt,
            nonfinite,
            zero,
        ]);
        let restored = index.chunks();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].fact_id.as_str(), "fact-good");
    }

    #[test]
    fn manifest_detects_tampered_local_artifact_bytes() {
        let root = std::env::temp_dir().join(format!(
            "agentcode-model-integrity-{}",
            StableId::new("tmp")
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("model.bin"), "trusted bytes").unwrap();
        let manifest = record_or_load_manifest(&root).unwrap();
        assert_eq!(manifest.trust_state, ModelTrustState::TrustOnFirstUse);
        verify_manifest_if_present(&root).unwrap();
        fs::write(root.join("model.bin"), "tampered bytes").unwrap();
        let err = verify_manifest_if_present(&root).unwrap_err();
        assert_eq!(err.code(), "EMBEDDING-MODEL_INTEGRITY");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unchanged_content_with_same_model_is_not_reembedded() {
        let mut index = SemanticMemoryIndex::default();
        let fact = MemoryFact {
            id: StableId::from_existing("fact-no-reembed").unwrap(),
            statement: "durable memory persists trusted architecture facts".to_string(),
            fact_type: crate::FactType::ArchitectureFact,
            source: crate::FactSource::Runtime,
            source_evidence: vec![StableId::new("evidence")],
            confidence: 100,
            freshness: FreshnessState::Fresh,
            scope: crate::MemoryScope {
                repository_id: StableId::new("repo"),
                mission_id: None,
                task_id: None,
                branch: None,
            },
            memory_class: crate::MemoryClass::Decision,
            observed_commit: "abc".to_string(),
            dependencies: Vec::new(),
            conflict_set: None,
            valid_from: TimestampMillis::now(),
            valid_until: None,
            superseded_by: None,
            last_validation: TimestampMillis::now(),
        };
        index.restore(vec![SemanticChunk {
            id: StableId::new("semchunk"),
            repository_id: fact.scope.repository_id.clone(),
            fact_id: fact.id.clone(),
            content: fact.statement.clone(),
            content_hash: stable_hash(&fact.statement),
            model_id: LOCAL_MODEL_ID.to_string(),
            dimension: LOCAL_DIMENSION,
            vector: vec![0.1; LOCAL_DIMENSION],
            freshness: FreshnessState::Fresh,
            created_at: TimestampMillis::now(),
        }]);
        assert!(!index.index_fact(&fact).unwrap());
        assert_eq!(index.chunks().len(), 1);
    }

    fn chunk(fact: &str, model_id: &str, dimension: usize, vector: Vec<f32>) -> SemanticChunk {
        SemanticChunk {
            id: StableId::new("semchunk"),
            repository_id: StableId::new("repo"),
            fact_id: StableId::from_existing(fact).unwrap(),
            content: fact.to_string(),
            content_hash: stable_hash(fact),
            model_id: model_id.to_string(),
            dimension,
            vector,
            freshness: FreshnessState::Fresh,
            created_at: TimestampMillis::now(),
        }
    }
}
