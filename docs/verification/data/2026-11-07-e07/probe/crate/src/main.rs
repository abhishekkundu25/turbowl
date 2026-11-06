//! The footprint probe of loop iteration 7 (macOS only). Loads the files
//! given, reasons once, and reads at every mark the library calls
//! (`probe_marks`, added by `insert_marks.py`):
//!
//! - the footprint (`ri_phys_footprint`) and its lifetime peak
//!   (`ri_lifetime_max_phys_footprint`, what the bench's `peak_rss_bytes`
//!   reads), from `proc_pid_rusage`;
//! - the bytes the allocator holds in use (`size_in_use`) and its pages
//!   (`size_allocated`), from `malloc_zone_statistics` over every zone.
//!
//! Usage: probe <all|one> <file>...
//!
//! MB is 10^6 bytes. Times are ms from the start of `reason`.

use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use turbowl::{Reasoner, Threads};

#[repr(C)]
#[derive(Default)]
struct MallocStatistics {
    blocks_in_use: u32,
    size_in_use: usize,
    max_size_in_use: usize,
    size_allocated: usize,
}

unsafe extern "C" {
    fn malloc_zone_statistics(zone: *mut std::ffi::c_void, stats: *mut MallocStatistics);
}

#[derive(Clone, Copy)]
struct Reading {
    foot: u64,
    life: u64,
    used: usize,
    allocated: usize,
}

fn read() -> Reading {
    let mut info: libc::rusage_info_v4 = unsafe { std::mem::zeroed() };
    let buffer: *mut libc::rusage_info_v4 = &mut info;
    let rc = unsafe { libc::proc_pid_rusage(std::process::id() as libc::c_int, libc::RUSAGE_INFO_V4, buffer.cast()) };
    assert_eq!(rc, 0, "proc_pid_rusage");
    let mut stats = MallocStatistics::default();
    unsafe { malloc_zone_statistics(std::ptr::null_mut(), &mut stats) };
    Reading {
        foot: info.ri_phys_footprint,
        life: info.ri_lifetime_max_phys_footprint,
        used: stats.size_in_use,
        allocated: stats.size_allocated,
    }
}

#[derive(Clone, Copy)]
struct Mark {
    label: &'static str,
    t: usize,
    n: usize,
    ms: f64,
    at: Reading,
}

static START: OnceLock<Instant> = OnceLock::new();
static MARKS: Mutex<Vec<Mark>> = Mutex::new(Vec::new());

fn hook(label: &'static str, t: usize, n: usize) {
    let ms = START.get().map_or(0.0, |s| s.elapsed().as_secs_f64() * 1e3);
    let at = read();
    MARKS.lock().unwrap().push(Mark { label, t, n, ms, at });
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / 1e6
}

fn delta(a: u64, b: u64) -> f64 {
    (b as f64 - a as f64) / 1e6
}

fn main() {
    let mut args = std::env::args().skip(1);
    let threads = match args.next().as_deref() {
        Some("all") => Threads::ALL,
        Some("one") => Threads::ONE,
        _ => panic!("usage: probe <all|one> <file>..."),
    };
    let files: Vec<String> = args.collect();
    let mut r = Reasoner::new();
    r.set_threads(threads);
    for f in &files {
        r.load_file(std::path::Path::new(f)).expect("a file that loads");
    }
    let loaded = read();
    turbowl::probe_marks::set_hook(hook);
    START.set(Instant::now()).unwrap();
    hook("reason_begins", 0, 0);
    let summary = r.reason();
    hook("reason_ends", 0, 0);
    let took = START.get().unwrap().elapsed().as_secs_f64() * 1e3;
    let end = read();
    let closure = summary.base + summary.inferred;
    let mut marks = MARKS.lock().unwrap().clone();
    marks.sort_by(|a, b| a.ms.total_cmp(&b.ms));

    println!(
        "threads {}, closure {closure}, base {}, marks {}",
        if threads == Threads::ALL { "all" } else { "one" },
        summary.base,
        marks.len()
    );
    println!(
        "after load: footprint {:.1} MB, lifetime peak {:.1} MB, in use {:.1} MB",
        mb(loaded.foot),
        mb(loaded.life),
        mb(loaded.used as u64)
    );
    println!(
        "after reason: footprint {:.1} MB, lifetime peak {:.1} MB ({:.1} B/triple), in use {:.1} MB, reason took {took:.1} ms",
        mb(end.foot),
        mb(end.life),
        end.life as f64 / closure as f64,
        mb(end.used as u64)
    );
    let max_used = marks.iter().map(|m| m.at.used).max().unwrap_or(0);
    println!("most in use at a mark: {:.1} MB", mb(max_used as u64));

    // The first index builds run on the caller's thread before the step's
    // jobs start: nothing else allocates between their two marks.
    println!("\nINDEX BUILDS (first build of an object index)");
    println!(
        "{:>4} {:>9} {:>8} {:>9} {:>9} {:>9} {:>9} {:>9} {:>8}",
        "t", "pairs", "MB", "foot at", "+foot", "+life", "+used", "+alloc", "ms"
    );
    let mut open: Vec<Mark> = Vec::new();
    for m in &marks {
        match m.label {
            "index_build_begins" => open.push(*m),
            "index_build_ends" => {
                let i = open.iter().position(|b| b.t == m.t).expect("a build that began");
                let b = open.swap_remove(i);
                println!(
                    "{:>4} {:>9} {:>8.1} {:>9.1} {:>9.1} {:>9.1} {:>9.1} {:>9.1} {:>8.2}",
                    m.t,
                    m.n,
                    mb((m.n * 8) as u64),
                    mb(b.at.foot),
                    delta(b.at.foot, m.at.foot),
                    delta(b.at.life, m.at.life),
                    delta(b.at.used as u64, m.at.used as u64),
                    delta(b.at.allocated as u64, m.at.allocated as u64),
                    m.ms - b.ms
                );
            }
            _ => {}
        }
    }

    // A job's phases run beside the other tables' jobs in a parallel step:
    // the window between its two marks counts what they allocate too.
    println!("\nINDEX SORTS (a step's new pairs flipped into an index; other jobs run alongside)");
    println!(
        "{:>4} {:>9} {:>9} {:>9} {:>9} {:>9} {:>8}",
        "t", "pairs", "foot at", "+foot", "+life", "+used", "ms"
    );
    let mut open: Vec<Mark> = Vec::new();
    for m in &marks {
        match m.label {
            "index_sort_begins" => open.push(*m),
            "index_sort_ends" => {
                let i = open.iter().position(|b| b.t == m.t).expect("a sort that began");
                let b = open.swap_remove(i);
                if b.n >= 100_000 {
                    println!(
                        "{:>4} {:>9} {:>9.1} {:>9.1} {:>9.1} {:>9.1} {:>8.2}",
                        m.t,
                        b.n,
                        mb(b.at.foot),
                        delta(b.at.foot, m.at.foot),
                        delta(b.at.life, m.at.life),
                        delta(b.at.used as u64, m.at.used as u64),
                        m.ms - b.ms
                    );
                }
            }
            _ => {}
        }
    }

    // Phases: each step (step_begins to step_ends) and each round (a
    // round's first mark to the next step's).
    println!("\nPHASES");
    println!(
        "{:<10} {:>8} {:>8} {:>9} {:>9} {:>9} {:>9} {:>9}",
        "phase", "from ms", "to ms", "foot at", "+foot", "+life", "life to", "+used"
    );
    let mut last: Option<Mark> = None;
    let mut step = 0;
    for m in &marks {
        let name = match m.label {
            "step_ends" => {
                step += 1;
                Some(format!("step {}", step - 1))
            }
            "step_begins" if step > 0 => Some(format!("round {step}")),
            _ => None,
        };
        if let (Some(name), Some(b)) = (name, last) {
            println!(
                "{:<10} {:>8.1} {:>8.1} {:>9.1} {:>9.1} {:>9.1} {:>9.1} {:>9.1}",
                name,
                b.ms,
                m.ms,
                mb(b.at.foot),
                delta(b.at.foot, m.at.foot),
                delta(b.at.life, m.at.life),
                mb(m.at.life),
                delta(b.at.used as u64, m.at.used as u64)
            );
        }
        if matches!(m.label, "step_begins" | "step_ends") {
            last = Some(*m);
        }
    }

    println!("\nMARKS label t n ms footprint life used allocated");
    for m in &marks {
        println!(
            "M {} t{} n{} {:.2} {:.1} {:.1} {:.1} {:.1}",
            m.label,
            m.t,
            m.n,
            m.ms,
            mb(m.at.foot),
            mb(m.at.life),
            mb(m.at.used as u64),
            mb(m.at.allocated as u64)
        );
    }
}
