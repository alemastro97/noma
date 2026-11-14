//! The `noma://` URL scheme: `noma://capture?text=…`, `noma://open?item=…`
//! and `noma://today`. The CLI and other apps use it to reach NOMA.

use tauri::{AppHandle, Emitter, Url};

use crate::commands::ai::{CaptureContext, create_capture};
use crate::filed::{OPEN_ITEM, OPEN_TODAY};
use crate::windows;

/// Longest capture accepted from a link.
const TEXT_MAX: usize = 4_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    Capture(String),
    Open(String),
    Today,
}

pub fn parse(url: &Url) -> Option<Link> {
    if url.scheme() != "noma" {
        return None;
    }
    let param = |key: &str| {
        url.query_pairs()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    // `noma://capture` puts the action in the host, `noma:capture` in the path.
    let action = url
        .host_str()
        .map(str::to_string)
        .unwrap_or_else(|| url.path().trim_matches('/').to_string());
    match action.as_str() {
        "capture" => param("text").map(|t| Link::Capture(t.chars().take(TEXT_MAX).collect())),
        "open" => param("item").map(Link::Open),
        "today" => Some(Link::Today),
        _ => None,
    }
}

pub fn handle(app: &AppHandle, url: &Url) {
    match parse(url) {
        Some(Link::Capture(text)) => {
            // Any web page or mail can fire this link: one capture every few seconds.
            static LAST: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);
            if let Ok(mut last) = LAST.lock() {
                if last.is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(3)) {
                    log::warn!("noma:// capture ignored: too many in a row");
                    return;
                }
                *last = Some(std::time::Instant::now());
            }
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) =
                    create_capture(&app, "text", &text, &CaptureContext::default()).await
                {
                    log::warn!("noma:// capture failed: {e}");
                }
            });
        }
        Some(Link::Open(id)) => {
            windows::show_main(app);
            let _ = app.emit(OPEN_ITEM, id);
        }
        Some(Link::Today) => {
            windows::show_main(app);
            let _ = app.emit(OPEN_TODAY, ());
        }
        None => log::warn!("unknown noma:// link"),
    }
}

/// Registers the handler for links opened while NOMA runs (or launches it).
pub fn setup(app: &AppHandle) {
    use tauri_plugin_deep_link::DeepLinkExt;
    let handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        for url in event.urls() {
            self::handle(&handle, &url);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(s: &str) -> Option<Link> {
        parse(&Url::parse(s).unwrap())
    }

    #[test]
    fn parses_the_three_actions() {
        assert_eq!(
            link("noma://capture?text=buy%20milk%20tomorrow"),
            Some(Link::Capture("buy milk tomorrow".into()))
        );
        assert_eq!(link("noma://open?item=abc"), Some(Link::Open("abc".into())));
        assert_eq!(link("noma://today"), Some(Link::Today));
        assert_eq!(link("noma:today"), Some(Link::Today));
        assert_eq!(link("noma://capture?text=%20"), None);
        assert_eq!(link("noma://delete?item=abc"), None);
        assert_eq!(link("https://capture?text=x"), None);
    }
}
