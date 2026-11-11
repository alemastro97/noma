//! Pure helpers for the semantic index: the text a card is embedded as, its
//! content hash, normalisation and brute-force cosine ranking.

use sha2::{Digest, Sha256};

/// Characters of card text sent to the embedding model (≈ 500–700 tokens,
/// well inside the model's 2048-token window).
pub const MAX_DOC_CHARS: usize = 2000;

/// The text a card is embedded as: title, then body.
pub fn doc_text(title: &str, body: Option<&str>) -> String {
    let title = title.trim();
    let body = body.map(str::trim).unwrap_or("");
    let text = if body.is_empty() || body == title {
        title.to_string()
    } else {
        format!("{title}\n{body}")
    };
    text.chars().take(MAX_DOC_CHARS).collect()
}

/// Changes whenever the embedded text or the model changes.
pub fn content_hash(model_id: &str, text: &str) -> String {
    let mut h = Sha256::new();
    h.update(model_id.as_bytes());
    h.update([0]);
    h.update(text.as_bytes());
    hex::encode(h.finalize())
}

/// Scales `v` to unit length (a zero vector stays zero).
pub fn normalize(mut v: Vec<f32>) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}

/// Cosine similarity of two unit vectors (their dot product). Mismatched
/// lengths (vectors of another model) score 0.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// The `k` best matches scoring at least `min`, best first.
pub fn top_k(
    query: &[f32],
    index: &[(String, Vec<f32>)],
    k: usize,
    min: f32,
) -> Vec<(String, f32)> {
    let mut scored: Vec<(String, f32)> = index
        .iter()
        .map(|(id, v)| (id.clone(), cosine(query, v)))
        .filter(|(_, s)| *s >= min)
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.truncate(k);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_text_skips_duplicate_body_and_truncates() {
        assert_eq!(doc_text("Buy film", Some("Buy film")), "Buy film");
        assert_eq!(
            doc_text("Buy film", Some("Portra 400")),
            "Buy film\nPortra 400"
        );
        let long = "x".repeat(5000);
        assert_eq!(doc_text(&long, None).chars().count(), MAX_DOC_CHARS);
    }

    #[test]
    fn hash_depends_on_model_and_text() {
        assert_ne!(content_hash("a", "t"), content_hash("b", "t"));
        assert_eq!(content_hash("a", "t"), content_hash("a", "t"));
    }

    #[test]
    fn ranks_by_cosine_with_threshold() {
        let index = vec![
            ("x".to_string(), normalize(vec![1.0, 0.0])),
            ("y".to_string(), normalize(vec![1.0, 1.0])),
            ("z".to_string(), normalize(vec![-1.0, 0.0])),
        ];
        let q = normalize(vec![1.0, 0.1]);
        let hits = top_k(&q, &index, 5, 0.5);
        let ids: Vec<&str> = hits.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(ids, vec!["x", "y"]);
    }
}
