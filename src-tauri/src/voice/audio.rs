//! Microphone capture with cpal. The input stream lives on its own thread
//! (cpal streams are not `Send` on macOS); samples are down-mixed, resampled
//! to 16 kHz mono and appended to a shared buffer.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat};

use super::VoiceError;

pub const SAMPLE_RATE: u32 = 16_000;
/// Hard cap on one capture: 90 s of audio.
pub const MAX_SAMPLES: usize = SAMPLE_RATE as usize * 90;

/// Streaming linear resampler (any rate → 16 kHz).
#[derive(Debug, Clone)]
pub struct Resampler {
    step: f64,
    t: f64,
    prev: Option<f32>,
}

impl Resampler {
    pub fn new(input_rate: u32) -> Self {
        Self {
            step: f64::from(input_rate) / f64::from(SAMPLE_RATE),
            t: 0.0,
            prev: None,
        }
    }

    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        for &x in input {
            let Some(prev) = self.prev else {
                self.prev = Some(x);
                out.push(x);
                self.t = self.step;
                continue;
            };
            while self.t <= 1.0 {
                out.push(prev + (x - prev) * self.t as f32);
                self.t += self.step;
            }
            self.t -= 1.0;
            self.prev = Some(x);
        }
    }
}

/// Root-mean-square level of a block of samples.
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

pub struct Recorder {
    buffer: Arc<Mutex<Vec<f32>>>,
    level: Arc<AtomicU32>,
    stop: mpsc::Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl Recorder {
    /// Opens the default input device and starts recording.
    pub fn start() -> Result<Self, VoiceError> {
        let buffer = Arc::new(Mutex::new(Vec::with_capacity(SAMPLE_RATE as usize * 10)));
        let level = Arc::new(AtomicU32::new(0));
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), VoiceError>>();

        let buf = Arc::clone(&buffer);
        let lvl = Arc::clone(&level);
        let thread = std::thread::Builder::new()
            .name("noma-mic".into())
            .spawn(move || {
                let stream = match open_stream(buf, lvl) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                if let Err(e) = stream.play() {
                    let _ = ready_tx.send(Err(VoiceError::Audio(e.to_string())));
                    return;
                }
                let _ = ready_tx.send(Ok(()));
                let _ = stop_rx.recv();
                drop(stream);
            })
            .map_err(|e| VoiceError::Audio(e.to_string()))?;

        ready_rx
            .recv()
            .map_err(|_| VoiceError::Audio("microphone thread ended".into()))??;
        Ok(Self {
            buffer,
            level,
            stop: stop_tx,
            thread: Some(thread),
        })
    }

    /// Current input level (RMS of the latest block), 0..1.
    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    pub fn len(&self) -> usize {
        self.buffer.lock().map(|b| b.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// A copy of the samples recorded so far (from `from` on).
    pub fn copy_from(&self, from: usize) -> Vec<f32> {
        self.buffer
            .lock()
            .map(|b| b[from.min(b.len())..].to_vec())
            .unwrap_or_default()
    }

    /// Stops recording and returns everything that was captured.
    pub fn stop(mut self) -> Vec<f32> {
        let _ = self.stop.send(());
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        std::mem::take(&mut *self.buffer.lock().expect("buffer lock"))
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn open_stream(
    buffer: Arc<Mutex<Vec<f32>>>,
    level: Arc<AtomicU32>,
) -> Result<cpal::Stream, VoiceError> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or(VoiceError::NoInput)?;
    let config = device
        .default_input_config()
        .map_err(|e| VoiceError::Audio(e.to_string()))?;
    let channels = usize::from(config.channels()).max(1);
    let rate = config.sample_rate();
    let format = config.sample_format();
    let err_fn = |e: cpal::Error| log::warn!("microphone stream error: {e}");

    fn make<T>(
        channels: usize,
        rate: u32,
        buffer: Arc<Mutex<Vec<f32>>>,
        level: Arc<AtomicU32>,
    ) -> impl FnMut(&[T], &cpal::InputCallbackInfo) + Send + 'static
    where
        T: Sample + Send + 'static,
        f32: FromSample<T>,
    {
        let mut resampler = Resampler::new(rate);
        let mut mono = Vec::new();
        let mut out = Vec::new();
        move |data: &[T], _| {
            mono.clear();
            for frame in data.chunks(channels) {
                let sum: f32 = frame.iter().map(|&s| f32::from_sample(s)).sum();
                mono.push(sum / channels as f32);
            }
            out.clear();
            resampler.process(&mono, &mut out);
            level.store(rms(&mono).to_bits(), Ordering::Relaxed);
            if let Ok(mut b) = buffer.try_lock()
                && b.len() < MAX_SAMPLES
            {
                b.extend_from_slice(&out);
            }
        }
    }

    let stream = match format {
        SampleFormat::F32 => device.build_input_stream(
            config.into(),
            make::<f32>(channels, rate, buffer, level),
            err_fn,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            config.into(),
            make::<i16>(channels, rate, buffer, level),
            err_fn,
            None,
        ),
        SampleFormat::I32 => device.build_input_stream(
            config.into(),
            make::<i32>(channels, rate, buffer, level),
            err_fn,
            None,
        ),
        other => {
            return Err(VoiceError::Audio(format!(
                "unsupported sample format {other}"
            )));
        }
    };
    stream.map_err(|e| VoiceError::Audio(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resampler_48k_to_16k_keeps_a_third_of_the_samples() {
        let mut r = Resampler::new(48_000);
        let input: Vec<f32> = (0..4800).map(|i| (i as f32 / 100.0).sin()).collect();
        let mut out = Vec::new();
        r.process(&input[..2400], &mut out);
        r.process(&input[2400..], &mut out);
        assert!((out.len() as i64 - 1600).abs() <= 2, "{}", out.len());
    }

    #[test]
    fn resampler_16k_is_identity_length() {
        let mut r = Resampler::new(16_000);
        let mut out = Vec::new();
        r.process(&[0.1; 1000], &mut out);
        assert!((out.len() as i64 - 1000).abs() <= 1);
    }

    #[test]
    fn rms_of_constant_signal() {
        assert!((rms(&[0.5; 100]) - 0.5).abs() < 1e-6);
        assert_eq!(rms(&[]), 0.0);
    }
}
