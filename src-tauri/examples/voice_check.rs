//! Voice smoke test without a microphone: synthesizes speech with macOS `say`,
//! runs the endpointer over it (with trailing silence) and transcribes it with
//! the installed Whisper model, reporting latency.
//!
//! cargo run --release --example voice_check -- <model.bin>

use noma_lib::voice::{
    audio::rms,
    stt::Stt,
    vad::{Endpointer, VadEvent},
};
use std::{path::PathBuf, process::Command, time::Instant};

const CASES: &[(&str, &str)] = &[
    ("Alice", "Ricordami di chiamare Marta domani alle dieci"),
    (
        "Samantha",
        "Book the photographer for the launch shoot next Monday",
    ),
    ("Alice", "Idea: una newsletter mensile per i clienti"),
];

fn wav16k(voice: &str, text: &str, dir: &std::path::Path, i: usize) -> Vec<f32> {
    let aiff = dir.join(format!("c{i}.aiff"));
    let wav = dir.join(format!("c{i}.wav"));
    assert!(
        Command::new("say")
            .args(["-v", voice, "-o"])
            .arg(&aiff)
            .arg(text)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("afconvert")
            .args(["-f", "WAVE", "-d", "LEI16@16000", "-c", "1"])
            .arg(&aiff)
            .arg(&wav)
            .status()
            .unwrap()
            .success()
    );
    let bytes = std::fs::read(&wav).unwrap();
    // Find the "data" chunk.
    let pos = bytes.windows(4).position(|w| w == b"data").unwrap() + 8;
    bytes[pos..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| i16::from_le_bytes(*c) as f32 / 32768.0)
        .collect()
}

fn main() {
    let model = PathBuf::from(std::env::args().nth(1).expect("model path"));
    let dir = std::env::temp_dir().join("noma-voice-check");
    std::fs::create_dir_all(&dir).unwrap();

    let t = Instant::now();
    let stt = Stt::load(&model, "check").expect("load");
    println!("load: {} ms", t.elapsed().as_millis());

    for (i, (voice, text)) in CASES.iter().enumerate() {
        let mut audio = wav16k(voice, text, &dir, i);
        let speech_ms = audio.len() / 16;
        audio.extend(std::iter::repeat_n(0.0005f32, 16_000 * 2)); // 2 s near-silence

        let mut vad = Endpointer::default();
        let mut endpoint_ms = None;
        for (f, frame) in audio.chunks(960).enumerate() {
            if vad.push(rms(frame), 60) == VadEvent::Endpoint {
                endpoint_ms = Some((f + 1) * 60);
                break;
            }
        }
        let t = Instant::now();
        let out = stt
            .transcribe(&audio[..audio.len() - 16_000], None)
            .expect("transcribe");
        println!(
            "[{i}] speech {speech_ms} ms · endpoint {:?} ms · stt {} ms · lang {:?}\n    in:  {text}\n    out: {}",
            endpoint_ms,
            t.elapsed().as_millis(),
            out.language,
            out.text
        );
    }
}
