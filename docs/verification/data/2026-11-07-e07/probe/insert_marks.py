"""Adds the probe's marks to a copy of the TurbOWL source tree.

Usage: python3 -I insert_marks.py <tree>

Writes crates/turbowl/src/probe_marks.rs (a hook the probe sets) and calls
to it in crates/turbowl/src/engine/store.rs, keyed on lines that exist both
before and after loop iteration 7. Fails if an anchor is missing, so a
later change to store.rs shows up here rather than as missing marks.
Never run it on a checkout you keep: it edits the tree in place.
"""
import pathlib
import re
import sys

tree = pathlib.Path(sys.argv[1])
lib = tree / "crates/turbowl/src/lib.rs"
store = tree / "crates/turbowl/src/engine/store.rs"

(tree / "crates/turbowl/src/probe_marks.rs").write_text('''//! Probe marks (not landed): a hook the footprint probe sets.

/// The probe's hook: a label, a table index and a count.
pub type Hook = fn(&'static str, usize, usize);

static HOOK: std::sync::OnceLock<Hook> = std::sync::OnceLock::new();

/// Sets the hook, once.
pub fn set_hook(hook: Hook) {
    let _ = HOOK.set(hook);
}

/// Calls the hook, when one is set.
#[inline]
pub(crate) fn mark(label: &'static str, t: usize, n: usize) {
    if let Some(hook) = HOOK.get() {
        hook(label, t, n);
    }
}
''')

text = lib.read_text()
anchor = "pub mod profile;\n"
assert anchor in text, "lib.rs: no `pub mod profile;`"
lib.write_text(text.replace(anchor, anchor + "#[doc(hidden)]\npub mod probe_marks;\n", 1))

text = store.read_text()
M = "crate::probe_marks::mark"


def once(old, new):
    global text
    count = text.count(old)
    assert count == 1, f"store.rs: {count} matches for {old!r}"
    text = text.replace(old, new, 1)


# Steps and rounds.
once("        let changed = self.advance(pending, &mut log);\n",
     f"        {M}(\"step_begins\", 0, self.rounds);\n"
     "        let changed = self.advance(pending, &mut log);\n"
     f"        {M}(\"step_ends\", 0, self.rounds);\n")
once("            rules(&mut Round {\n",
     f"            {M}(\"round_begins\", 0, self.rounds);\n            rules(&mut Round {{\n")
# The first build of an object index.
once("            if table.os.is_none() {\n",
     "            if table.os.is_none() {\n"
     f"                {M}(\"index_build_begins\", t, table.so.len());\n")
once("                table.os = Some(os);\n",
     "                table.os = Some(os);\n"
     f"                {M}(\"index_build_ends\", t, table.so.len());\n")
# The four phases of each table's job, with a count: the table's new pairs
# by subject, or for the index phases 0 when the table has no object index
# (the phase then does nothing).
def count(job, label):
    if label.startswith("index"):
        return f"{job}.table.os.as_ref().map_or(0, |_| {job}.table.so.recent().len())"
    return f"{job}.table.so.recent().len()"


for phase, label in [("sort", "sort"), ("change", "change"), ("sort_index", "index_sort"), ("change_index", "index_change")]:
    # In a parallel step (Job::advance) ...
    n = count("self", label)
    once(f"        self.{phase}(workers);\n",
         f"        {M}(\"{label}_begins\", self.t, {n});\n"
         f"        self.{phase}(workers);\n"
         f"        {M}(\"{label}_ends\", self.t, {n});\n")
    # ... and on one thread, phase by phase.
    n = count("job", label)
    once(f"            jobs.iter_mut().for_each(|job| job.{phase}(None));\n",
         f"            jobs.iter_mut().for_each(|job| {{\n"
         f"                {M}(\"{label}_begins\", job.t, {n});\n"
         f"                job.{phase}(None);\n"
         f"                {M}(\"{label}_ends\", job.t, {n});\n"
         f"            }});\n")
store.write_text(text)
print(f"marks added to {tree}")
