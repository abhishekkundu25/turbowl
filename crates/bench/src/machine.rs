//! What the numbers were measured on: CPU, cores, OS, memory, load average,
//! the repository's commit, the versions. And the process's peak memory.

use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

/// The 1, 5 and 15 minute load averages, or `None` where the system has none.
pub(crate) fn load_average() -> Option<[f64; 3]> {
    let mut loads = [0f64; 3];
    // SAFETY: `getloadavg` writes at most `nelem` (3) doubles into the buffer,
    // which holds 3.
    #[allow(unsafe_code)]
    let n = unsafe { libc::getloadavg(loads.as_mut_ptr(), 3) };
    (n == 3).then_some(loads)
}

/// This process's peak resident set size in bytes, as the kernel counts it.
/// `ru_maxrss` is in bytes on macOS and in kilobytes on Linux and the BSDs.
pub(crate) fn peak_rss_bytes() -> u64 {
    // SAFETY: `rusage` is plain data; all zeroes is a valid value.
    #[allow(unsafe_code)]
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: `getrusage` writes one `rusage` through a valid pointer.
    #[allow(unsafe_code)]
    let rc = unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    if rc != 0 {
        return 0;
    }
    let max = usage.ru_maxrss.max(0) as u64;
    if cfg!(target_os = "macos") { max } else { max * 1024 }
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

fn cpu_model() -> String {
    if cfg!(target_os = "macos") {
        return run("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_else(|| "unknown".into());
    }
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("model name") || l.starts_with("Model"))
                .and_then(|l| l.split_once(':'))
                .map(|(_, v)| v.trim().to_string())
        })
        .unwrap_or_else(|| "unknown".into())
}

fn os_name() -> String {
    let version = if cfg!(target_os = "macos") {
        run("sw_vers", &["-productVersion"]).map(|v| format!("macOS {v}"))
    } else {
        std::fs::read_to_string("/etc/os-release").ok().and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("PRETTY_NAME="))
                .map(|v| v.trim_matches('"').to_string())
        })
    };
    let kernel = run("uname", &["-r"]).unwrap_or_default();
    format!(
        "{} (kernel {kernel}, {})",
        version.unwrap_or_else(|| std::env::consts::OS.into()),
        std::env::consts::ARCH
    )
}

fn memory_bytes() -> Option<u64> {
    if cfg!(target_os = "macos") {
        return run("sysctl", &["-n", "hw.memsize"]).and_then(|s| s.parse().ok());
    }
    let s = std::fs::read_to_string("/proc/meminfo").ok()?;
    let kb: u64 = s
        .lines()
        .find_map(|l| l.strip_prefix("MemTotal:"))?
        .trim()
        .trim_end_matches("kB")
        .trim()
        .parse()
        .ok()?;
    Some(kb * 1024)
}

/// The commit of the repository at `root` and whether tracked files differ from it.
fn git(root: &Path) -> (Option<String>, Option<bool>) {
    let root = root.to_string_lossy();
    let commit = run("git", &["-C", &root, "rev-parse", "HEAD"]);
    let dirty = Command::new("git")
        .args(["-C", &root, "status", "--porcelain", "--untracked-files=no"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| !o.stdout.is_empty());
    (commit, dirty)
}

/// Today's date and time in UTC, as `YYYY-MM-DDTHH:MM:SSZ`.
pub(crate) fn utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// The machine and build, for the results file's `meta`.
pub(crate) fn describe(root: &Path) -> Value {
    let (commit, dirty) = git(root);
    json!({
        "cpu": cpu_model(),
        "cores": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
        "os": os_name(),
        "memory_bytes": memory_bytes(),
        "commit": commit,
        "dirty": dirty,
        "build": if cfg!(debug_assertions) { "debug" } else { "release" },
        "versions": {
            "bench": env!("CARGO_PKG_VERSION"),
            "turtle_reasoner": env!("CARGO_PKG_VERSION"),
            "reasonable": crate::engine::REASONABLE_VERSION,
            "rustc": env!("BENCH_RUSTC_VERSION"),
        },
    })
}
