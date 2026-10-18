//! Hidden `term-write` subcommand: write the raw bytes in `PURR_SEQ` to the
//! real terminal the session runs in. Windows only; the mod's Linux path uses
//! `sh`/`ps` itself. Exit codes: 0 written, 1 no reachable console, 2 no input.
//! Never prints to stdout; diagnostics go through `debug::log`.

use std::collections::HashMap;

use crate::debug;

const MAX_DEPTH: usize = 32;

/// Ancestors of `start` (excluding `start`), nearest first, from a pid ->
/// parent-pid map. Stops on a cycle (pid reuse), a missing parent, pid 0,
/// and after `max_depth` entries.
pub(crate) fn ancestor_chain(procs: &HashMap<u32, u32>, start: u32, max_depth: usize) -> Vec<u32> {
    let mut chain = Vec::new();
    let mut seen = vec![start];
    let mut cur = start;
    while chain.len() < max_depth {
        let Some(&parent) = procs.get(&cur) else { break };
        if parent == 0 || seen.contains(&parent) {
            break;
        }
        chain.push(parent);
        seen.push(parent);
        cur = parent;
    }
    chain
}

pub fn run() -> i32 {
    let seq = match std::env::var_os("PURR_SEQ") {
        Some(v) if !v.is_empty() => v.to_string_lossy().into_owned().into_bytes(),
        _ => return 2,
    };
    write_to_console(&seq)
}

#[cfg(not(windows))]
fn write_to_console(_seq: &[u8]) -> i32 {
    debug::log("term-write: not supported on this platform");
    1
}

#[cfg(windows)]
fn write_to_console(seq: &[u8]) -> i32 {
    use win::*;
    let (parents, names) = snapshot();
    let me = std::process::id();
    let chain = ancestor_chain(&parents, me, MAX_DEPTH);
    debug::log(&format!("term-write: pid={me} ancestors={chain:?}"));
    free_console();
    for pid in chain {
        let exe = names.get(&pid).map(String::as_str).unwrap_or("?");
        if !attach_console(pid) {
            debug::log(&format!("term-write: attach failed pid={pid} exe={exe}"));
            continue;
        }
        if console_window() == 0 {
            debug::log(&format!("term-write: pid={pid} exe={exe} has no window, skipping"));
            free_console();
            continue;
        }
        if write_conout(seq) {
            debug::log(&format!("term-write: wrote {} bytes via pid={pid} exe={exe}", seq.len()));
            return 0;
        }
        debug::log(&format!("term-write: CONOUT$ write failed pid={pid} exe={exe}"));
        free_console();
    }
    debug::log("term-write: no console found");
    1
}

#[cfg(windows)]
mod win {
    use std::collections::HashMap;

    type Handle = isize;
    const INVALID_HANDLE_VALUE: Handle = -1;
    const TH32CS_SNAPPROCESS: u32 = 0x2;
    const GENERIC_READ: u32 = 0x8000_0000;
    const GENERIC_WRITE: u32 = 0x4000_0000;
    const FILE_SHARE_READ: u32 = 1;
    const FILE_SHARE_WRITE: u32 = 2;
    const OPEN_EXISTING: u32 = 3;

    // Win32 type name — intentional upper-case for FFI fidelity.
    #[allow(clippy::upper_case_acronyms)]
    #[repr(C)]
    struct PROCESSENTRY32W {
        size: u32,
        usage: u32,
        pid: u32,
        heap_id: usize,
        module_id: u32,
        threads: u32,
        parent_pid: u32,
        pri_class_base: i32,
        flags: u32,
        exe_file: [u16; 260],
    }

    unsafe extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> Handle;
        fn Process32FirstW(snap: Handle, entry: *mut PROCESSENTRY32W) -> i32;
        fn Process32NextW(snap: Handle, entry: *mut PROCESSENTRY32W) -> i32;
        fn CloseHandle(h: Handle) -> i32;
        fn FreeConsole() -> i32;
        fn AttachConsole(pid: u32) -> i32;
        fn GetConsoleWindow() -> isize;
        fn CreateFileW(
            name: *const u16, access: u32, share: u32, sec: *const u8,
            disposition: u32, flags: u32, template: Handle,
        ) -> Handle;
        fn WriteFile(h: Handle, buf: *const u8, len: u32, written: *mut u32, ovl: *mut u8) -> i32;
    }

    /// pid -> parent pid, and pid -> exe name. Empty maps if the snapshot fails.
    pub fn snapshot() -> (HashMap<u32, u32>, HashMap<u32, String>) {
        let mut parents = HashMap::new();
        let mut names = HashMap::new();
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE || snap == 0 {
                return (parents, names);
            }
            let mut e: PROCESSENTRY32W = std::mem::zeroed();
            e.size = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut ok = Process32FirstW(snap, &mut e) != 0;
            while ok {
                let len = e.exe_file.iter().position(|&c| c == 0).unwrap_or(e.exe_file.len());
                parents.insert(e.pid, e.parent_pid);
                names.insert(e.pid, String::from_utf16_lossy(&e.exe_file[..len]));
                ok = Process32NextW(snap, &mut e) != 0;
            }
            CloseHandle(snap);
        }
        (parents, names)
    }

    pub fn free_console() { unsafe { FreeConsole(); } }
    pub fn attach_console(pid: u32) -> bool { unsafe { AttachConsole(pid) != 0 } }
    pub fn console_window() -> isize { unsafe { GetConsoleWindow() } }

    /// Open the attached console's output buffer and write every byte.
    pub fn write_conout(seq: &[u8]) -> bool {
        let name: Vec<u16> = "CONOUT$".encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let h = CreateFileW(
                name.as_ptr(), GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE, std::ptr::null(),
                OPEN_EXISTING, 0, 0,
            );
            if h == INVALID_HANDLE_VALUE || h == 0 {
                return false;
            }
            let mut done = 0usize;
            let mut ok = true;
            while done < seq.len() {
                let mut n = 0u32;
                let rest = &seq[done..];
                if WriteFile(h, rest.as_ptr(), rest.len() as u32, &mut n, std::ptr::null_mut()) == 0 || n == 0 {
                    ok = false;
                    break;
                }
                done += n as usize;
            }
            CloseHandle(h);
            ok
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(u32, u32)]) -> HashMap<u32, u32> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn normal_chain_nearest_first() {
        let m = map(&[(10, 20), (20, 30), (30, 40), (40, 0)]);
        assert_eq!(ancestor_chain(&m, 10, 32), vec![20, 30, 40]);
    }

    #[test]
    fn cycle_stops() {
        let m = map(&[(1, 2), (2, 3), (3, 2)]);
        assert_eq!(ancestor_chain(&m, 1, 32), vec![2, 3]);
    }

    #[test]
    fn cycle_back_to_start_stops() {
        let m = map(&[(1, 2), (2, 1)]);
        assert_eq!(ancestor_chain(&m, 1, 32), vec![2]);
    }

    #[test]
    fn self_parent_is_empty() {
        let m = map(&[(5, 5)]);
        assert!(ancestor_chain(&m, 5, 32).is_empty());
    }

    #[test]
    fn missing_parent_keeps_the_known_ancestor() {
        let m = map(&[(1, 2)]);
        assert_eq!(ancestor_chain(&m, 1, 32), vec![2]);
    }

    #[test]
    fn depth_cap() {
        let pairs: Vec<(u32, u32)> = (1..=100).map(|p| (p, p + 1)).collect();
        let m = map(&pairs);
        let c = ancestor_chain(&m, 1, 32);
        assert_eq!(c.len(), 32);
        assert_eq!(c[0], 2);
        assert_eq!(c[31], 33);
    }

    #[test]
    fn start_not_in_map() {
        assert!(ancestor_chain(&map(&[(1, 2)]), 99, 32).is_empty());
    }

    #[test]
    fn pid_zero_parent_stops() {
        let m = map(&[(1, 0)]);
        assert!(ancestor_chain(&m, 1, 32).is_empty());
    }
}
