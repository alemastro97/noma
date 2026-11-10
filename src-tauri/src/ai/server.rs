//! Supervises the `llama-server` child process: start on demand, wait until the
//! model is loaded, stop when idle or when NOMA quits.

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::process::{Child, Command};

use super::AiError;
use super::download::server_binary_name;

pub struct LlamaServer {
    child: Child,
    pub port: u16,
    pub model_id: String,
    pub started: Instant,
}

impl LlamaServer {
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    pub async fn stop(mut self) {
        let _ = self.child.kill().await;
    }

    /// Synchronous kill for app exit paths.
    pub fn kill_now(&mut self) {
        let _ = self.child.start_kill();
    }
}

/// The pid file sits next to the log: `llama-server.pid`.
fn pid_path(log_path: &Path) -> PathBuf {
    log_path.with_extension("pid")
}

/// Kills a server left behind by a previous NOMA that died without cleaning up
/// (crash, `kill -9`, dev restart). Only kills if the pid is still llama-server.
pub fn reap_stale(log_path: &Path) {
    let pid_file = pid_path(log_path);
    let Some(pid) = std::fs::read_to_string(&pid_file)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
    else {
        return;
    };
    let _ = std::fs::remove_file(&pid_file);
    let comm = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    if comm.contains("llama-server") {
        log::info!("stopping stale llama-server (pid {pid})");
        let _ = std::process::Command::new("kill")
            .arg(pid.to_string())
            .status();
    }
}

/// Memory flags: flash attention + 8-bit KV cache halve the cache (576 → 306 MiB
/// at 4096 ctx) and a smaller micro-batch trims the compute buffer.
/// `NOMA_LLAMA_FULL_KV=1` turns this off (for A/B measurements).
fn memory_flags() -> Vec<&'static str> {
    if std::env::var_os("NOMA_LLAMA_FULL_KV").is_some() {
        return vec![];
    }
    vec![
        "--flash-attn",
        "on",
        "--cache-type-k",
        "q8_0",
        "--cache-type-v",
        "q8_0",
        "--ubatch-size",
        "256",
    ]
}

fn free_port() -> Result<u16, AiError> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

fn tail(path: &Path, lines: usize) -> String {
    std::fs::read_to_string(path)
        .map(|s| {
            let all: Vec<&str> = s.lines().collect();
            all[all.len().saturating_sub(lines)..].join("\n")
        })
        .unwrap_or_default()
}

/// Starts the server for `model` and resolves once `/health` answers 200.
pub async fn start(
    client: &reqwest::Client,
    bin_dir: &Path,
    model: &Path,
    model_id: &str,
    context_size: u32,
    log_path: &Path,
) -> Result<LlamaServer, AiError> {
    reap_stale(log_path);
    let port = free_port()?;
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let log = std::fs::File::create(log_path)?;
    let binary: PathBuf = bin_dir.join(server_binary_name());
    let child = Command::new(&binary)
        .current_dir(bin_dir)
        .args([
            "--model",
            &model.to_string_lossy(),
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--ctx-size",
            &context_size.to_string(),
            "--n-gpu-layers",
            "99",
            "--parallel",
            "1",
            "--no-webui",
            "--jinja",
            "--offline",
        ])
        .args(memory_flags())
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| AiError::Server(format!("could not start {}: {e}", binary.display())))?;

    if let Some(pid) = child.id() {
        let _ = std::fs::write(pid_path(log_path), pid.to_string());
    }
    let mut server = LlamaServer {
        child,
        port,
        model_id: model_id.to_string(),
        started: Instant::now(),
    };
    let deadline = Instant::now() + Duration::from_secs(120);
    let url = format!("{}/health", server.base_url());
    loop {
        if !server.is_running() {
            return Err(AiError::Server(format!(
                "llama-server exited during startup:\n{}",
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
                "llama-server did not become ready in 120 s".into(),
            ));
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}
