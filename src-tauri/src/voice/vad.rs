//! Energy-based voice activity detection: an adaptive noise floor, a speech
//! threshold above it, and an endpoint after enough trailing silence.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadEvent {
    None,
    SpeechStart,
    /// Speech happened and has been followed by enough silence: stop listening.
    Endpoint,
}

#[derive(Debug, Clone)]
pub struct Endpointer {
    noise: f32,
    speaking: bool,
    heard_speech_ms: u32,
    silence_ms: u32,
    total_ms: u32,
    /// Trailing silence that ends a capture.
    pub silence_end_ms: u32,
    /// Minimum speech before an endpoint is allowed.
    pub min_speech_ms: u32,
    /// Hard cap on one capture.
    pub max_ms: u32,
}

impl Default for Endpointer {
    fn default() -> Self {
        Self {
            noise: 0.004,
            speaking: false,
            heard_speech_ms: 0,
            silence_ms: 0,
            total_ms: 0,
            silence_end_ms: 1400,
            min_speech_ms: 350,
            max_ms: 60_000,
        }
    }
}

impl Endpointer {
    fn threshold(&self) -> f32 {
        (self.noise * 3.0).max(0.012)
    }

    pub fn heard_speech(&self) -> bool {
        self.heard_speech_ms >= self.min_speech_ms
    }

    /// Feeds the RMS of the latest `frame_ms` of audio.
    pub fn push(&mut self, rms: f32, frame_ms: u32) -> VadEvent {
        self.total_ms += frame_ms;
        if self.total_ms >= self.max_ms {
            return VadEvent::Endpoint;
        }
        if rms > self.threshold() {
            self.silence_ms = 0;
            self.heard_speech_ms += frame_ms;
            if !self.speaking {
                self.speaking = true;
                return VadEvent::SpeechStart;
            }
            VadEvent::None
        } else {
            // Track the room's noise floor only while nobody speaks.
            self.noise = self.noise * 0.95 + rms * 0.05;
            self.speaking = false;
            self.silence_ms += frame_ms;
            if self.heard_speech() && self.silence_ms >= self.silence_end_ms {
                VadEvent::Endpoint
            } else {
                VadEvent::None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(e: &mut Endpointer, rms: f32, ms: u32) -> Vec<VadEvent> {
        (0..ms / 50).map(|_| e.push(rms, 50)).collect()
    }

    #[test]
    fn silence_alone_never_ends_the_capture_early() {
        let mut e = Endpointer::default();
        assert!(
            run(&mut e, 0.002, 5000)
                .iter()
                .all(|v| *v == VadEvent::None)
        );
    }

    #[test]
    fn speech_then_silence_endpoints() {
        let mut e = Endpointer::default();
        run(&mut e, 0.002, 500);
        let events = run(&mut e, 0.08, 1000);
        assert_eq!(events[0], VadEvent::SpeechStart);
        let tail = run(&mut e, 0.002, 2000);
        assert!(tail.contains(&VadEvent::Endpoint));
        // Not before the configured silence.
        let first = tail.iter().position(|v| *v == VadEvent::Endpoint).unwrap();
        assert!((first as u32 + 1) * 50 >= e.silence_end_ms);
    }

    #[test]
    fn a_short_click_is_not_speech() {
        let mut e = Endpointer::default();
        run(&mut e, 0.08, 100);
        assert!(!run(&mut e, 0.002, 3000).contains(&VadEvent::Endpoint));
    }

    #[test]
    fn hard_cap_ends_the_capture() {
        let mut e = Endpointer {
            max_ms: 1000,
            ..Default::default()
        };
        assert!(run(&mut e, 0.08, 1100).contains(&VadEvent::Endpoint));
    }
}
