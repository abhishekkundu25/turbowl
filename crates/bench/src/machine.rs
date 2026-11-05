//! What the numbers were measured on: CPU, cores, OS, memory, load average,
//! the repository's commit, the versions. And memory: a process's peak, and
//! what a run holds now (for the memory cap).

use serde_json::{Value, json};
use std::path::Path;
use std::process::{Child, Command};
use std::time::Duration;

/// The 1, 5 and 15 minute load averages, or `None` where the system has none.
pub(crate) fn load_average() -> Option<[f64; 3]> {
    let mut loads = [0f64; 3];
    // SAFETY: `getloadavg` writes at most `nelem` (3) doubles into the buffer,
    // which holds 3.
    #[allow(unsafe_code)]
    let n = unsafe { libc::getloadavg(loads.as_mut_ptr(), 3) };
    (n == 3).then_some(loads)
}

/// This process's peak memory so far, in bytes. On macOS it is the peak of
/// its physical footprint over its lifetime (`ri_lifetime_max_phys_footprint`),
/// the measure the memory cap reads ([`footprint_bytes`]) at its highest.
/// Elsewhere it is the peak resident size (`ru_maxrss`, in kilobytes on
/// Linux), which leaves out what was swapped out.
pub(crate) fn own_peak_bytes() -> u64 {
    #[cfg(target_os = "macos")]
    {
        lifetime_peak_footprint(std::process::id())
    }
    #[cfg(not(target_os = "macos"))]
    {
        max_rss_kb(libc::RUSAGE_SELF) * 1024
    }
}

/// The largest `ru_maxrss` of this process (`RUSAGE_SELF`) or of its
/// waited-for descendants (`RUSAGE_CHILDREN`), in kilobytes (Linux).
#[cfg(not(target_os = "macos"))]
fn max_rss_kb(who: libc::c_int) -> u64 {
    // SAFETY: `rusage` is plain data; all zeroes is a valid value.
    #[allow(unsafe_code)]
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: `getrusage` writes one `rusage` through a valid pointer.
    #[allow(unsafe_code)]
    let rc = unsafe { libc::getrusage(who, &mut usage) };
    if rc != 0 { 0 } else { usage.ru_maxrss.max(0) as u64 }
}

/// The peak physical footprint of a process over its lifetime, in bytes,
/// also once it has exited and until it is reaped; 0 when it cannot be read.
#[cfg(target_os = "macos")]
fn lifetime_peak_footprint(pid: u32) -> u64 {
    // SAFETY: `rusage_info_v4` is plain data; all zeroes is a valid value.
    #[allow(unsafe_code)]
    let mut info: libc::rusage_info_v4 = unsafe { std::mem::zeroed() };
    let buffer: *mut libc::rusage_info_v4 = &mut info;
    // SAFETY: with flavor RUSAGE_INFO_V4 the call writes one `rusage_info_v4`.
    #[allow(unsafe_code)]
    let rc = unsafe { libc::proc_pid_rusage(pid as libc::c_int, libc::RUSAGE_INFO_V4, buffer.cast()) };
    if rc == 0 {
        info.ri_lifetime_max_phys_footprint
    } else {
        0
    }
}

/// Waits for a child to exit, at most `timeout`, and returns its exit status
/// and its peak memory in bytes; `None` when it ran over `timeout`, after
/// stopping it. The peak is read as [`own_peak_bytes`] reads this process's:
/// on macOS the child's lifetime peak footprint, read after it exits and
/// before it is reaped; elsewhere the largest `ru_maxrss` of this process's
/// waited-for children (call it with one child at a time).
pub(crate) fn wait_with_peak(
    child: &mut Child,
    timeout: Duration,
) -> std::io::Result<Option<(std::process::ExitStatus, u64)>> {
    let start = std::time::Instant::now();
    loop {
        if let Some(done) = exited(child)? {
            return Ok(Some(done));
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The exit status and peak memory of a child that has exited, reaping it.
#[cfg(target_os = "macos")]
fn exited(child: &mut Child) -> std::io::Result<Option<(std::process::ExitStatus, u64)>> {
    // SAFETY: `siginfo_t` is plain data; all zeroes is a valid value, and
    // `si_pid` stays 0 when no child has exited (WNOHANG).
    #[allow(unsafe_code)]
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    // WNOWAIT leaves the child a zombie, so its usage can still be read.
    // SAFETY: `waitid` writes one `siginfo_t` through a valid pointer.
    #[allow(unsafe_code)]
    let rc = unsafe {
        libc::waitid(
            libc::P_PID,
            child.id() as libc::id_t,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    if rc != 0 {
        let e = std::io::Error::last_os_error();
        return if e.kind() == std::io::ErrorKind::Interrupted {
            Ok(None)
        } else {
            Err(e)
        };
    }
    if info.si_pid == 0 {
        return Ok(None);
    }
    let peak = lifetime_peak_footprint(child.id());
    Ok(Some((child.wait()?, peak)))
}

/// The exit status and peak memory of a child that has exited, reaping it.
#[cfg(not(target_os = "macos"))]
fn exited(child: &mut Child) -> std::io::Result<Option<(std::process::ExitStatus, u64)>> {
    Ok(child
        .try_wait()?
        .map(|status| (status, max_rss_kb(libc::RUSAGE_CHILDREN) * 1024)))
}

/// A process and its descendants, the process first: a run and the engine
/// it starts (Nemo's `nmo`). A process that has exited has none.
pub(crate) fn process_tree(pid: u32) -> Vec<u32> {
    let mut tree = vec![pid];
    let mut i = 0;
    while i < tree.len() {
        tree.extend(children(tree[i]));
        i += 1;
    }
    tree
}

#[cfg(target_os = "macos")]
fn children(pid: u32) -> Vec<u32> {
    let mut pids: Vec<libc::pid_t> = vec![0; 64];
    loop {
        pids.fill(0);
        let size = (pids.len() * std::mem::size_of::<libc::pid_t>()) as libc::c_int;
        // SAFETY: the buffer holds `size` bytes; the call writes at most that many.
        #[allow(unsafe_code)]
        let n = unsafe { libc::proc_listchildpids(pid as libc::pid_t, pids.as_mut_ptr().cast(), size) };
        // Some releases return a count of pids, others of bytes: read at
        // most the buffer (unwritten entries stay 0), and grow it while full.
        let n = (n.max(0) as usize).min(pids.len());
        if n < pids.len() {
            return pids[..n].iter().filter(|&&p| p > 0).map(|&p| p as u32).collect();
        }
        pids.resize(pids.len() * 2, 0);
    }
}

#[cfg(not(target_os = "macos"))]
fn children(pid: u32) -> Vec<u32> {
    // Linux lists each thread's children in /proc/PID/task/TID/children.
    let Ok(tasks) = std::fs::read_dir(format!("/proc/{pid}/task")) else {
        return Vec::new();
    };
    tasks
        .flatten()
        .filter_map(|t| std::fs::read_to_string(t.path().join("children")).ok())
        .flat_map(|s| {
            s.split_whitespace()
                .filter_map(|p| p.parse().ok())
                .collect::<Vec<u32>>()
        })
        .collect()
}

/// The memory a process holds now, in bytes, in RAM or not: its physical
/// footprint, which counts the pages macOS has compressed or swapped out
/// (its resident size does not: under memory pressure a run holding 69 GB
/// showed 2 GB resident). 0 once it has exited.
#[cfg(target_os = "macos")]
pub(crate) fn footprint_bytes(pid: u32) -> u64 {
    // SAFETY: `rusage_info_v2` is plain data; all zeroes is a valid value.
    #[allow(unsafe_code)]
    let mut info: libc::rusage_info_v2 = unsafe { std::mem::zeroed() };
    let buffer: *mut libc::rusage_info_v2 = &mut info;
    // SAFETY: with flavor RUSAGE_INFO_V2 the call writes one `rusage_info_v2`.
    #[allow(unsafe_code)]
    let rc = unsafe { libc::proc_pid_rusage(pid as libc::c_int, libc::RUSAGE_INFO_V2, buffer.cast()) };
    if rc == 0 { info.ri_phys_footprint } else { 0 }
}

/// The memory a process holds now, in bytes, in RAM or not: its resident
/// size plus what is swapped out (`VmRSS` + `VmSwap`). 0 once it has exited.
#[cfg(not(target_os = "macos"))]
pub(crate) fn footprint_bytes(pid: u32) -> u64 {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap_or_default();
    let kb = |key: &str| {
        status
            .lines()
            .find_map(|l| l.strip_prefix(key))
            .and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<u64>().ok())
            .unwrap_or(0)
    };
    (kb("VmRSS:") + kb("VmSwap:")) * 1024
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
    let mut versions = json!({
        "bench": env!("CARGO_PKG_VERSION"),
        "turbowl": env!("CARGO_PKG_VERSION"),
        "rustc": env!("BENCH_RUSTC_VERSION"),
    });
    // Each competitor's fetched version; null when it is not fetched.
    for e in crate::engine::EngineId::ALL {
        if let Some(tool) = e.tool() {
            versions[e.name()] = json!(tool.installed_version());
        }
    }
    json!({
        "cpu": cpu_model(),
        "cores": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
        "os": os_name(),
        "memory_bytes": memory_bytes(),
        "commit": commit,
        "dirty": dirty,
        "build": if cfg!(debug_assertions) { "debug" } else { "release" },
        "versions": versions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A child that fills a 64 MB buffer has a peak of at least that, read
    /// after it exits; one that outlasts its timeout is stopped.
    #[test]
    fn reads_a_childs_peak_and_stops_a_slow_one() {
        let mut dd = Command::new("dd")
            .args(["if=/dev/zero", "of=/dev/null", "bs=67108864", "count=4"])
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let (status, peak) = wait_with_peak(&mut dd, Duration::from_secs(60)).unwrap().unwrap();
        assert!(status.success());
        assert!(peak >= 64 << 20, "{peak}");
        assert!(own_peak_bytes() > 0);

        let mut slow = Command::new("sleep").arg("10").spawn().unwrap();
        let start = std::time::Instant::now();
        assert!(wait_with_peak(&mut slow, Duration::from_millis(100)).unwrap().is_none());
        assert!(start.elapsed() < Duration::from_secs(5));
        assert!(slow.try_wait().unwrap().is_some(), "the slow child was reaped");
    }
}
