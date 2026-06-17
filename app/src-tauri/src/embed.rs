use crate::db;
use sha2::{Digest, Sha256};

pub trait Embedder: Send + Sync {
    fn dim(&self) -> usize;
    fn embed(&self, text: &str) -> Vec<f32>;
}

/// Stub embedder -- replaced by candle e5-small in Faz 2c.
pub struct StubEmbedder;

impl Embedder for StubEmbedder {
    fn dim(&self) -> usize {
        db::EMBEDDING_DIM
    }

    fn embed(&self, text: &str) -> Vec<f32> {
        let mut vector = vec![0.0; db::EMBEDDING_DIM];
        let mut tokens_seen = 0usize;

        for token in tokens(text) {
            tokens_seen += 1;
            add_token(&mut vector, &token);
        }

        if tokens_seen == 0 {
            add_token(&mut vector, text);
        }

        normalize(vector)
    }
}

#[cfg(feature = "candle")]
mod candle_backend {
    use super::{Embedder, StubEmbedder};
    use candle_core::{DType, Device, Tensor};
    use candle_nn::VarBuilder;
    use candle_transformers::models::bert::{BertModel, Config};
    use hf_hub::api::sync::Api;
    use std::path::PathBuf;
    use tokenizers::{Tokenizer, TruncationParams};

    const MODEL_ID: &str = "intfloat/multilingual-e5-small";
    const MAX_SEQ_LEN: usize = 512;

    enum Weights {
        Safetensors(PathBuf),
        Pytorch(PathBuf),
    }

    pub struct CandleEmbedder {
        model: BertModel,
        tokenizer: Tokenizer,
        device: Device,
        pad_token_id: u32,
    }

    impl CandleEmbedder {
        pub fn new() -> Result<Self, String> {
            let api = Api::new().map_err(|err| format!("failed to initialize hf-hub: {err}"))?;
            let repo = api.model(MODEL_ID.to_string());

            let weights = match repo.get("model.safetensors") {
                Ok(path) => Weights::Safetensors(path),
                Err(safetensors_err) => match repo.get("pytorch_model.bin") {
                    Ok(path) => Weights::Pytorch(path),
                    Err(pytorch_err) => {
                        return Err(format!(
                            "failed to fetch model weights: model.safetensors: {safetensors_err}; pytorch_model.bin: {pytorch_err}"
                        ));
                    }
                },
            };
            let tokenizer_path = repo
                .get("tokenizer.json")
                .map_err(|err| format!("failed to fetch tokenizer.json: {err}"))?;
            let config_path = repo
                .get("config.json")
                .map_err(|err| format!("failed to fetch config.json: {err}"))?;

            let device = Device::new_metal(0).unwrap_or(Device::Cpu);
            let config_text = std::fs::read_to_string(&config_path)
                .map_err(|err| format!("failed to read {}: {err}", config_path.display()))?;
            let config: Config = serde_json::from_str(&config_text)
                .map_err(|err| format!("failed to parse config.json: {err}"))?;
            if config.hidden_size != crate::db::EMBEDDING_DIM {
                return Err(format!(
                    "model hidden size mismatch: expected {}, got {}",
                    crate::db::EMBEDDING_DIM,
                    config.hidden_size
                ));
            }
            let pad_token_id = config.pad_token_id as u32;

            let vb = match &weights {
                Weights::Safetensors(path) => {
                    unsafe { VarBuilder::from_mmaped_safetensors(&[path], DType::F32, &device) }
                        .map_err(|err| format!("failed to load safetensors weights: {err}"))?
                }
                Weights::Pytorch(path) => VarBuilder::from_pth(path, DType::F32, &device)
                    .map_err(|err| format!("failed to load pytorch weights: {err}"))?,
            };
            let model = BertModel::load(vb, &config)
                .map_err(|err| format!("failed to build BERT model: {err}"))?;

            let mut tokenizer = Tokenizer::from_file(&tokenizer_path)
                .map_err(|err| format!("failed to load tokenizer.json: {err}"))?;
            tokenizer
                .with_truncation(Some(TruncationParams {
                    max_length: MAX_SEQ_LEN,
                    ..Default::default()
                }))
                .map_err(|err| format!("failed to configure tokenizer truncation: {err}"))?;

            Ok(Self {
                model,
                tokenizer,
                device,
                pad_token_id,
            })
        }

        fn embed_inner(&self, text: &str) -> Result<Vec<f32>, String> {
            let input = format!("query: {text}");
            let encoding = self
                .tokenizer
                .encode(input, true)
                .map_err(|err| format!("failed to tokenize input: {err}"))?;
            let mut ids = encoding.get_ids().to_vec();
            let mut attention = encoding.get_attention_mask().to_vec();
            if ids.is_empty() {
                return Ok(StubEmbedder.embed(text));
            }
            ids.truncate(MAX_SEQ_LEN);
            attention.truncate(MAX_SEQ_LEN);
            ids.resize(MAX_SEQ_LEN, self.pad_token_id);
            attention.resize(MAX_SEQ_LEN, 0);

            let token_type_ids = vec![0u32; ids.len()];
            let input_ids = Tensor::new(ids.as_slice(), &self.device)
                .and_then(|tensor| tensor.unsqueeze(0))
                .map_err(|err| format!("failed to create input_ids tensor: {err}"))?;
            let token_type_ids = Tensor::new(token_type_ids.as_slice(), &self.device)
                .and_then(|tensor| tensor.unsqueeze(0))
                .map_err(|err| format!("failed to create token_type_ids tensor: {err}"))?;
            let attention_mask = Tensor::new(attention.clone(), &self.device)
                .and_then(|tensor| tensor.unsqueeze(0))
                .map_err(|err| format!("failed to create attention_mask tensor: {err}"))?;

            let token_embeddings = self
                .model
                .forward(&input_ids, &token_type_ids, Some(&attention_mask))
                .map_err(|err| format!("failed to run BERT forward pass: {err}"))?;
            let token_embeddings = token_embeddings
                .to_device(&Device::Cpu)
                .map_err(|err| format!("failed to copy embeddings to CPU: {err}"))?
                .to_vec3::<f32>()
                .map_err(|err| format!("failed to read embeddings: {err}"))?;

            let sequence = token_embeddings
                .first()
                .ok_or_else(|| "BERT forward pass returned no batch rows".to_string())?;
            mean_pool_l2(sequence, &attention)
        }
    }

    impl Embedder for CandleEmbedder {
        fn dim(&self) -> usize {
            crate::db::EMBEDDING_DIM
        }

        fn embed(&self, text: &str) -> Vec<f32> {
            match self.embed_inner(text) {
                Ok(vector) => vector,
                Err(err) => {
                    eprintln!(
                        "warning: CandleEmbedder failed; falling back to StubEmbedder: {err}"
                    );
                    StubEmbedder.embed(text)
                }
            }
        }
    }

    fn mean_pool_l2(token_embeddings: &[Vec<f32>], attention: &[u32]) -> Result<Vec<f32>, String> {
        let mut pooled = vec![0.0; crate::db::EMBEDDING_DIM];
        let mut token_count = 0.0f32;

        for (embedding, mask) in token_embeddings.iter().zip(attention.iter()) {
            if *mask == 0 {
                continue;
            }
            if embedding.len() != crate::db::EMBEDDING_DIM {
                return Err(format!(
                    "token embedding dimension mismatch: expected {}, got {}",
                    crate::db::EMBEDDING_DIM,
                    embedding.len()
                ));
            }
            token_count += 1.0;
            for (pooled_value, embedding_value) in pooled.iter_mut().zip(embedding.iter()) {
                *pooled_value += *embedding_value;
            }
        }

        if token_count == 0.0 {
            pooled[0] = 1.0;
            return Ok(pooled);
        }

        for value in &mut pooled {
            *value /= token_count;
        }

        let norm = pooled.iter().map(|value| value * value).sum::<f32>().sqrt();
        if norm == 0.0 {
            pooled[0] = 1.0;
            return Ok(pooled);
        }

        for value in &mut pooled {
            *value /= norm;
        }

        Ok(pooled)
    }
}

#[cfg(feature = "candle")]
pub use candle_backend::CandleEmbedder;

pub fn default_embedder() -> Box<dyn Embedder> {
    #[cfg(feature = "candle")]
    {
        match CandleEmbedder::new() {
            Ok(embedder) => return Box::new(embedder),
            Err(err) => {
                eprintln!("warning: failed to initialize CandleEmbedder; falling back to StubEmbedder: {err}");
            }
        }
    }

    Box::new(StubEmbedder)
}

fn tokens(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn add_token(vector: &mut [f32], token: &str) {
    let digest = Sha256::digest(token.as_bytes());
    for lane in 0..8 {
        let offset = lane * 4;
        let raw = u32::from_le_bytes([
            digest[offset],
            digest[offset + 1],
            digest[offset + 2],
            digest[offset + 3],
        ]);
        let index = raw as usize % vector.len();
        let sign = if raw & 0x8000_0000 == 0 { 1.0 } else { -1.0 };
        let weight = 1.0 + ((raw >> 8) & 0xff) as f32 / 255.0;
        vector[index] += sign * weight;
    }
}

fn normalize(mut vector: Vec<f32>) -> Vec<f32> {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();

    if norm == 0.0 {
        vector[0] = 1.0;
        return vector;
    }

    for value in &mut vector {
        *value /= norm;
    }
    vector
}
