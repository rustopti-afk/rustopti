/// Security checks for release builds.
pub fn perform_all_security_checks() {
    #[cfg(not(debug_assertions))]
    {
        check_suspicious_processes();
    }
}

/// Multi-method debugger detection.
/// Returns true if any indicator suggests a debugger is present.
#[cfg(not(debug_assertions))]
pub fn is_debugger_attached() -> bool {
    // Method 1: Windows API — IsDebuggerPresent
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::Diagnostics::Debug::IsDebuggerPresent;
        unsafe {
            if IsDebuggerPresent().as_bool() {
                return true;
            }
        }
    }

    // Method 2: Process name check (catch renamed debuggers by window class later,
    // but still check common names — not everyone renames)
    use sysinfo::System;
    let suspect_fragments = [
        "x64dbg", "x32dbg", "ollydbg",
        "ida64", "ida32", "idaq",
        "windbg", "dbgview",
        "ghidra", "radare2", "r2",
        "dnspy", "de4dot", "ilspy",
        "processhacker", "apimonitor",
        "fiddler", "wireshark",
        "httpdebugger",
    ];

    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    for (_, process) in sys.processes() {
        let name = process.name().to_string_lossy().to_lowercase();
        for &frag in &suspect_fragments {
            if name.contains(frag) {
                return true;
            }
        }
    }

    false
}

// Debug builds — always returns false
#[cfg(debug_assertions)]
pub fn is_debugger_attached() -> bool {
    false
}

#[cfg(not(debug_assertions))]
fn check_suspicious_processes() {
    use sysinfo::System;

    let mut s = System::new_all();
    s.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let cheat_tools = ["cheatengine"];

    for (_, process) in s.processes() {
        let name = process.name().to_string_lossy().to_lowercase();
        for tool in cheat_tools {
            if name.contains(tool) {
                eprintln!("[RustOpti] Warning: detected suspicious tool running. Some features may be disabled.");
                return;
            }
        }
    }
}
