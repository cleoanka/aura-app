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
