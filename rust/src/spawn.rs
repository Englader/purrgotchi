//! Cross-platform detached subprocess spawn for fire-and-forget workers.
//!
//! Used by `hooks::turn` to launch `purrgotchi react` without blocking the
//! UserPromptSubmit hook. Failures (couldn't resolve current_exe, spawn failed)
//! are silent — the caller falls through to its existing behaviour.

use std::process::{Command, Stdio};

/// Spawn `purrgotchi <args...>` detached from the current process.
/// Drops the resulting `Child` handle so the parent can exit immediately
/// without waiting. All stdio is nulled so the child cannot keep the parent's
/// pipes open.
pub fn detached_self(args: &[&str]) {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return,
    };
    let mut cmd = Command::new(exe);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW);
    }

    // Intentionally drop the Child handle — parent exits without waiting.
    let _ = cmd.spawn();
}
