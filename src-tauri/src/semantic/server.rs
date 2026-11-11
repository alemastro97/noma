//! A second `llama-server`, started with `--embedding` on the small embedding
//! model. Same binary as the chat server; its own port, log and pid file.

use std::net::TcpListener;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tokio::process::{Child, Command};

use crate::ai::AiError;
use crate::ai::download::server_binary_name;

pub struct EmbedServer {
    child: Child,
    port: u16,
    pub model_id: String,
}

impl EmbedServer {
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    pub async fn stop(mut self) {
        let _ = self.child.kill().await;
    }

    pub fn kill_now(&mut self) {
        let _ = self.child.start_kill();
    }
}

fn tail(path: &Path, lines: usize) -> String {
    std::fs::read_to_string(path)
        .map(|s| {
            let all: Vec<&str> = s.lines().collect();
            all[all.len().saturating_sub(lines)..].join("\n")
        })
        .unwrap_or_default()
}

/// Starts the embedding server and resolves once `/health` answers 200.
pub async fn start(
    client: &reqwest::Client,
    bin_dir: &Path,
    model: &Path,
    model_id: &str,
    context_size: u32,
    log_path: &Path,
) -> Result<EmbedServer, AiError> {
    crate::ai::server::reap_stale(log_path);
    let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let log = std::fs::File::create(log_path)?;
    let binary = bin_dir.join(server_binary_name());
    let ctx = context_size.to_string();
    let child = Command::new(&binary)
        .current_dir(bin_dir)
        .args([
            "--model",
            &model.to_string_lossy(),
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--embedding",
            "--ctx-size",
            &ctx,
            // A whole card must fit in one micro-batch for non-causal models.
            "--batch-size",
            &ctx,
            "--ubatch-size",
            &ctx,
            "--n-gpu-layers",
            "99",
            "--parallel",
            "1",
            "--no-webui",
            "--offline",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| AiError::Server(format!("could not start {}: {e}", binary.display())))?;
    if let Some(pid) = child.id() {
        let _ = std::fs::write(log_path.with_extension("pid"), pid.to_string());
    }
    let mut server = EmbedServer {
        child,
        port,
        model_id: model_id.to_string(),
    };
    let deadline = Instant::now() + Duration::from_secs(60);
    let url = format!("{}/health", server.base_url());
    loop {
        if !server.is_running() {
            return Err(AiError::Server(format!(
                "embedding server exited during startup:\n{}",
                tail(log_path, 12)
            )));
        }
        if let Ok(resp) = client
            .get(&url)
            .timeout(Duration::from_millis(500))
            .send()
            .await
            && resp.status().is_success()
        {
            return Ok(server);
        }
        if Instant::now() > deadline {
            server.stop().await;
            return Err(AiError::Server(
                "embedding server did not become ready in 60 s".into(),
            ));
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}

/// Embeds `inputs` (OpenAI-compatible `/v1/embeddings`), one vector per input,
/// in input order.
pub async fn embed(
    client: &reqwest::Client,
    base_url: &str,
    inputs: &[String],
) -> Result<Vec<Vec<f32>>, AiError> {
    let resp = client
        .post(format!("{base_url}/v1/embeddings"))
        .timeout(Duration::from_secs(60))
        .json(&json!({ "input": inputs, "encoding_format": "float" }))
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let detail: String = resp
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(300)
            .collect();
        return Err(AiError::Server(format!("embeddings: {status}: {detail}")));
    }
    let body: Value = resp.json().await?;
    parse_embeddings(&body, inputs.len())
}

fn parse_embeddings(body: &Value, expected: usize) -> Result<Vec<Vec<f32>>, AiError> {
    let data = body["data"]
        .as_array()
        .ok_or_else(|| AiError::BadOutput("embeddings: no data".into()))?;
    let mut out: Vec<Option<Vec<f32>>> = vec![None; expected];
    for (pos, row) in data.iter().enumerate() {
        let index = row["index"].as_u64().map(|i| i as usize).unwrap_or(pos);
        let vector: Vec<f32> = row["embedding"]
            .as_array()
            .ok_or_else(|| AiError::BadOutput("embeddings: missing vector".into()))?
            .iter()
            .map(|x| x.as_f64().unwrap_or(0.0) as f32)
            .collect();
        if let Some(slot) = out.get_mut(index) {
            *slot = Some(vector);
        }
    }
    out.into_iter()
        .map(|v| v.ok_or_else(|| AiError::BadOutput("embeddings: missing row".into())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rows_by_index() {
        let body = json!({ "data": [
            { "index": 1, "embedding": [0.0, 1.0] },
            { "index": 0, "embedding": [1.0, 0.0] }
        ]});
        let v = parse_embeddings(&body, 2).unwrap();
        assert_eq!(v[0], vec![1.0, 0.0]);
        assert_eq!(v[1], vec![0.0, 1.0]);
        assert!(parse_embeddings(&json!({ "data": [] }), 1).is_err());
    }
}
