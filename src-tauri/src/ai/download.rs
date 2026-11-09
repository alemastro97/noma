//! Resumable, checksum-verified downloads for the server build and the models.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::AiError;

pub type Progress = dyn Fn(u64, u64) + Send + Sync;

/// Downloads `url` to `dest`, resuming a `.part` file if one exists, then
/// verifies the SHA-256. `progress(received, total)` is throttled to ~4 Hz.
pub async fn download(
    url: &str,
    dest: &Path,
    sha256: &str,
    expected_size: u64,
    cancel: Arc<AtomicBool>,
    progress: &Progress,
) -> Result<(), AiError> {
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let part: PathBuf = dest.with_extension(format!(
        "{}.part",
        dest.extension().and_then(|e| e.to_str()).unwrap_or("bin")
    ));
    let mut have = tokio::fs::metadata(&part)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    if have > expected_size {
        tokio::fs::remove_file(&part).await?;
        have = 0;
    }

    if have < expected_size {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(20))
            .build()?;
        let mut req = client.get(url);
        if have > 0 {
            req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
        }
        let resp = req.send().await?.error_for_status()?;
        // A server that ignores Range sends the whole file again.
        if have > 0 && resp.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            have = 0;
        }
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(have > 0)
            .truncate(have == 0)
            .open(&part)
            .await?;
        let mut stream = resp.bytes_stream();
        let mut received = have;
        let mut last = Instant::now();
        progress(received, expected_size);
        while let Some(chunk) = stream.next().await {
            if cancel.load(Ordering::Relaxed) {
                file.flush().await?;
                return Err(AiError::Cancelled);
            }
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            received += chunk.len() as u64;
            if last.elapsed() >= Duration::from_millis(250) {
                progress(received, expected_size);
                last = Instant::now();
            }
        }
        file.flush().await?;
        progress(received, expected_size);
    }

    let actual = sha256_file(&part).await?;
    if !actual.eq_ignore_ascii_case(sha256) {
        let _ = tokio::fs::remove_file(&part).await;
        return Err(AiError::Checksum {
            expected: sha256.to_string(),
            actual,
        });
    }
    tokio::fs::rename(&part, dest).await?;
    Ok(())
}

pub async fn sha256_file(path: &Path) -> Result<String, AiError> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Extracts a `.tar.gz` into `into` and returns the directory that contains
/// `llama-server` (its dylibs sit next to it).
pub fn extract_server(archive: &Path, into: &Path) -> Result<PathBuf, AiError> {
    std::fs::create_dir_all(into)?;
    let file = std::fs::File::open(archive)?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    tar.unpack(into)?;
    let binary = find_file(into, server_binary_name())
        .ok_or_else(|| AiError::Server("llama-server not found in archive".into()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))?;
    }
    prepare_for_gatekeeper(into, &binary);
    Ok(binary
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| into.to_path_buf()))
}

pub fn server_binary_name() -> &'static str {
    if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    }
}

pub fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file(&path, name) {
                return Some(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Some(path);
        }
    }
    None
}

/// Files fetched by the app are not quarantined (only browsers tag downloads),
/// but strip the attribute and ad-hoc sign defensively so Gatekeeper never
/// blocks the server on first launch.
fn prepare_for_gatekeeper(dir: &Path, binary: &Path) {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let _ = Command::new("xattr")
            .args(["-dr", "com.apple.quarantine"])
            .arg(dir)
            .status();
        let signed = Command::new("codesign")
            .args(["-v"])
            .arg(binary)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !signed {
            let _ = Command::new("codesign")
                .args(["--force", "-s", "-"])
                .arg(binary)
                .status();
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (dir, binary);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sha256_matches_in_memory_digest() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.txt");
        tokio::fs::write(&p, b"noma").await.unwrap();
        let expected = hex::encode(Sha256::digest(b"noma"));
        assert_eq!(sha256_file(&p).await.unwrap(), expected);
    }

    #[tokio::test]
    async fn checksum_mismatch_deletes_partial_file() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("model.gguf");
        let part = dir.path().join("model.gguf.part");
        tokio::fs::write(&part, b"abcd").await.unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let err = download(
            "http://127.0.0.1:9/unused",
            &dest,
            &"0".repeat(64),
            4,
            cancel,
            &|_, _| {},
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AiError::Checksum { .. }));
        assert!(!part.exists());
        assert!(!dest.exists());
    }

    #[test]
    fn find_file_searches_nested_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("build").join("bin");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("llama-server"), b"").unwrap();
        assert_eq!(
            find_file(dir.path(), "llama-server").unwrap(),
            nested.join("llama-server")
        );
        assert!(find_file(dir.path(), "missing").is_none());
    }
}
