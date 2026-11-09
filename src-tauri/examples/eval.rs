//! Extraction eval: runs `ai/evals/cases.jsonl` through a model and reports
//! per-field accuracy and latency. Guards prompt and model changes.
//!
//! ```bash
//! # against a llama-server you started yourself
//! cargo run --release --example eval -- --llama http://127.0.0.1:8080
//! # against the server NOMA downloaded (starts and stops it)
//! cargo run --release --example eval -- --start ~/Library/Application\ Support/com.noma.app/ai/bin/b12110 \
//!     ~/Library/Application\ Support/com.noma.app/ai/models/Qwen3-4B-Instruct-2507-Q4_K_M.gguf
//! # against Ollama
//! cargo run --release --example eval -- --ollama qwen3:4b-instruct
//! ```
//!
//! Fixed context: Monday 9 November 2026, 10:00, Europe/Rome; projects
//! "Spring launch", "Studio move", "Reading list".
//! Gate (ADR-0003): type ≥ 90 %, date ≥ 85 %, warm p95 < 2.5 s.

use std::path::PathBuf;
use std::time::Instant;

use chrono::{TimeZone, Utc};
use chrono_tz::Tz;
use noma_lib::ai::extract::{self, Context};
use noma_lib::ai::provider::Provider;
use noma_lib::db::models::ItemType;
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    text: String,
    #[serde(rename = "type")]
    kind: String,
    /// Expected anchor (due, else remind) as local "YYYY-MM-DD" (date only) or "YYYY-MM-DDTHH:MM".
    when: Option<String>,
    repeat: String,
    project: Option<String>,
    count: usize,
}

fn kind_name(k: ItemType, creates_project: bool) -> &'static str {
    if creates_project {
        return "project";
    }
    match k {
        ItemType::Task => "task",
        ItemType::Reminder => "reminder",
        ItemType::Note => "note",
        ItemType::Idea => "idea",
        ItemType::Event => "event",
        ItemType::ProjectRef => "project",
    }
}

fn repeat_name(rule: Option<&str>) -> &'static str {
    match rule {
        None => "none",
        Some(r) if r.contains("MO,TU,WE,TH,FR") => "weekdays",
        Some(r) if r.contains("DAILY") => "daily",
        Some(r) if r.contains("WEEKLY") => "weekly",
        Some(r) if r.contains("MONTHLY") => "monthly",
        Some(r) if r.contains("YEARLY") => "yearly",
        Some(_) => "other",
    }
}

fn pct(n: usize, d: usize) -> f64 {
    if d == 0 {
        100.0
    } else {
        100.0 * n as f64 / d as f64
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let client = reqwest::Client::new();
    let mut server = None;
    let provider = match args.first().map(String::as_str) {
        Some("--llama") => Provider::Llama {
            base_url: args
                .get(1)
                .cloned()
                .unwrap_or_else(|| "http://127.0.0.1:8080".into()),
            thinking: args.iter().any(|a| a == "--thinking"),
        },
        Some("--ollama") => Provider::Ollama {
            base_url: "http://127.0.0.1:11434".into(),
            model: args.get(1).cloned().expect("--ollama MODEL"),
        },
        Some("--start") => {
            let bin = PathBuf::from(args.get(1).expect("--start BIN_DIR MODEL"));
            let model = PathBuf::from(args.get(2).expect("--start BIN_DIR MODEL"));
            let log = std::env::temp_dir().join("noma-eval-llama.log");
            let s = noma_lib::ai::server::start(&client, &bin, &model, "eval", 4096, &log)
                .await
                .expect("start llama-server");
            let base_url = s.base_url();
            server = Some(s);
            Provider::Llama {
                base_url,
                thinking: args.iter().any(|a| a == "--thinking"),
            }
        }
        _ => {
            eprintln!(
                "usage: eval --llama URL | --ollama MODEL | --start BIN_DIR MODEL.gguf [--thinking]"
            );
            std::process::exit(2);
        }
    };

    let cases: Vec<Case> = include_str!("../../ai/evals/cases.jsonl")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("valid case"))
        .collect();

    let tz: Tz = "Europe/Rome".parse().unwrap();
    let now = Utc.with_ymd_and_hms(2026, 11, 9, 9, 0, 0).unwrap();
    let projects = vec![
        ("p1".to_string(), "Spring launch".to_string()),
        ("p2".to_string(), "Studio move".to_string()),
        ("p3".to_string(), "Reading list".to_string()),
    ];
    let ctx = Context {
        now,
        tz,
        projects: &projects,
    };

    // Warm-up: load the model and fill the prompt cache.
    let _ = provider
        .complete_json(
            &client,
            extract::SYSTEM_PROMPT,
            &extract::user_prompt("warm up", &ctx),
            &extract::schema(),
        )
        .await;

    let (mut ok_type, mut ok_when, mut n_when, mut ok_repeat, mut ok_project, mut ok_count) =
        (0, 0, 0, 0, 0, 0);
    let mut latencies = Vec::new();
    let mut failures = Vec::new();

    for case in &cases {
        let started = Instant::now();
        let reply = provider
            .complete_json(
                &client,
                extract::SYSTEM_PROMPT,
                &extract::user_prompt(&case.text, &ctx),
                &extract::schema(),
            )
            .await;
        latencies.push(started.elapsed().as_millis() as u64);
        let items = match reply
            .map_err(|e| e.to_string())
            .and_then(|r| extract::parse_output(&r, &case.text, &ctx).map_err(|e| e.to_string()))
        {
            Ok(items) => items,
            Err(e) => {
                failures.push(format!("✗ {} → error: {e}", case.text));
                continue;
            }
        };
        let first = &items[0];
        let got_type = kind_name(first.kind, first.creates_project);
        let anchor = first
            .due_at
            .or(first.remind_at)
            .map(|d| d.with_timezone(&tz));
        let got_when = anchor.map(|a| match &case.when {
            Some(w) if w.len() == 10 => a.format("%Y-%m-%d").to_string(),
            _ => a.format("%Y-%m-%dT%H:%M").to_string(),
        });
        let got_repeat = repeat_name(first.recurrence.as_deref());
        let got_project = first.project_id.as_ref().and_then(|id| {
            projects
                .iter()
                .find(|(p, _)| p == id)
                .map(|(_, n)| n.clone())
        });

        let mut miss = Vec::new();
        if got_type == case.kind {
            ok_type += 1;
        } else {
            miss.push(format!("type {got_type}≠{}", case.kind));
        }
        if case.count == 1 {
            n_when += 1;
            if got_when == case.when {
                ok_when += 1;
            } else {
                miss.push(format!("when {:?}≠{:?}", got_when, case.when));
            }
        }
        if got_repeat == case.repeat {
            ok_repeat += 1;
        } else {
            miss.push(format!("repeat {got_repeat}≠{}", case.repeat));
        }
        if got_project == case.project {
            ok_project += 1;
        } else {
            miss.push(format!("project {:?}≠{:?}", got_project, case.project));
        }
        if items.len() == case.count {
            ok_count += 1;
        } else {
            miss.push(format!("count {}≠{}", items.len(), case.count));
        }
        if !miss.is_empty() {
            failures.push(format!("✗ {} → {}", case.text, miss.join(", ")));
        }
    }

    latencies.sort_unstable();
    let p = |q: f64| latencies[((latencies.len() as f64 - 1.0) * q).round() as usize];
    let n = cases.len();
    println!(
        "\nNOMA extraction eval — {} · {n} cases\n",
        provider.label()
    );
    for f in &failures {
        println!("{f}");
    }
    println!();
    println!("type     {:5.1} %", pct(ok_type, n));
    println!(
        "date     {:5.1} %  ({n_when} single-item cases)",
        pct(ok_when, n_when)
    );
    println!("repeat   {:5.1} %", pct(ok_repeat, n));
    println!("project  {:5.1} %", pct(ok_project, n));
    println!("count    {:5.1} %", pct(ok_count, n));
    println!("latency  p50 {} ms · p95 {} ms", p(0.5), p(0.95));
    let pass = pct(ok_type, n) >= 90.0 && pct(ok_when, n_when) >= 85.0 && p(0.95) < 2500;
    println!("\ngate     {}", if pass { "PASS" } else { "FAIL" });
    if let Some(s) = server {
        s.stop().await;
    }
}
