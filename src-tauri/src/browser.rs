//! Opening the sign-in URL in the default browser (261011-sso-login,
//! functional-design review: URL; Windows since 261015-windows-support
//! FR4.5).
//!
//! Only the start of the browser lives here; whether the URL may be opened
//! at all (the allow-list and the running attempt) is decided by the
//! caller, `open_sso_url` in `lib.rs`, and does not depend on the OS.
//! Neither side goes through a shell: on Unix the URL is one argument of
//! `/usr/bin/open` (a fixed path); on Windows it is handed to
//! `ShellExecuteW` with the `open` verb, which asks the OS for the default
//! browser.

use std::io;

/// Hands `url` to the default browser; returns once the OS took it (the
/// browser itself may still be starting).
///
/// # Errors
/// The OS error when the browser could not be asked to open the URL.
#[cfg(unix)]
pub fn open_url(url: &str) -> io::Result<()> {
    use std::process::{Command, Stdio};

    let mut child = Command::new("/usr/bin/open")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    // `open` ends at once; reap it so no zombie is left.
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// See the Unix version above.
///
/// # Errors
/// The code `ShellExecuteW` answered with when the OS refused.
#[cfg(windows)]
pub fn open_url(url: &str) -> io::Result<()> {
    use std::ptr;

    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    /// `ShellExecuteW` answers a value above 32 on success; below it is an
    /// error code (`SE_ERR_*`).
    const SUCCESS_THRESHOLD: usize = 32;

    let verb = wide("open");
    let file = wide(url);
    // SAFETY: every string is NUL-terminated UTF-16 that outlives the call;
    // the window, parameters and directory may be null; the verb `open`
    // on an https URL asks the registered default browser to show it.
    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    let code = result as usize;
    if code > SUCCESS_THRESHOLD {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "ShellExecuteW refused to open the URL (code {code})"
        )))
    }
}

/// `text` as NUL-terminated UTF-16 for the Windows API.
#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
