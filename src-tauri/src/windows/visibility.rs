//! Pure visibility toggle logic, kept separate so it can be unit-tested
//! without a running Tauri app.

/// Returns the next visibility state for a window that is currently `is_visible`.
///
/// A toggle is a pure flip: pressing the shortcut twice always returns to the
/// starting state, so rapid presses cannot drift or create duplicate windows.
pub fn toggle_visibility(is_visible: bool) -> bool {
    !is_visible
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_visibility_is_idempotent() {
        assert!(toggle_visibility(false));
        assert!(!toggle_visibility(true));
        assert!(!toggle_visibility(toggle_visibility(false)));
        // Five rapid presses from hidden end visible; six end hidden.
        let mut v = false;
        for _ in 0..5 {
            v = toggle_visibility(v);
        }
        assert!(v);
        assert!(!toggle_visibility(v));
    }
}
