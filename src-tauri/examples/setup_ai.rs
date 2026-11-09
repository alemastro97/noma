//! Downloads the pinned llama-server build and a model into NOMA's data dir,
//! using the same resumable, checksum-verified code path as the app.
//!
//! ```bash
//! cargo run --release --example setup_ai [model-id]
//! ```

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use noma_lib::ai::{Paths, download, manifest};

fn app_data_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME");
    PathBuf::from(home).join("Library/Application Support/com.noma.app")
}

#[tokio::main]
async fn main() {
    let model_id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| manifest::default_model().id.clone());
    let spec = manifest::model(&model_id).expect("unknown model id");
    let paths = Paths::new(&app_data_dir());
    let cancel = Arc::new(AtomicBool::new(false));
    let report = |label: &'static str| {
        move |received: u64, total: u64| {
            let pct = (received * 100).checked_div(total).unwrap_or(0);
            println!("{label} {pct:3}% ({} / {} MB)", received >> 20, total >> 20);
        }
    };

    let server = &manifest::manifest().server;
    let server_dir = paths.server_dir(&server.version);
    if download::find_file(&server_dir, download::server_binary_name()).is_none() {
        let asset = manifest::server_asset().expect("server asset for this platform");
        let archive = paths
            .downloads()
            .join(format!("llama-{}.tar.gz", server.version));
        download::download(
            &asset.url,
            &archive,
            &asset.sha256,
            asset.size_bytes,
            cancel.clone(),
            &report("server"),
        )
        .await
        .expect("server download");
        let bin = download::extract_server(&archive, &server_dir).expect("extract server");
        let _ = std::fs::remove_file(&archive);
        println!("server ready in {}", bin.display());
    } else {
        println!("server already installed");
    }

    let dest = paths.model_file(&spec.file);
    if !dest.exists() {
        download::download(
            &spec.url,
            &dest,
            &spec.sha256,
            spec.size_bytes,
            cancel,
            &report("model"),
        )
        .await
        .expect("model download");
    }
    println!("model ready at {}", dest.display());
}
