//! Clipboard helpers.
//!
//! Copying must work both locally and over SSH. Two mechanisms are used together:
//!
//! 1. **OSC 52** — an escape sequence that asks the *terminal emulator* to place
//!    text into the user's local clipboard. This works transparently over SSH
//!    (the bytes travel back through the same terminal connection), provided the
//!    terminal supports OSC 52 (iTerm2, kitty, Alacritty, WezTerm, recent
//!    xterm, tmux with `set-clipboard on`, etc.).
//! 2. **System clipboard** (`arboard`) — sets the OS clipboard directly. This is
//!    the reliable path when running locally.
//!
//! Both are attempted; success of either is treated as success.

use std::io::Write;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

/// Outcome of a clipboard copy attempt.
pub enum CopyOutcome {
    /// At least one mechanism succeeded.
    Ok,
    /// Both mechanisms failed; contains a human-readable reason.
    Failed(String),
}

/// Copy `text` to the clipboard using OSC 52 and the system clipboard.
///
/// Returns [`CopyOutcome::Ok`] if either mechanism succeeds.
pub fn copy(text: &str) -> CopyOutcome {
    let osc_ok = write_osc52(text).is_ok();
    let system_ok = write_system_clipboard(text).is_ok();

    if osc_ok || system_ok {
        CopyOutcome::Ok
    } else {
        CopyOutcome::Failed("no clipboard mechanism available".to_string())
    }
}

/// Set the OS clipboard via `arboard`.
///
/// On macOS, NSPasteboard occasionally writes diagnostic lines straight to the
/// process's stderr (fd 2), which corrupts the TUI. We silence stderr for the
/// duration of the call via [`SilencedStderr`].
fn write_system_clipboard(text: &str) -> Result<(), String> {
    let _guard = SilencedStderr::new();
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())
}

/// RAII guard that redirects stderr (fd 2) to /dev/null while alive, restoring
/// it on drop. Only active on macOS; a no-op elsewhere.
struct SilencedStderr {
    #[cfg(target_os = "macos")]
    saved_fd: Option<libc::c_int>,
}

impl SilencedStderr {
    #[cfg(target_os = "macos")]
    fn new() -> Self {
        // SAFETY: dup/open/dup2 on the well-known stderr fd; failures fall back
        // to leaving stderr untouched.
        unsafe {
            let saved = libc::dup(libc::STDERR_FILENO);
            if saved < 0 {
                return Self { saved_fd: None };
            }
            let devnull = libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY);
            if devnull < 0 {
                libc::close(saved);
                return Self { saved_fd: None };
            }
            libc::dup2(devnull, libc::STDERR_FILENO);
            libc::close(devnull);
            Self {
                saved_fd: Some(saved),
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn new() -> Self {
        Self {}
    }
}

impl Drop for SilencedStderr {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(saved) = self.saved_fd.take() {
            // SAFETY: restore the original stderr and close the saved descriptor.
            unsafe {
                libc::dup2(saved, libc::STDERR_FILENO);
                libc::close(saved);
            }
        }
    }
}

/// Emit an OSC 52 sequence to stdout to set the terminal's clipboard.
///
/// When running inside tmux, the sequence is wrapped in tmux's passthrough so
/// it reaches the outer terminal.
fn write_osc52(text: &str) -> std::io::Result<()> {
    let encoded = BASE64.encode(text.as_bytes());

    // Base OSC 52 sequence: ESC ] 52 ; c ; <base64> BEL
    let inner = format!("\x1b]52;c;{encoded}\x07");

    let payload = if is_tmux() {
        // tmux passthrough: ESC P tmux ; <escaped ESCs> ESC \
        // Each ESC inside the wrapped sequence must be doubled.
        let escaped = inner.replace('\x1b', "\x1b\x1b");
        format!("\x1bPtmux;{escaped}\x1b\\")
    } else {
        inner
    };

    let mut stdout = std::io::stdout();
    stdout.write_all(payload.as_bytes())?;
    stdout.flush()
}

/// Detect whether we are running inside a tmux session.
fn is_tmux() -> bool {
    std::env::var_os("TMUX").is_some()
        || std::env::var("TERM")
            .map(|t| t.starts_with("tmux") || t.starts_with("screen"))
            .unwrap_or(false)
}
