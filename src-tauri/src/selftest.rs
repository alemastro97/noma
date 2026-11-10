//! Debug-only end-to-end self-test (`NOMA_SELFTEST_DB=/tmp/x.db pnpm tauri dev`).
//!
//! Runs on a throwaway database: opens the real floating windows, then pushes
//! Italian sentences synthesized with `say` through Whisper (loaded on demand),
//! the capture pipeline and the local model, and prints what came out. Quits at
//! the end, which also exercises the clean-exit path. Results go to stdout.

use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

use crate::db;
use crate::db::models::{ItemType, NewItem};
use crate::state::{AppState, now};
use crate::voice::session::{Finish, VoiceService};
use crate::windows;

fn say(msg: &str) {
    println!("SELFTEST {msg}");
}

fn check(name: &str, ok: bool) {
    say(&format!("{} {name}", if ok { "PASS" } else { "FAIL" }));
}

fn speech(voice: &str, text: &str, n: usize) -> Vec<f32> {
    let dir = std::env::temp_dir().join("noma-selftest");
    std::fs::create_dir_all(&dir).unwrap();
    let (aiff, wav) = (dir.join(format!("{n}.aiff")), dir.join(format!("{n}.wav")));
    Command::new("say")
        .args(["-v", voice, "-o"])
        .arg(&aiff)
        .arg(text)
        .status()
        .expect("say");
    Command::new("afconvert")
        .args(["-f", "WAVE", "-d", "LEI16@16000", "-c", "1"])
        .arg(&aiff)
        .arg(&wav)
        .status()
        .expect("afconvert");
    let bytes = std::fs::read(&wav).unwrap();
    let pos = bytes.windows(4).position(|w| w == b"data").unwrap() + 8;
    bytes[pos..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| i16::from_le_bytes(*c) as f32 / 32768.0)
        .collect()
}

async fn windows_phase(app: &AppHandle) {
    for label in [
        windows::PANEL,
        windows::TOASTS,
        windows::OVERLAY,
        windows::CAPTURE,
    ] {
        let t = Instant::now();
        windows::show(app, label);
        let mut shown = false;
        for _ in 0..40 {
            if windows::is_shown(app, label) {
                shown = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        check(
            &format!("{label} window shows ({} ms)", t.elapsed().as_millis()),
            shown,
        );
        tokio::time::sleep(Duration::from_millis(700)).await;
        windows::hide(app, label);
        tokio::time::sleep(Duration::from_millis(150)).await;
        check(
            &format!("{label} window hides"),
            !windows::is_shown(app, label),
        );
    }
}

fn seed(app: &AppHandle) -> (String, String) {
    let state = app.state::<AppState>();
    let conn = state.db.lock().unwrap();
    let t = now();
    let studio = db::projects::create_project(&conn, "Studio move", "#79B8F5", t).unwrap();
    let spring = db::projects::create_project(&conn, "Spring launch", "#78DCC0", t).unwrap();
    let trips = db::boards::create_board(&conn, "Trips 2027", t).unwrap();
    for (title, project, board) in [
        ("Scatoloni per lo studio", Some(&studio.id), None),
        ("Elettricista per il nuovo spazio", Some(&studio.id), None),
        ("Shortlist locali per il lancio", Some(&spring.id), None),
        ("Itinerario Giappone", None, Some(&trips.id)),
        ("Voli per Tokyo", None, Some(&trips.id)),
    ] {
        db::items::create_item(
            &conn,
            NewItem {
                kind: ItemType::Task,
                title: title.into(),
                project_id: project.cloned(),
                board_id: board.cloned(),
                ..Default::default()
            },
            t,
        )
        .unwrap();
    }
    (studio.id, trips.id)
}

pub async fn run(app: AppHandle) {
    tokio::time::sleep(Duration::from_secs(3)).await;
    say("start");
    windows_phase(&app).await;

    let (studio_id, trips_id) = seed(&app);
    let voice = Arc::clone(&app.state::<Arc<VoiceService>>());
    let cases = [
        (
            "Alice",
            "Ricordami di chiedere ai traslocatori se portano anche la stampante",
        ),
        (
            "Alice",
            "Per il viaggio in Giappone devo prenotare i treni tra Tokyo e Kyoto",
        ),
        (
            "Alice",
            "Mi è venuto in mente che per la newsletter potremmo raccontare ogni mese un cliente diverso, magari con una breve intervista di tre domande e una foto, così diventa anche materiale per i social",
        ),
    ];

    for (i, (voice_name, text)) in cases.iter().enumerate() {
        say(&format!("--- spoken: {text}"));
        let samples = speech(voice_name, text, i);
        let t = Instant::now();
        let heard = voice.process(samples, Finish::Save).await;
        say(&format!(
            "heard in {} ms: {:?}",
            t.elapsed().as_millis(),
            heard
        ));
        check("transcribed something", heard.is_some());

        // Wait for the background AI capture to finish.
        let start = Instant::now();
        loop {
            let done = {
                let state = app.state::<AppState>();
                let conn = state.db.lock().unwrap();
                conn.query_row(
                    "SELECT COUNT(*) FROM captures WHERE status IN ('pending','processing')",
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap()
                    == 0
            };
            if done || start.elapsed() > Duration::from_secs(90) {
                say(&format!(
                    "capture finished in {} ms",
                    start.elapsed().as_millis()
                ));
                break;
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }

        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        let row: (String, String, String, Option<String>, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT i.id, i.kind, i.title, i.body, i.project_id, p.board_id
                 FROM items i LEFT JOIN placements p ON p.item_id = i.id
                 WHERE i.id = (SELECT item_id FROM captures ORDER BY created_at DESC, rowid DESC LIMIT 1)",
                [],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )
            .unwrap();
        let links: Vec<String> = {
            let mut st = conn
                .prepare(
                    "SELECT t.title FROM connections c JOIN items t
                       ON t.id = CASE WHEN c.from_item = ?1 THEN c.to_item ELSE c.from_item END
                     WHERE c.from_item = ?1 OR c.to_item = ?1",
                )
                .unwrap();
            st.query_map([&row.0], |r| r.get(0))
                .unwrap()
                .map(|x| x.unwrap())
                .collect()
        };
        say(&format!(
            "card: kind={} title={:?}\n         body={:?}\n         project={:?} board={:?} links={:?}",
            row.1, row.2, row.3, row.4, row.5, links
        ));
        match i {
            0 => {
                check(
                    "→ filed in project Studio move",
                    row.4.as_deref() == Some(&studio_id),
                );
                check(
                    "→ short title (≤ 7 words)",
                    row.2.split_whitespace().count() <= 7,
                );
            }
            1 => {
                check(
                    "→ filed on board Trips 2027",
                    row.5.as_deref() == Some(&trips_id) && row.4.is_none(),
                );
                check(
                    "→ linked to a Japan/Tokyo card",
                    links
                        .iter()
                        .any(|l| l.contains("Giappone") || l.contains("Tokyo")),
                );
            }
            _ => {
                check(
                    "→ long note has summary + original",
                    row.3.as_deref().is_some_and(|b| b.contains("You said:")),
                );
                check(
                    "→ short title (≤ 7 words)",
                    row.2.split_whitespace().count() <= 7,
                );
            }
        }
    }

    say("done — quitting (exercises the clean-exit path)");
    app.exit(0);
}
