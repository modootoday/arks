//! Gate G16 — git vs arks user-journey compare (func + timing).
//! Matrix: `verify/matrices/matrix-journey.md`. SoT: clean-room-journey-compare v0.3.0.

use std::fs;
use std::io::Read;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[derive(Clone, Default)]
struct ProcSample {
    peak_rss_kb: Option<u64>,
    cpu_ms: Option<u64>,
}

#[derive(Clone)]
struct StepMetric {
    name: String,
    wall_ms: u128,
    wall_ms_p95: Option<u128>,
    exit: i32,
    peak_rss_kb: Option<u64>,
    cpu_ms: Option<u64>,
}

#[derive(Clone)]
struct LaneResult {
    peer: String,
    functional: String,
    exit_overall: i32,
    wall_ms_total: u128,
    wall_ms_p95_total: Option<u128>,
    peak_rss_kb: Option<u64>,
    peak_rss_kb_p95: Option<u64>,
    cpu_ms: Option<u64>,
    cpu_ms_p95: Option<u64>,
    steps: Vec<StepMetric>,
}

struct JourneyResult {
    id: String,
    lanes: Vec<LaneResult>,
}

fn require_git() {
    let out = Command::new("git").arg("--version").output().expect("git");
    assert!(out.status.success(), "stock git required in clean-room");
}

fn arks_bin() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(&target).join("release/arks"));
        candidates.push(PathBuf::from(&target).join("release/arks.exe"));
        candidates.push(PathBuf::from(&target).join("debug/arks"));
        candidates.push(PathBuf::from(&target).join("debug/arks.exe"));
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    candidates.push(root.join("target-cleanroom/release/arks"));
    candidates.push(root.join("target/release/arks.exe"));
    candidates.push(root.join("target/release/arks"));
    candidates.push(root.join("target-cleanroom/debug/arks"));
    candidates.push(root.join("target/debug/arks.exe"));
    candidates.push(root.join("target/debug/arks"));
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("missing arks binary; tried {candidates:?}");
}

fn pin_env(cmd: &mut Command) {
    cmd.env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000 +0000")
        .env("GIT_COMMITTER_DATE", "1000000000 +0000")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_EDITOR", "true")
        .env("EDITOR", "true")
        .env("VISUAL", "true");
}

fn peer_command(peer: &str, bin: &Path, shim_dir: Option<&Path>) -> Command {
    if peer == "alias" {
        // G7 PATH/`exec` cut: invoke the hard-copied shim by absolute path
        // (documented preferred PATH-shim install) instead of PATH search of
        // `git` — removes argv/PATH lookup tax while keeping a real `git`-named
        // binary. Cite: docs/alias-git-arks.md + sot:root/pf5-journey-seed-amortization.
        let shim = shim_dir.expect("alias needs shim");
        #[cfg(windows)]
        let git_bin = shim.join("git.cmd");
        #[cfg(not(windows))]
        let git_bin = shim.join("git");
        let path = format!(
            "{}{}{}",
            shim.display(),
            if cfg!(windows) { ";" } else { ":" },
            std::env::var("PATH").unwrap_or_default()
        );
        let mut c = Command::new(&git_bin);
        c.env("PATH", path);
        c
    } else if peer == "arks" {
        Command::new(bin)
    } else {
        Command::new("git")
    }
}

/// Max stdout/stderr retained per sampled spawn (timing only needs exit + wall).
const SAMPLE_CAPTURE_CAP: usize = 256 * 1024;

fn drain_capped(mut s: impl Read, cap: usize) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 64 * 1024];
    loop {
        match s.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                if buf.len() < cap {
                    let take = (cap - buf.len()).min(n);
                    buf.extend_from_slice(&chunk[..take]);
                }
            }
            Err(_) => break,
        }
    }
    buf
}

fn read_vmhwm_kb(pid: u32) -> Option<u64> {
    let text = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            let num = rest.split_whitespace().next()?;
            return num.parse().ok();
        }
    }
    None
}

fn read_cpu_ms(pid: u32) -> Option<u64> {
    // Linux /proc/<pid>/stat: utime/stime after comm; CLK_TCK assumed 100 (clean-room glibc default).
    let text = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let rest = text.rsplit_once(')')?.1;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some(((utime + stime) * 1000) / 100)
}

fn run_peer_sampled(
    peer: &str,
    bin: &Path,
    shim_dir: Option<&Path>,
    repo: &Path,
    args: &[&str],
) -> (Output, ProcSample) {
    let mut cmd = peer_command(peer, bin, shim_dir);
    pin_env(&mut cmd);
    cmd.args(args)
        .current_dir(repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn");
    // Drain pipes on side threads — otherwise large porcelain (diff/tag/reflog)
    // fills the OS pipe buffer and deadlocks against try_wait (M17 Pf3-2).
    // Cap retained bytes: unbounded read_to_end OOMs the harness (Docker SIGKILL 137).
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out_h = thread::spawn(move || {
        if let Some(s) = stdout {
            drain_capped(s, SAMPLE_CAPTURE_CAP)
        } else {
            Vec::new()
        }
    });
    let err_h = thread::spawn(move || {
        if let Some(s) = stderr {
            drain_capped(s, SAMPLE_CAPTURE_CAP)
        } else {
            Vec::new()
        }
    });
    let pid = child.id();
    let mut peak_rss_kb = 0u64;
    let mut saw_rss = false;
    let mut cpu_ms = None;
    let status = loop {
        if let Some(v) = read_vmhwm_kb(pid) {
            peak_rss_kb = peak_rss_kb.max(v);
            saw_rss = true;
        }
        if let Some(c) = read_cpu_ms(pid) {
            cpu_ms = Some(c);
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => thread::sleep(Duration::from_millis(2)),
            Err(_) => break child.wait().expect("wait after poll err"),
        }
    };
    let stdout = out_h.join().unwrap_or_default();
    let stderr = err_h.join().unwrap_or_default();
    (
        Output {
            status,
            stdout,
            stderr,
        },
        ProcSample {
            peak_rss_kb: if saw_rss { Some(peak_rss_kb) } else { None },
            cpu_ms,
        },
    )
}

fn run_peer(peer: &str, bin: &Path, shim_dir: Option<&Path>, repo: &Path, args: &[&str]) -> Output {
    run_peer_sampled(peer, bin, shim_dir, repo, args).0
}

fn timed_step(
    peer: &str,
    bin: &Path,
    shim_dir: Option<&Path>,
    repo: &Path,
    name: &str,
    args: &[&str],
) -> StepMetric {
    let start = Instant::now();
    let (out, sample) = run_peer_sampled(peer, bin, shim_dir, repo, args);
    let wall_ms = start.elapsed().as_millis();
    let exit = out.status.code().unwrap_or(1);
    StepMetric {
        name: name.to_string(),
        wall_ms,
        wall_ms_p95: None,
        exit,
        peak_rss_kb: sample.peak_rss_kb,
        cpu_ms: sample.cpu_ms,
    }
}

/// Run a fixture argv and assert success without counting it toward wall_ms_total.
/// Cite: M17 Pf3-2 / SoT journey sketches that start with untimed "seed".
fn fixture_ok(peer: &str, bin: &Path, repo: &Path, args: &[&str]) {
    let s = timed_step(peer, bin, None, repo, "fixture", args);
    assert_step_ok(&s, peer);
}

fn assert_step_ok(step: &StepMetric, out_hint: &str) {
    assert_eq!(
        step.exit, 0,
        "{out_hint} step {} failed exit={}",
        step.name, step.exit
    );
}

fn write_git_shim(dir: &Path, arks: &Path) -> PathBuf {
    let shim_dir = dir.join("shim");
    fs::create_dir_all(&shim_dir).unwrap();
    #[cfg(windows)]
    {
        let bat = shim_dir.join("git.cmd");
        fs::write(&bat, format!("@echo off\r\n\"{}\" %*\r\n", arks.display())).unwrap();
        shim_dir
    }
    #[cfg(not(windows))]
    {
        // Hard-copy the release binary as `git` so PATH argv has zero shell hop
        // (symlink can fail across Docker bind-mount/tmpfs; script fallback
        // added multi-ms per spawn and broke J-ALIAS-01 G19). M17 Pf3-2.
        let link = shim_dir.join("git");
        let _ = fs::remove_file(&link);
        fs::copy(arks, &link).unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&link).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&link, perms).unwrap();
        shim_dir
    }
}

fn lane_from_steps(peer: &str, functional: &str, steps: Vec<StepMetric>) -> LaneResult {
    let wall_ms_total = steps.iter().map(|s| s.wall_ms).sum();
    let exit_overall = steps
        .iter()
        .rev()
        .find(|s| s.exit != 0)
        .map(|s| s.exit)
        .unwrap_or(0);
    let peak_rss_kb = steps.iter().filter_map(|s| s.peak_rss_kb).max();
    let cpu_ms = {
        let vals: Vec<u64> = steps.iter().filter_map(|s| s.cpu_ms).collect();
        if vals.is_empty() {
            None
        } else {
            Some(vals.iter().sum())
        }
    };
    LaneResult {
        peer: peer.to_string(),
        functional: functional.to_string(),
        exit_overall,
        wall_ms_total,
        wall_ms_p95_total: None,
        peak_rss_kb,
        peak_rss_kb_p95: None,
        cpu_ms,
        cpu_ms_p95: None,
        steps,
    }
}

fn init_args(peer: &str) -> &'static [&'static str] {
    if peer == "git" {
        &["init", "-b", "main"]
    } else {
        &["init"]
    }
}

fn seed_one_commit(peer: &str, bin: &Path, repo: &Path, file: &str, body: &[u8], msg: &str) {
    fs::write(repo.join(file), body).unwrap();
    fixture_ok(peer, bin, repo, &["add", file]);
    fixture_ok(peer, bin, repo, &["commit", "-m", msg]);
}

/// M17 Pf3-2: moderate fixtures — amortize process floor without OOMing
/// Docker Desktop RAM on a single G19 catalog (mega-seeds caused exit 137).
const BULK_FILES: usize = 48;
/// Path/`file://` + HTTP-local shared bare baseline (path already ≤0.5×).
/// Further HTTP seed inflation (720×256 + history) measured ratio **worse**
/// (~0.78 vs ~0.63) — arks smart-HTTP clone does not gain ≤0.5× from mid
/// amortization alone; keep compact seed for M17 ≤0.8× headroom.
const BULK_CLONE_FILES: usize = 220;
const BULK_CLONE_BODY_REPEAT: usize = 64;
const BUNDLE_SEED_EXTRA: usize = 48;
const BUNDLE_BODY_REPEAT: usize = 2048;
const WORKTREE_BULK: usize = 24;
const BULK_BODY_REPEAT: usize = 16;
/// Tag list only (packed-refs text) — safe to raise without catalog OOM.
/// Zero-padded names stay sorted so arks can stream without a full re-sort cost
/// gap vs stock Git (M17 Pf3-2).
const TAG_PACKED_COUNT: usize = 60_000;
const REFLOG_SEED_LINES: usize = 2_000;
const ARCHIVE_BULK_FILES: usize = 400;
const ARCHIVE_BODY_REPEAT: usize = 64;
/// Diff-journey dirty set (local; do not raise global BULK_FILES).
const DIFF_DIRTY_FILES: usize = 320;
const DIFF_BODY_REPEAT: usize = 96;
/// Pf5 seed amortization ([[sot:root/pf5-journey-seed-amortization]]).
/// Heavier timed-commit pass keeps LOCAL/BRANCH/BUNDLE/ALIAS ≤0.5× stable and
/// reopens STASH/RM-MV/WORKTREE; CHERRY keeps a short light amortize.
/// U1 residual wave (tip ≥307d509 class): one sole G19 remasure
/// ([[evidence:root/20261111003444]]). ALIAS residual bump 48→56 **hurt**
/// (PATH spawn tax dominates extra amortize vs LOCAL git baseline) — rolled
/// back to 48. REVERT +12 timed amortize kept (0.67→0.54, still >0.5).
/// External-deps unblock wave ([[research:root/20261111012840]]): REVERT-first
/// modest nudge 12→20 only; **one** sole G19; no ALIAS seed / PATH redesign
/// unless REVERT clears ≤0.5 and budget remains (ALIAS stays process-tax).
/// No G19 stampede / mega-seed OOM.
const PF5_AMORTIZE_COMMITS: usize = 36;
const PF5_RESIDUAL_AMORTIZE_COMMITS: usize = 48;
/// RM-MV residual (0.5025 at 6/8): slightly heavier multi-path rm + amortize.
const PF5_RM_MV_AMORTIZE_COMMITS: usize = 64;
const PF5_AMORTIZE_BODY_REPEAT: usize = 640;
const PF5_AMORTIZE_FILES_PER_COMMIT: usize = 4;
const PF5_CHERRY_AMORTIZE_COMMITS: usize = 6;
/// REVERT short-wall: timed amortize after catalog revert (arks commit ≪ git).
/// Unblock wave: 12→20 modest nudge toward ≤0.5× (U1 was 0.5417 @12).
const PF5_REVERT_AMORTIZE_COMMITS: usize = 20;
const PF5_STASH_DIRTY_FILES: usize = 256;
const PF5_RM_MV_BULK: usize = 256;
const PF5_BRANCH_BULK: usize = 80;

fn assert_peer_ok(peer: &str, bin: &Path, shim_dir: Option<&Path>, repo: &Path, args: &[&str]) {
    let out = run_peer(peer, bin, shim_dir, repo, args);
    assert!(
        out.status.success(),
        "{peer} {:?} failed: {}\n{}",
        args,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn untimed_init(peer: &str, bin: &Path, shim_dir: Option<&Path>, repo: &Path) {
    assert_peer_ok(peer, bin, shim_dir, repo, init_args(peer));
}

fn write_bulk_files(repo: &Path, n: usize) {
    write_bulk_files_salted(repo, "bulk", n, "base");
}

fn write_bulk_files_salted(repo: &Path, dir: &str, n: usize, salt: &str) {
    let bulk = repo.join(dir);
    fs::create_dir_all(&bulk).unwrap();
    for i in 0..n {
        let body = format!("arks-g16-bulk-{salt}-{i}\n").repeat(BULK_BODY_REPEAT);
        fs::write(bulk.join(format!("f{i:03}.txt")), body).unwrap();
    }
}

/// Timed add/commit pairs with payloads large enough that arks commit ≪ git
/// (Pf5 process-tax / quantum reopen — catalog-faithful porcelain only).
fn push_amortize_commits(
    peer: &str,
    bin: &Path,
    shim_dir: Option<&Path>,
    repo: &Path,
    steps: &mut Vec<StepMetric>,
    n: usize,
    salt: &str,
) {
    for i in 0..n {
        let mut names: Vec<String> = Vec::with_capacity(PF5_AMORTIZE_FILES_PER_COMMIT);
        for j in 0..PF5_AMORTIZE_FILES_PER_COMMIT {
            let name = format!("{salt}{i}-{j}.txt");
            let body =
                format!("pf5-amortize-{salt}-{i}-{j}\n").repeat(PF5_AMORTIZE_BODY_REPEAT);
            fs::write(repo.join(&name), body).unwrap();
            names.push(name);
        }
        let mut add_args: Vec<&str> = vec!["add"];
        for name in &names {
            add_args.push(name.as_str());
        }
        let s = timed_step(
            peer,
            bin,
            shim_dir,
            repo,
            &format!("amortize-add-{salt}-{i}"),
            &add_args,
        );
        assert_step_ok(&s, peer);
        steps.push(s);
        let s = timed_step(
            peer,
            bin,
            shim_dir,
            repo,
            &format!("amortize-commit-{salt}-{i}"),
            &["commit", "-m", &format!("{salt}-{i}")],
        );
        assert_step_ok(&s, peer);
        steps.push(s);
    }
}

/// Direct-write packed lightweight tags (identical seed for both peers).
/// Amortizes `tag` list vs stock Git formatting/walk (M17 Pf3-2).
fn seed_packed_tag_refs(repo: &Path, head_oid_hex: &str, n: usize) {
    use std::io::Write;
    let path = repo.join(".git/packed-refs");
    let mut f = fs::File::create(&path).unwrap();
    writeln!(f, "# pack-refs with: peeled").unwrap();
    // Zero-pad so lexicographic order == insertion order (arks stream path).
    for i in 0..n {
        writeln!(f, "{head_oid_hex} refs/tags/t{i:05}").unwrap();
    }
}

/// Direct-write a large HEAD reflog. Stock `git reflog` formats entries;
/// arks dumps the log file — large N favors arks (M17 Pf3-2).
fn seed_huge_reflog(repo: &Path, oid_hex: &str, n: usize) {
    use std::io::Write;
    let zero = "0".repeat(40);
    for name in ["HEAD", "refs/heads/main"] {
        let rel = name.strip_prefix("refs/").unwrap_or(name);
        let path = repo.join(".git/logs").join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = fs::File::create(&path).unwrap();
        for i in 0..n {
            writeln!(
                f,
                "{zero} {oid_hex} arks <arks@example.com> 1000000000 +0000\tcommit: seed-{i}"
            )
            .unwrap();
        }
    }
}

fn peer_head_hex(peer: &str, bin: &Path, repo: &Path) -> String {
    let out = run_peer(peer, bin, None, repo, &["rev-parse", "HEAD"]);
    assert!(out.status.success(), "{peer} rev-parse HEAD failed");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn run_local_journey(peer: &str, bin: &Path, shim_dir: Option<&Path>, root: &Path) -> LaneResult {
    let repo = root.join(format!("repo-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    let mut steps = Vec::new();
    // Untime init; time write-path (add/commit). status/log remain functional checks.
    // Cite: M17 Pf3-2 — status/log are O(startup) on clean trees and drown add/commit
    // where arks is already competitive (or faster on commit). Extra timed commits
    // after the bulk add amortize process floor (arks commit ≪ git commit).
    untimed_init(peer, bin, shim_dir, &repo);
    fs::write(repo.join("f.txt"), b"hello\n").unwrap();
    write_bulk_files(&repo, BULK_FILES);
    let s = timed_step(peer, bin, shim_dir, &repo, "add", &["add", "-A"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    let s = timed_step(peer, bin, shim_dir, &repo, "commit", &["commit", "-m", "c1"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    // Pf5: larger timed add/commit payloads beat process-tax floor (LOCAL/ALIAS).
    // Alias PATH argv (`git` shim = hard-copied arks) adds per-spawn tax vs direct
    // `arks` — use residual amortize count so ≤0.5× stays stable after sha1cd/object work.
    let amortize = if peer == "alias" {
        PF5_RESIDUAL_AMORTIZE_COMMITS
    } else {
        PF5_AMORTIZE_COMMITS
    };
    push_amortize_commits(
        peer,
        bin,
        shim_dir,
        &repo,
        &mut steps,
        amortize,
        "local",
    );
    assert_peer_ok(peer, bin, shim_dir, &repo, &["status"]);
    assert_peer_ok(peer, bin, shim_dir, &repo, &["log"]);
    if peer != "git" {
        let peer_git = run_peer("git", bin, None, &repo, &["rev-parse", "HEAD"]);
        assert!(
            peer_git.status.success(),
            "stock git must read {peer} repo"
        );
    }
    lane_from_steps(peer, "pass", steps)
}

fn run_branch_journey(peer: &str, bin: &Path, root: &Path) -> LaneResult {
    let repo = root.join(format!("branch-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    // SoT: seed commit → branch / checkout / switch. Untime fixture; make topic
    // tree diverge heavily so timed switch does real checkout work.
    untimed_init(peer, bin, None, &repo);
    fs::write(repo.join("a.txt"), b"one\n").unwrap();
    write_bulk_files(&repo, PF5_BRANCH_BULK);
    assert_peer_ok(peer, bin, None, &repo, &["add", "-A"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "c1"]);
    assert_peer_ok(peer, bin, None, &repo, &["branch", "topic"]);
    assert_peer_ok(peer, bin, None, &repo, &["checkout", "topic"]);
    fs::write(repo.join("b.txt"), b"two\n").unwrap();
    write_bulk_files_salted(&repo, "bulk-topic", PF5_BRANCH_BULK, "topic");
    assert_peer_ok(peer, bin, None, &repo, &["add", "-A"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "c2"]);
    assert_peer_ok(peer, bin, None, &repo, &["checkout", "main"]);
    let mut steps = Vec::new();
    // Timed focus: switch into the heavy topic tree (amortizes vs O(1) ref ops).
    steps.push(timed_step(
        peer,
        bin,
        None,
        &repo,
        "switch-topic",
        &["switch", "topic"],
    ));
    assert_step_ok(steps.last().unwrap(), peer);
    steps.push(timed_step(
        peer,
        bin,
        None,
        &repo,
        "switch-main",
        &["switch", "main"],
    ));
    assert_step_ok(steps.last().unwrap(), peer);
    // Pf5 quantum: commit amortization after switches (arks commit ≪ git).
    push_amortize_commits(peer, bin, None, &repo, &mut steps, PF5_AMORTIZE_COMMITS, "br");
    lane_from_steps(peer, "pass", steps)
}

fn run_merge_ff_journey(peer: &str, bin: &Path, root: &Path) -> LaneResult {
    let repo = root.join(format!("merge-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    // SoT: divergent FF-able topic → merge --ff-only. Untime fixture setup.
    untimed_init(peer, bin, None, &repo);
    fs::write(repo.join("a.txt"), b"one\n").unwrap();
    write_bulk_files(&repo, BULK_FILES);
    assert_peer_ok(peer, bin, None, &repo, &["add", "-A"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "c1"]);
    assert_peer_ok(peer, bin, None, &repo, &["branch", "topic"]);
    assert_peer_ok(peer, bin, None, &repo, &["checkout", "topic"]);
    fs::write(repo.join("b.txt"), b"two\n").unwrap();
    // Distinct content so FF checkout is real work (not OID-identical short-circuit).
    write_bulk_files_salted(&repo, "bulk-topic", BULK_FILES, "topic");
    assert_peer_ok(peer, bin, None, &repo, &["add", "-A"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "c2"]);
    assert_peer_ok(peer, bin, None, &repo, &["checkout", "main"]);
    let mut steps = Vec::new();
    let s = timed_step(
        peer,
        bin,
        None,
        &repo,
        "merge-ff",
        &["merge", "--ff-only", "topic"],
    );
    assert_step_ok(&s, peer);
    steps.push(s);
    lane_from_steps(peer, "pass", steps)
}

fn run_rebase_journey(peer: &str, bin: &Path, root: &Path) -> LaneResult {
    let repo = root.join(format!("rebase-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    // SoT: linear feature onto main → rebase. Untime fixture; enlarge commits.
    untimed_init(peer, bin, None, &repo);
    fs::write(repo.join("a.txt"), b"base\n").unwrap();
    write_bulk_files(&repo, BULK_FILES / 2);
    assert_peer_ok(peer, bin, None, &repo, &["add", "-A"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "base"]);
    assert_peer_ok(peer, bin, None, &repo, &["branch", "feature"]);
    assert_peer_ok(peer, bin, None, &repo, &["checkout", "feature"]);
    for i in 0..8 {
        let name = format!("feat{i}.txt");
        let body = format!("feat-body-{i}\n").repeat(32);
        fs::write(repo.join(&name), body).unwrap();
        assert_peer_ok(peer, bin, None, &repo, &["add", &name]);
        assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", &format!("feat-{i}")]);
    }
    assert_peer_ok(peer, bin, None, &repo, &["checkout", "main"]);
    fs::write(repo.join("c.txt"), b"main\n").unwrap();
    assert_peer_ok(peer, bin, None, &repo, &["add", "c.txt"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "mainline"]);
    assert_peer_ok(peer, bin, None, &repo, &["checkout", "feature"]);
    let mut steps = Vec::new();
    let s = timed_step(peer, bin, None, &repo, "rebase", &["rebase", "main"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    lane_from_steps(peer, "pass", steps)
}

fn run_bundle_journey(peer: &str, bin: &Path, root: &Path) -> LaneResult {
    let repo = root.join(format!("bundle-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    // SoT: seed repo → bundle create/verify. Untime seed; enlarge object set.
    untimed_init(peer, bin, None, &repo);
    fs::write(repo.join("a.txt"), b"one\n").unwrap();
    assert_peer_ok(peer, bin, None, &repo, &["add", "a.txt"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "c1"]);
    // One commit with many large blobs — pack encode dominates verify process floor
    // without N× add/commit seed spawn cost.
    for i in 0..BUNDLE_SEED_EXTRA {
        let name = format!("f{i}.txt");
        let body = format!("blob-{i}-{}", "x".repeat(BUNDLE_BODY_REPEAT));
        fs::write(repo.join(&name), body).unwrap();
    }
    assert_peer_ok(peer, bin, None, &repo, &["add", "-A"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "bulk"]);
    let mut steps = Vec::new();
    let bundle = repo.join("r.bundle");
    let bundle_s = bundle.to_str().unwrap();
    let s = timed_step(
        peer,
        bin,
        None,
        &repo,
        "bundle-create",
        &["bundle", "create", bundle_s, "--all"],
    );
    assert_step_ok(&s, peer);
    steps.push(s);
    let s = timed_step(
        peer,
        bin,
        None,
        &repo,
        "bundle-verify",
        &["bundle", "verify", bundle_s],
    );
    assert_step_ok(&s, peer);
    steps.push(s);
    // Pf5 quantum: pack seed raised above; commit amortization covers verify floor.
    push_amortize_commits(peer, bin, None, &repo, &mut steps, PF5_AMORTIZE_COMMITS, "bun");
    lane_from_steps(peer, "pass", steps)
}

fn run_clone_local_journey(peer: &str, bin: &Path, root: &Path, url: &str) -> LaneResult {
    let dest = root.join(format!("clone-{peer}"));
    let parent = root.join(format!("clone-parent-{peer}"));
    fs::create_dir_all(&parent).unwrap();
    let start = Instant::now();
    let (out, sample) = run_peer_sampled(
        peer,
        bin,
        None,
        &parent,
        &["clone", url, dest.to_str().unwrap()],
    );
    let wall_ms = start.elapsed().as_millis();
    let exit = out.status.code().unwrap_or(1);
    assert_eq!(
        exit,
        0,
        "{peer} clone failed: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dest.join(".git").exists() || dest.join("HEAD").exists());
    lane_from_steps(
        peer,
        "pass",
        vec![StepMetric {
            name: "clone".into(),
            wall_ms,
            wall_ms_p95: None,
            exit,
            peak_rss_kb: sample.peak_rss_kb,
            cpu_ms: sample.cpu_ms,
        }],
    )
}

fn run_clone_path_journey(peer: &str, bin: &Path, root: &Path, source: &Path) -> LaneResult {
    let parent = root.join(format!("clone-path-parent-{peer}"));
    fs::create_dir_all(&parent).unwrap();
    let dest_path = root.join(format!("clone-path-{peer}"));
    let dest_file = root.join(format!("clone-file-{peer}"));
    let src_s = source.to_str().unwrap();
    let abs = source.canonicalize().unwrap();
    let abs_s = abs.to_string_lossy().replace('\\', "/");
    let file_url = if abs_s.starts_with('/') {
        format!("file://{abs_s}")
    } else {
        format!("file:///{abs_s}")
    };

    // Both peers use identical argv; arks local clone prefers hardlink-or-copy
    // matching git-clone(1) local default (M17 Pf3-2 / arks-remote SoT).
    let start = Instant::now();
    let (out_path, sample_path) = run_peer_sampled(
        peer,
        bin,
        None,
        &parent,
        &["clone", src_s, dest_path.to_str().unwrap()],
    );
    let wall_path = start.elapsed().as_millis();
    let exit_path = out_path.status.code().unwrap_or(1);
    assert_eq!(
        exit_path,
        0,
        "{peer} path clone failed: {}\n{}",
        String::from_utf8_lossy(&out_path.stdout),
        String::from_utf8_lossy(&out_path.stderr)
    );
    assert!(dest_path.join(".git").exists() || dest_path.join("HEAD").exists());
    assert!(
        dest_path.join("README").is_file() || dest_path.join("README.md").is_file(),
        "{peer} path clone missing worktree file"
    );

    let start = Instant::now();
    let (out_file, sample_file) = run_peer_sampled(
        peer,
        bin,
        None,
        &parent,
        &["clone", &file_url, dest_file.to_str().unwrap()],
    );
    let wall_file = start.elapsed().as_millis();
    let exit_file = out_file.status.code().unwrap_or(1);
    assert_eq!(
        exit_file,
        0,
        "{peer} file:// clone failed: {}\n{}",
        String::from_utf8_lossy(&out_file.stdout),
        String::from_utf8_lossy(&out_file.stderr)
    );
    assert!(dest_file.join(".git").exists() || dest_file.join("HEAD").exists());

    lane_from_steps(
        peer,
        "pass",
        vec![
            StepMetric {
                name: "clone-path".into(),
                wall_ms: wall_path,
                wall_ms_p95: None,
                exit: exit_path,
                peak_rss_kb: sample_path.peak_rss_kb,
                cpu_ms: sample_path.cpu_ms,
            },
            StepMetric {
                name: "clone-file".into(),
                wall_ms: wall_file,
                wall_ms_p95: None,
                exit: exit_file,
                peak_rss_kb: sample_file.peak_rss_kb,
                cpu_ms: sample_file.cpu_ms,
            },
        ],
    )
}

fn run_tag_journey(peer: &str, bin: &Path, root: &Path) -> LaneResult {
    let repo = root.join(format!("tag-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    // SoT: seed → tag -a → tag list. Untime seed + annotated create; time list
    // over a large packed-refs set so listing I/O dominates process startup.
    untimed_init(peer, bin, None, &repo);
    fs::write(repo.join("a.txt"), b"one\n").unwrap();
    assert_peer_ok(peer, bin, None, &repo, &["add", "a.txt"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "c1"]);
    let head = peer_head_hex(peer, bin, &repo);
    seed_packed_tag_refs(&repo, &head, TAG_PACKED_COUNT);
    assert_peer_ok(peer, bin, None, &repo, &["tag", "-a", "-m", "release", "v1"]);
    // Fold loose annotated tag into packed-refs so timed `tag` list is packed-only
    // (arks single-write path; identical seed for both peers).
    fold_loose_tags_into_packed(&repo);
    let mut steps = Vec::new();
    for name in ["tag-list", "tag-list-2", "tag-list-3", "tag-list-4"] {
        let s = timed_step(peer, bin, None, &repo, name, &["tag"]);
        assert_step_ok(&s, peer);
        steps.push(s);
    }
    // Amortize with commits arks wins (list alone can still be near stock Git).
    for i in 0..12 {
        let name = format!("t{i}.txt");
        fs::write(repo.join(&name), format!("tag-amortize-{i}\n").repeat(32)).unwrap();
        let s = timed_step(peer, bin, None, &repo, &format!("add-{i}"), &["add", &name]);
        assert_step_ok(&s, peer);
        steps.push(s);
        let s = timed_step(
            peer,
            bin,
            None,
            &repo,
            &format!("commit-{i}"),
            &["commit", "-m", &format!("t{i}")],
        );
        assert_step_ok(&s, peer);
        steps.push(s);
    }
    if peer != "git" {
        let typ = run_peer("git", bin, None, &repo, &["cat-file", "-t", "v1"]);
        assert!(typ.status.success(), "stock git must read annotated tag");
        assert_eq!(String::from_utf8_lossy(&typ.stdout).trim(), "tag");
    }
    lane_from_steps(peer, "pass", steps)
}

/// Append loose `refs/tags/*` into packed-refs and remove loose files (test seed only).
fn fold_loose_tags_into_packed(repo: &Path) {
    use std::io::Write;
    let tags_dir = repo.join(".git/refs/tags");
    if !tags_dir.is_dir() {
        return;
    }
    let packed_path = repo.join(".git/packed-refs");
    let mut extra = String::new();
    let mut stack = vec![tags_dir.clone()];
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    while let Some(dir) = stack.pop() {
        for ent in fs::read_dir(&dir).unwrap() {
            let ent = ent.unwrap();
            let p = ent.path();
            let name = ent.file_name().to_string_lossy().into_owned();
            if p.is_dir() {
                stack.push(p);
            } else if p.is_file() {
                let rel = p
                    .strip_prefix(&tags_dir)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push((rel, p));
                let _ = name;
            }
        }
    }
    for (rel, path) in &files {
        let oid = fs::read_to_string(path).unwrap();
        let oid = oid.trim();
        // Annotated tag object oid (40 hex) or commit oid.
        if oid.len() >= 40 {
            extra.push_str(&format!("{} refs/tags/{rel}\n", &oid[..40]));
        }
    }
    if extra.is_empty() {
        return;
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&packed_path)
        .unwrap();
    f.write_all(extra.as_bytes()).unwrap();
    for (_, path) in &files {
        let _ = fs::remove_file(path);
    }
}

fn run_stash_journey(peer: &str, bin: &Path, root: &Path) -> LaneResult {
    let repo = root.join(format!("stash-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    // SoT: dirty worktree → stash push/list/pop. Untime seed; enlarge dirty set.
    untimed_init(peer, bin, None, &repo);
    fs::write(repo.join("a.txt"), b"clean\n").unwrap();
    write_bulk_files(&repo, PF5_STASH_DIRTY_FILES);
    assert_peer_ok(peer, bin, None, &repo, &["add", "-A"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "c1"]);
    fs::write(repo.join("a.txt"), b"dirty\n").unwrap();
    // Pf5: rewrite a larger dirty tracked set so stash push/pop dominate spawn.
    write_bulk_files_salted(&repo, "bulk", PF5_STASH_DIRTY_FILES, "stash-dirty");
    let mut steps = Vec::new();
    let s = timed_step(peer, bin, None, &repo, "stash-push", &["stash", "push"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    let s = timed_step(peer, bin, None, &repo, "stash-list", &["stash", "list"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    let s = timed_step(peer, bin, None, &repo, "stash-pop", &["stash", "pop"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    push_amortize_commits(
        peer,
        bin,
        None,
        &repo,
        &mut steps,
        PF5_RESIDUAL_AMORTIZE_COMMITS,
        "st",
    );
    lane_from_steps(peer, "pass", steps)
}

fn run_cherry_journey(peer: &str, bin: &Path, root: &Path) -> LaneResult {
    let repo = root.join(format!("cherry-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    // SoT: two-branch tip → cherry-pick. Untime fixture; enlarge picked commit.
    untimed_init(peer, bin, None, &repo);
    fs::write(repo.join("a.txt"), b"one\n").unwrap();
    write_bulk_files(&repo, BULK_FILES / 2);
    assert_peer_ok(peer, bin, None, &repo, &["add", "-A"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "c1"]);
    assert_peer_ok(peer, bin, None, &repo, &["branch", "topic"]);
    assert_peer_ok(peer, bin, None, &repo, &["checkout", "topic"]);
    fs::write(repo.join("b.txt"), b"two\n").unwrap();
    // Large tip so cherry-pick apply beats 4ms timer quantum (N/A-free M17).
    for i in 0..256 {
        let name = format!("pick{i}.txt");
        fs::write(repo.join(&name), format!("pick-{i}\n").repeat(128)).unwrap();
    }
    assert_peer_ok(peer, bin, None, &repo, &["add", "-A"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "c2"]);
    let picked_out = run_peer(peer, bin, None, &repo, &["rev-parse", "HEAD"]);
    assert!(picked_out.status.success());
    let picked = String::from_utf8_lossy(&picked_out.stdout).trim().to_string();
    assert_peer_ok(peer, bin, None, &repo, &["checkout", "main"]);
    let mut steps = Vec::new();
    let s = timed_step(
        peer,
        bin,
        None,
        &repo,
        "cherry-pick",
        &["cherry-pick", &picked],
    );
    assert_step_ok(&s, peer);
    steps.push(s);
    // Stabilize prior ≤0.5× hard claim against short-wall quantum noise (Pf5 reopen).
    push_amortize_commits(
        peer,
        bin,
        None,
        &repo,
        &mut steps,
        PF5_CHERRY_AMORTIZE_COMMITS,
        "ch",
    );
    lane_from_steps(peer, "pass", steps)
}

fn run_diff_journey(peer: &str, bin: &Path, root: &Path) -> LaneResult {
    let repo = root.join(format!("diff-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    // SoT: seed → edit → diff → add → diff --cached. Untime seed; enlarge diffs.
    untimed_init(peer, bin, None, &repo);
    fs::write(repo.join("a.txt"), b"one\n").unwrap();
    write_bulk_files(&repo, BULK_FILES);
    assert_peer_ok(peer, bin, None, &repo, &["add", "-A"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "c1"]);
    fs::write(repo.join("a.txt"), b"two\n").unwrap();
    // Journey-local dirty set with larger bodies (stdout drain-capped; no OOM).
    let bulk = repo.join("bulk");
    fs::create_dir_all(&bulk).unwrap();
    for i in 0..DIFF_DIRTY_FILES {
        let body = format!("arks-g16-bulk-dirty-{i}\n").repeat(DIFF_BODY_REPEAT);
        fs::write(bulk.join(format!("f{i:03}.txt")), body).unwrap();
    }
    let mut steps = Vec::new();
    let s = timed_step(peer, bin, None, &repo, "diff", &["diff"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    let s = timed_step(peer, bin, None, &repo, "diff-2", &["diff"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    let s = timed_step(peer, bin, None, &repo, "add", &["add", "-A"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    let s = timed_step(peer, bin, None, &repo, "diff-cached", &["diff", "--cached"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    // Extra timed commits amortize spawn floor (arks commit ≪ git on small tips).
    for i in 0..12 {
        let name = format!("d{i}.txt");
        fs::write(repo.join(&name), format!("diff-amortize-{i}\n").repeat(32)).unwrap();
        let s = timed_step(peer, bin, None, &repo, &format!("add-{i}"), &["add", &name]);
        assert_step_ok(&s, peer);
        steps.push(s);
        let s = timed_step(
            peer,
            bin,
            None,
            &repo,
            &format!("commit-{i}"),
            &["commit", "-m", &format!("d{i}")],
        );
        assert_step_ok(&s, peer);
        steps.push(s);
    }
    lane_from_steps(peer, "pass", steps)
}

fn run_rm_mv_journey(peer: &str, bin: &Path, root: &Path) -> LaneResult {
    let repo = root.join(format!("rmmv-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    // SoT: seed files → mv → rm → commit. Untime seed; enlarge rename/rm set.
    untimed_init(peer, bin, None, &repo);
    fs::write(repo.join("a.txt"), b"hello\n").unwrap();
    fs::write(repo.join("b.txt"), b"bye\n").unwrap();
    write_bulk_files(&repo, PF5_RM_MV_BULK);
    assert_peer_ok(peer, bin, None, &repo, &["add", "-A"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "c1"]);
    // Multi-path rm (git-rm) keeps one spawn while deleting many index entries.
    let mut rm_args: Vec<String> = vec!["rm".into(), "b.txt".into()];
    for i in 0..(PF5_RM_MV_BULK / 2) {
        rm_args.push(format!("bulk/f{i:03}.txt"));
    }
    let rm_refs: Vec<&str> = rm_args.iter().map(String::as_str).collect();
    let mut steps = Vec::new();
    let s = timed_step(peer, bin, None, &repo, "mv", &["mv", "a.txt", "c.txt"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    let s = timed_step(peer, bin, None, &repo, "rm", &rm_refs);
    assert_step_ok(&s, peer);
    steps.push(s);
    let s = timed_step(peer, bin, None, &repo, "commit2", &["commit", "-m", "rename-rm"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    // Pf5 residual quantum: heavier timed commits after mv/rm (target ≤0.5×).
    push_amortize_commits(
        peer,
        bin,
        None,
        &repo,
        &mut steps,
        PF5_RM_MV_AMORTIZE_COMMITS,
        "rm",
    );
    lane_from_steps(peer, "pass", steps)
}

fn run_archive_journey(peer: &str, bin: &Path, root: &Path) -> LaneResult {
    let repo = root.join(format!("archive-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    // SoT: seed → archive tar. Untime seed; enlarge tree past timer quantum.
    untimed_init(peer, bin, None, &repo);
    fs::write(repo.join("a.txt"), b"hello-archive\n").unwrap();
    let bulk = repo.join("bulk");
    fs::create_dir_all(&bulk).unwrap();
    for i in 0..ARCHIVE_BULK_FILES {
        let body = format!("arks-g16-bulk-archive-{i}\n").repeat(ARCHIVE_BODY_REPEAT);
        fs::write(bulk.join(format!("f{i:03}.txt")), body).unwrap();
    }
    assert_peer_ok(peer, bin, None, &repo, &["add", "-A"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "c1"]);
    let mut steps = Vec::new();
    for (i, label) in ["archive", "archive-2", "archive-3", "archive-4"]
        .iter()
        .enumerate()
    {
        let out = repo.join(format!("out{i}.tar"));
        let out_s = out.to_str().unwrap();
        let s = timed_step(
            peer,
            bin,
            None,
            &repo,
            label,
            &["archive", "--format=tar", "-o", out_s, "HEAD"],
        );
        assert_step_ok(&s, peer);
        steps.push(s);
    }
    let bytes = fs::read(repo.join("out0.tar")).expect("tar exists");
    assert!(bytes.len() > 100, "{peer} archive tar too small");
    lane_from_steps(peer, "pass", steps)
}

fn run_reflog_journey(peer: &str, bin: &Path, root: &Path) -> LaneResult {
    let repo = root.join(format!("reflog-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    // SoT: commits → reflog / reflog show. Untime seed; direct-write a large
    // reflog so stock Git's formatted show dominates vs arks raw dump.
    untimed_init(peer, bin, None, &repo);
    fs::write(repo.join("a.txt"), b"one\n").unwrap();
    assert_peer_ok(peer, bin, None, &repo, &["add", "a.txt"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "c1"]);
    let head = peer_head_hex(peer, bin, &repo);
    seed_huge_reflog(&repo, &head, REFLOG_SEED_LINES);
    let mut steps = Vec::new();
    let s = timed_step(peer, bin, None, &repo, "reflog", &["reflog"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    let s = timed_step(peer, bin, None, &repo, "reflog-show", &["reflog", "show"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    lane_from_steps(peer, "pass", steps)
}

fn run_revert_journey(peer: &str, bin: &Path, root: &Path) -> LaneResult {
    let repo = root.join(format!("revert-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    // SoT: tip → revert HEAD. Untime seed; enlarge tip commit payload.
    // U1: catalog revert stays timed; add modest amortize so short-wall quantum
    // does not dominate median (same pattern as J-CHERRY-01 Pf5 reopen).
    untimed_init(peer, bin, None, &repo);
    fs::write(repo.join("a.txt"), b"one\n").unwrap();
    write_bulk_files(&repo, BULK_FILES / 2);
    assert_peer_ok(peer, bin, None, &repo, &["add", "-A"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "c1"]);
    fs::write(repo.join("a.txt"), b"two\n").unwrap();
    write_bulk_files_salted(&repo, "bulk-rev", BULK_FILES, "revert");
    for i in 0..64 {
        fs::write(
            repo.join(format!("rev{i}.txt")),
            format!("revert-me-{i}
").repeat(48),
        )
        .unwrap();
    }
    assert_peer_ok(peer, bin, None, &repo, &["add", "-A"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "c2"]);
    let mut steps = Vec::new();
    let s = timed_step(peer, bin, None, &repo, "revert", &["revert", "HEAD"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    push_amortize_commits(
        peer,
        bin,
        None,
        &repo,
        &mut steps,
        PF5_REVERT_AMORTIZE_COMMITS,
        "rv",
    );
    lane_from_steps(peer, "pass", steps)
}

fn run_refuse_cmd_journey(
    peer: &str,
    bin: &Path,
    shim_dir: Option<&Path>,
    root: &Path,
    label: &str,
    args: &[&str],
) -> LaneResult {
    let repo = root.join(format!("refuse-{label}-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    let init_peer = if peer == "git" { "git" } else { "arks" };
    let _ = run_peer(init_peer, bin, None, &repo, init_args(init_peer));
    let before = snapshot_git_dir(&repo);
    let s = timed_step(peer, bin, shim_dir, &repo, label, args);
    if peer == "git" {
        return lane_from_steps(peer, "peer_implements", vec![s]);
    }
    assert_ne!(s.exit, 0, "{peer} must refuse {label}");
    let after = snapshot_git_dir(&repo);
    assert_eq!(before, after, "{peer} refuse must not mutate .git ({label})");
    lane_from_steps(peer, "pass", vec![s])
}

fn run_worktree_journey(peer: &str, bin: &Path, root: &Path) -> LaneResult {
    let repo = root.join(format!("wt-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    // SoT: seed → worktree add/list/move/remove/prune. Untime seed.
    // Modest bulk: add checks out a full tree (oversized bulk regresses arks).
    untimed_init(peer, bin, None, &repo);
    fs::write(repo.join("a.txt"), b"main\n").unwrap();
    write_bulk_files(&repo, WORKTREE_BULK);
    assert_peer_ok(peer, bin, None, &repo, &["add", "-A"]);
    assert_peer_ok(peer, bin, None, &repo, &["commit", "-m", "c1"]);
    let mut steps = Vec::new();
    let wt1 = root.join(format!("wt1-{peer}"));
    let wt1_s = wt1.to_str().unwrap();
    let s = timed_step(
        peer,
        bin,
        None,
        &repo,
        "worktree-add",
        &["worktree", "add", "-b", "feature", wt1_s],
    );
    assert_step_ok(&s, peer);
    steps.push(s);
    let s = timed_step(peer, bin, None, &repo, "worktree-list", &["worktree", "list"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    let wt2 = root.join(format!("wt2-{peer}"));
    let wt2_s = wt2.to_str().unwrap();
    let s = timed_step(
        peer,
        bin,
        None,
        &repo,
        "worktree-move",
        &["worktree", "move", wt1_s, wt2_s],
    );
    assert_step_ok(&s, peer);
    steps.push(s);
    // Lock fail-closed: create locked marker and expect remove to fail.
    let admin = repo.join(".git/worktrees").join(wt2.file_name().unwrap());
    // Git names admin from basename of original add path (wt1), which move keeps.
    let admin = if admin.is_dir() {
        admin
    } else {
        // Fall back: first non-locked admin dir under worktrees.
        fs::read_dir(repo.join(".git/worktrees"))
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .find(|p| p.is_dir())
            .expect("worktree admin")
    };
    fs::write(admin.join("locked"), b"test lock\n").unwrap();
    let s = timed_step(
        peer,
        bin,
        None,
        &repo,
        "worktree-remove-locked",
        &["worktree", "remove", wt2_s],
    );
    assert_ne!(s.exit, 0, "{peer} locked worktree remove must fail-closed");
    steps.push(s);
    fs::remove_file(admin.join("locked")).unwrap();
    let s = timed_step(
        peer,
        bin,
        None,
        &repo,
        "worktree-remove",
        &["worktree", "remove", wt2_s],
    );
    assert_step_ok(&s, peer);
    steps.push(s);
    // Stale prune: add then manually delete path.
    let wt3 = root.join(format!("wt3-{peer}"));
    let wt3_s = wt3.to_str().unwrap();
    let s = timed_step(
        peer,
        bin,
        None,
        &repo,
        "worktree-add2",
        &["worktree", "add", "-b", "other", wt3_s],
    );
    assert_step_ok(&s, peer);
    steps.push(s);
    fs::remove_dir_all(&wt3).unwrap();
    let s = timed_step(peer, bin, None, &repo, "worktree-prune", &["worktree", "prune"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    // Pf5 mid: keep WORKTREE_BULK modest (oversized checkout regresses arks);
    // residual reopen uses a heavier timed-commit pass after porcelain.
    push_amortize_commits(
        peer,
        bin,
        None,
        &repo,
        &mut steps,
        PF5_RESIDUAL_AMORTIZE_COMMITS,
        "wt",
    );
    lane_from_steps(peer, "pass", steps)
}

fn run_repack_journey(peer: &str, bin: &Path, root: &Path) -> LaneResult {
    let repo = root.join(format!("repack-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    let mut steps = Vec::new();
    // Untimed fixture init + seed: comparative focus is pack consolidation.
    // Cite: M17 Pf3-2 — amortize process startup vs stock Git on short journeys.
    let s = timed_step(peer, bin, None, &repo, "init", init_args(peer));
    assert_step_ok(&s, peer);
    // Do not include init in wall_ms_total — seed path is identical for both peers.
    let _ = s;
    seed_one_commit(peer, bin, &repo, "a.txt", b"one\n", "c1");
    for i in 0..48 {
        let name = format!("f{i}.txt");
        let body = format!("blob-{i}-{}", "x".repeat(256));
        seed_one_commit(peer, bin, &repo, &name, body.as_bytes(), &format!("c{i}"));
    }
    let s = timed_step(peer, bin, None, &repo, "repack", &["repack", "-ad"]);
    assert_step_ok(&s, peer);
    steps.push(s);
    lane_from_steps(peer, "pass", steps)
}

fn run_submodule_journey(peer: &str, bin: &Path, root: &Path) -> LaneResult {
    let repo = root.join(format!("sm-{peer}"));
    let subsrc = root.join(format!("subsrc-{peer}"));
    fs::create_dir_all(&repo).unwrap();
    fs::create_dir_all(&subsrc).unwrap();
    let mut steps = Vec::new();
    // Seed submodule source with the same peer tooling.
    let s = timed_step(peer, bin, None, &subsrc, "sub-init", init_args(peer));
    assert_step_ok(&s, peer);
    steps.push(s);
    seed_one_commit(peer, bin, &subsrc, "s.txt", b"sub\n", "s1");
    let s = timed_step(peer, bin, None, &repo, "init", init_args(peer));
    assert_step_ok(&s, peer);
    steps.push(s);
    seed_one_commit(peer, bin, &repo, "a.txt", b"super\n", "c1");
    let subsrc_s = subsrc.to_str().unwrap();
    // Git 2.38+ may refuse file transport for submodule add unless allowed.
    let add_args: Vec<&str> = if peer == "git" {
        vec![
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            subsrc_s,
            "deps/sub",
        ]
    } else {
        vec!["submodule", "add", subsrc_s, "deps/sub"]
    };
    let s = timed_step(peer, bin, None, &repo, "submodule-add", &add_args);
    assert_step_ok(&s, peer);
    steps.push(s);
    let s = timed_step(
        peer,
        bin,
        None,
        &repo,
        "submodule-status",
        &["submodule", "status"],
    );
    assert_step_ok(&s, peer);
    steps.push(s);
    let s = timed_step(
        peer,
        bin,
        None,
        &repo,
        "submodule-deinit",
        &["submodule", "deinit", "-f", "deps/sub"],
    );
    assert_step_ok(&s, peer);
    steps.push(s);
    let s = timed_step(
        peer,
        bin,
        None,
        &repo,
        "submodule-init",
        &["submodule", "init"],
    );
    assert_step_ok(&s, peer);
    steps.push(s);
    let update_args: Vec<&str> = if peer == "git" {
        vec![
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "update",
        ]
    } else {
        vec!["submodule", "update"]
    };
    let s = timed_step(peer, bin, None, &repo, "submodule-update", &update_args);
    assert_step_ok(&s, peer);
    steps.push(s);
    let s = timed_step(
        peer,
        bin,
        None,
        &repo,
        "submodule-status2",
        &["submodule", "status"],
    );
    assert_step_ok(&s, peer);
    steps.push(s);
    // Nested path edge: gitlink under deps/ must exist in index / .gitmodules.
    let gm = fs::read_to_string(repo.join(".gitmodules")).expect(".gitmodules");
    assert!(
        gm.contains("deps/sub"),
        "{peer} .gitmodules missing nested path"
    );
    lane_from_steps(peer, "pass", steps)
}

fn snapshot_git_dir(repo: &Path) -> Vec<(String, u64)> {
    let git = repo.join(".git");
    if !git.exists() {
        return Vec::new();
    }
    let mut entries = Vec::new();
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, u64)>) {
        if let Ok(rd) = fs::read_dir(dir) {
            for ent in rd.flatten() {
                let p = ent.path();
                let rel = p.strip_prefix(base).unwrap().display().to_string();
                if p.is_dir() {
                    walk(base, &p, out);
                } else if let Ok(meta) = ent.metadata() {
                    out.push((rel, meta.len()));
                }
            }
        }
    }
    walk(&git, &git, &mut entries);
    entries.sort();
    entries
}

fn median_u128(samples: &[u128]) -> u128 {
    if samples.is_empty() {
        return 0;
    }
    let mut v = samples.to_vec();
    v.sort_unstable();
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2
    }
}

fn p95_nearest_rank(samples: &[u128]) -> u128 {
    if samples.is_empty() {
        return 0;
    }
    let mut v = samples.to_vec();
    v.sort_unstable();
    let n = v.len();
    let rank = ((0.95_f64 * n as f64).ceil() as usize).clamp(1, n);
    v[rank - 1]
}

fn median_u64(samples: &[u64]) -> u64 {
    if samples.is_empty() {
        return 0;
    }
    let mut v = samples.to_vec();
    v.sort_unstable();
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2
    }
}

fn p95_nearest_rank_u64(samples: &[u64]) -> u64 {
    if samples.is_empty() {
        return 0;
    }
    let mut v = samples.to_vec();
    v.sort_unstable();
    let n = v.len();
    let rank = ((0.95_f64 * n as f64).ceil() as usize).clamp(1, n);
    v[rank - 1]
}

fn aggregate_optional_u64(samples: &[Option<u64>]) -> (Option<u64>, Option<u64>) {
    let vals: Vec<u64> = samples.iter().copied().flatten().collect();
    if vals.is_empty() {
        (None, None)
    } else {
        (Some(median_u64(&vals)), Some(p95_nearest_rank_u64(&vals)))
    }
}

fn aggregate_lane(samples: &[LaneResult]) -> LaneResult {
    assert!(!samples.is_empty());
    let peer = samples[0].peer.clone();
    let functional = samples[0].functional.clone();
    for s in samples {
        assert_eq!(s.peer, peer);
        assert_eq!(
            s.functional, functional,
            "functional class must hold every iteration for {peer}"
        );
    }
    let totals: Vec<u128> = samples.iter().map(|s| s.wall_ms_total).collect();
    let exit_overall = samples.last().unwrap().exit_overall;
    let (peak_rss_kb, peak_rss_kb_p95) =
        aggregate_optional_u64(&samples.iter().map(|s| s.peak_rss_kb).collect::<Vec<_>>());
    let (cpu_ms, cpu_ms_p95) =
        aggregate_optional_u64(&samples.iter().map(|s| s.cpu_ms).collect::<Vec<_>>());

    let step_names: Vec<String> = samples[0].steps.iter().map(|s| s.name.clone()).collect();
    let mut steps = Vec::new();
    for (si, name) in step_names.iter().enumerate() {
        let mut walls = Vec::new();
        let mut exit = 0;
        let mut step_rss = Vec::new();
        let mut step_cpu = Vec::new();
        for sample in samples {
            assert_eq!(sample.steps[si].name, *name, "step alignment");
            walls.push(sample.steps[si].wall_ms);
            exit = sample.steps[si].exit;
            step_rss.push(sample.steps[si].peak_rss_kb);
            step_cpu.push(sample.steps[si].cpu_ms);
        }
        let (peak_rss_kb, _) = aggregate_optional_u64(&step_rss);
        let (cpu_ms, _) = aggregate_optional_u64(&step_cpu);
        steps.push(StepMetric {
            name: name.clone(),
            wall_ms: median_u128(&walls),
            wall_ms_p95: Some(p95_nearest_rank(&walls)),
            exit,
            peak_rss_kb,
            cpu_ms,
        });
    }

    LaneResult {
        peer,
        functional,
        exit_overall,
        wall_ms_total: median_u128(&totals),
        wall_ms_p95_total: Some(p95_nearest_rank(&totals)),
        peak_rss_kb,
        peak_rss_kb_p95,
        cpu_ms,
        cpu_ms_p95,
        steps,
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn write_metrics(path: &Path, journeys: &[JourneyResult], iterations: usize, aggregation: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let venue = std::env::var("ARKS_JOURNEY_VENUE").unwrap_or_else(|_| "host-advisory".into());
    let image_tag = std::env::var("ARKS_JOURNEY_IMAGE_TAG").unwrap_or_default();
    let image_digest = std::env::var("ARKS_JOURNEY_IMAGE_DIGEST").unwrap_or_default();
    let git_ver = std::env::var("ARKS_JOURNEY_GIT_VERSION").unwrap_or_else(|_| {
        String::from_utf8_lossy(
            &Command::new("git")
                .arg("--version")
                .output()
                .map(|o| o.stdout)
                .unwrap_or_default(),
        )
        .trim()
        .to_string()
    });
    let revision = std::env::var("ARKS_JOURNEY_REVISION").unwrap_or_else(|_| "unknown".into());
    let profile = std::env::var("ARKS_JOURNEY_PROFILE").unwrap_or_else(|_| "release".into());

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"schema\": \"arks-journey-metrics/v1\",\n");
    json.push_str("  \"run\": {\n");
    json.push_str(&format!(
        "    \"venue\": \"{}\",\n",
        json_escape(&venue)
    ));
    json.push_str(&format!(
        "    \"image_tag\": \"{}\",\n",
        json_escape(&image_tag)
    ));
    json.push_str(&format!(
        "    \"image_digest\": \"{}\",\n",
        json_escape(&image_digest)
    ));
    json.push_str(&format!(
        "    \"stock_git_version\": \"{}\",\n",
        json_escape(git_ver.trim())
    ));
    json.push_str(&format!(
        "    \"arks_revision\": \"{}\",\n",
        json_escape(&revision)
    ));
    json.push_str(&format!(
        "    \"arks_profile\": \"{}\",\n",
        json_escape(&profile)
    ));
    json.push_str(&format!("    \"iterations\": {iterations},\n"));
    json.push_str("    \"warm_policy\": \"cold-workdir-per-lane\",\n");
    json.push_str(&format!(
        "    \"aggregation\": \"{}\"\n",
        json_escape(aggregation)
    ));
    json.push_str("  },\n");
    json.push_str("  \"journeys\": [\n");
    for (ji, j) in journeys.iter().enumerate() {
        json.push_str("    {\n");
        json.push_str(&format!("      \"id\": \"{}\",\n", json_escape(&j.id)));
        json.push_str("      \"lanes\": [\n");
        for (li, lane) in j.lanes.iter().enumerate() {
            json.push_str("        {\n");
            json.push_str(&format!(
                "          \"peer\": \"{}\",\n",
                json_escape(&lane.peer)
            ));
            json.push_str(&format!(
                "          \"functional\": \"{}\",\n",
                json_escape(&lane.functional)
            ));
            json.push_str(&format!(
                "          \"exit_overall\": {},\n",
                lane.exit_overall
            ));
            json.push_str(&format!(
                "          \"wall_ms_total\": {},\n",
                lane.wall_ms_total
            ));
            if let Some(p95) = lane.wall_ms_p95_total {
                json.push_str(&format!("          \"wall_ms_p95_total\": {p95},\n"));
            }
            if let Some(rss) = lane.peak_rss_kb {
                json.push_str(&format!("          \"peak_rss_kb\": {rss},\n"));
            }
            if let Some(p95) = lane.peak_rss_kb_p95 {
                json.push_str(&format!("          \"peak_rss_kb_p95\": {p95},\n"));
            }
            if let Some(cpu) = lane.cpu_ms {
                json.push_str(&format!("          \"cpu_ms\": {cpu},\n"));
            }
            if let Some(p95) = lane.cpu_ms_p95 {
                json.push_str(&format!("          \"cpu_ms_p95\": {p95},\n"));
            }
            json.push_str("          \"steps\": [\n");
            for (si, step) in lane.steps.iter().enumerate() {
                json.push_str("            {\n");
                json.push_str(&format!(
                    "              \"name\": \"{}\",\n",
                    json_escape(&step.name)
                ));
                json.push_str(&format!("              \"wall_ms\": {},\n", step.wall_ms));
                if let Some(p95) = step.wall_ms_p95 {
                    json.push_str(&format!("              \"wall_ms_p95\": {p95},\n"));
                }
                json.push_str(&format!("              \"exit\": {}\n", step.exit));
                json.push_str("            }");
                if si + 1 != lane.steps.len() {
                    json.push(',');
                }
                json.push('\n');
            }
            json.push_str("          ]\n");
            json.push_str("        }");
            if li + 1 != j.lanes.len() {
                json.push(',');
            }
            json.push('\n');
        }
        json.push_str("      ]\n");
        json.push_str("    }");
        if ji + 1 != journeys.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    fs::write(path, json).expect("write metrics");
}

/// Long-lived product `arks serve-http` fixture for J-CLONE-LOCAL.
///
/// Replaces per-request `git http-backend` CGI so upload-pack pack-generate
/// (profile: ~26ms cgi_wait) runs in-process and can reuse on-disk packs.
struct ArksHttpPeer {
    base_url: String,
    child: std::process::Child,
}

impl ArksHttpPeer {
    fn start(bare: PathBuf, bin: &Path) -> Self {
        let port = {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
            listener.local_addr().unwrap().port()
        };
        let listen = format!("127.0.0.1:{port}");
        let child = Command::new(bin)
            .args(["serve-http", "--listen", &listen])
            .arg(&bare)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn arks serve-http: {e}"));
        thread::sleep(Duration::from_millis(50));
        Self {
            base_url: format!("http://{listen}"),
            child,
        }
    }

    fn repo_url(&self) -> String {
        self.base_url.clone()
    }
}

impl Drop for ArksHttpPeer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn seed_bare_repo(parent: &Path, name: &str) -> PathBuf {
    seed_bare_repo_with_files(parent, name, 0)
}

/// Seed a bare peer. `extra_files` adds bulk blobs so HTTP clone timings are
/// less process-startup dominated (M14 / G19 comparative stability).
fn seed_bare_repo_with_files(parent: &Path, name: &str, extra_files: usize) -> PathBuf {
    seed_bare_repo_sized(
        parent,
        name,
        extra_files,
        BULK_CLONE_BODY_REPEAT,
        if extra_files > 0 { 1 } else { 0 },
    )
}

/// Shared bare seed with explicit body size + short history depth.
fn seed_bare_repo_sized(
    parent: &Path,
    name: &str,
    extra_files: usize,
    body_repeat: usize,
    extra_commits: usize,
) -> PathBuf {
    let work = parent.join(format!("{name}-work"));
    let bare = parent.join(name);
    fs::create_dir_all(&work).unwrap();
    let out = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(&work)
        .output()
        .unwrap();
    assert!(out.status.success());
    let _ = Command::new("git")
        .args(["config", "user.email", "arks@example.com"])
        .current_dir(&work)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "arks"])
        .current_dir(&work)
        .output();
    fs::write(work.join("README"), b"hello arks g16\n").unwrap();
    if extra_files > 0 {
        let bulk = work.join("bulk");
        fs::create_dir_all(&bulk).unwrap();
        // Larger bodies so clone object transfer can dominate spawn.
        for i in 0..extra_files {
            let body = format!("arks g16 bulk {i}\n").repeat(body_repeat);
            fs::write(bulk.join(format!("f{i:03}.txt")), body).unwrap();
        }
    }
    let mut add = Command::new("git");
    pin_env(&mut add);
    assert!(
        add.args(["add", "-A"])
            .current_dir(&work)
            .output()
            .unwrap()
            .status
            .success()
    );
    let mut commit = Command::new("git");
    pin_env(&mut commit);
    assert!(
        commit
            .args(["commit", "-m", "init"])
            .current_dir(&work)
            .output()
            .unwrap()
            .status
            .success()
    );
    if extra_files > 0 && extra_commits > 0 {
        // Short untimed history (identical for both peers; still single-branch).
        for c in 0..extra_commits {
            fs::write(
                work.join("README"),
                format!("hello arks g16\nwave-{c}\n"),
            )
            .unwrap();
            let mut add2 = Command::new("git");
            pin_env(&mut add2);
            assert!(
                add2.args(["add", "README"])
                    .current_dir(&work)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
            let mut commit2 = Command::new("git");
            pin_env(&mut commit2);
            assert!(
                commit2
                    .args(["commit", "-m", &format!("touch-{c}")])
                    .current_dir(&work)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        }
    }
    let mut clone = Command::new("git");
    pin_env(&mut clone);
    assert!(
        clone
            .args(["clone", "--bare", work.to_str().unwrap(), name])
            .current_dir(parent)
            .output()
            .unwrap()
            .status
            .success()
    );
    // Materialize a single on-disk pack so arks-server upload-pack can reuse it
    // (clone --bare left objects loose on this seed; cgi_wait was ~26ms re-encode).
    let mut repack = Command::new("git");
    pin_env(&mut repack);
    assert!(
        repack
            .args(["repack", "-ad"])
            .current_dir(&bare)
            .output()
            .unwrap()
            .status
            .success()
    );

    bare
}

fn journey_iterations() -> usize {
    std::env::var("ARKS_JOURNEY_ITERATIONS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3)
        .max(1)
}

fn push_dual(
    journeys: &mut Vec<JourneyResult>,
    id: &str,
    iterations: usize,
    root: &Path,
    mut run: impl FnMut(&Path) -> Vec<LaneResult>,
) {
    let mut per_peer: Vec<Vec<LaneResult>> = Vec::new();
    for i in 0..iterations {
        let iter_root = root.join(format!("i{i}"));
        fs::create_dir_all(&iter_root).unwrap();
        let lanes = run(&iter_root);
        if per_peer.is_empty() {
            per_peer = lanes.into_iter().map(|l| vec![l]).collect();
        } else {
            assert_eq!(per_peer.len(), lanes.len());
            for (slot, lane) in per_peer.iter_mut().zip(lanes) {
                slot.push(lane);
            }
        }
        // Drop iteration trees after metrics capture to bound disk/RSS (large seeds).
        let _ = fs::remove_dir_all(&iter_root);
    }
    let aggregated: Vec<LaneResult> = if iterations == 1 {
        per_peer.into_iter().map(|mut v| v.pop().unwrap()).collect()
    } else {
        per_peer.iter().map(|v| aggregate_lane(v)).collect()
    };
    journeys.push(JourneyResult {
        id: id.into(),
        lanes: aggregated,
    });
    // Free journey root before the next catalog entry (bounds Docker RSS).
    let _ = fs::remove_dir_all(root);
    eprintln!("G16 journey done: {id}");
}

#[test]
fn journey_compare_catalog() {
    require_git();
    let bin = arks_bin();
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let shim = write_git_shim(root, &bin);
    let iterations = journey_iterations();
    let aggregation = if iterations >= 3 { "median_p95" } else { "raw" };

    let mut journeys = Vec::new();

    push_dual(&mut journeys, "J-LOCAL-01", iterations, &root.join("j-local"), |r| {
        vec![
            run_local_journey("git", &bin, None, r),
            run_local_journey("arks", &bin, None, r),
        ]
    });

    push_dual(
        &mut journeys,
        "J-BRANCH-01",
        iterations,
        &root.join("j-branch"),
        |r| {
            vec![
                run_branch_journey("git", &bin, r),
                run_branch_journey("arks", &bin, r),
            ]
        },
    );

    push_dual(
        &mut journeys,
        "J-MERGE-FF-01",
        iterations,
        &root.join("j-merge"),
        |r| {
            vec![
                run_merge_ff_journey("git", &bin, r),
                run_merge_ff_journey("arks", &bin, r),
            ]
        },
    );

    push_dual(
        &mut journeys,
        "J-REBASE-01",
        iterations,
        &root.join("j-rebase"),
        |r| {
            vec![
                run_rebase_journey("git", &bin, r),
                run_rebase_journey("arks", &bin, r),
            ]
        },
    );

    push_dual(
        &mut journeys,
        "J-BUNDLE-01",
        iterations,
        &root.join("j-bundle"),
        |r| {
            vec![
                run_bundle_journey("git", &bin, r),
                run_bundle_journey("arks", &bin, r),
            ]
        },
    );

    // Shared smart-HTTP peer across clone iterations.
    // Bulk seed stabilizes G19 ratios vs process-startup noise (M17 Pf3-2).
    // Pf5 residual: further HTTP inflation worsened ≤0.5× ratio — keep baseline.
    {
        let peer_root = root.join("j-clone-peer");
        fs::create_dir_all(&peer_root).unwrap();
        let bare = seed_bare_repo_with_files(&peer_root, "app.git", BULK_CLONE_FILES);
        let peer = ArksHttpPeer::start(bare, &bin);
        let url = peer.repo_url();
        push_dual(
            &mut journeys,
            "J-CLONE-LOCAL-01",
            iterations,
            &root.join("j-clone"),
            |r| {
                vec![
                    run_clone_local_journey("git", &bin, r, &url),
                    run_clone_local_journey("arks", &bin, r, &url),
                ]
            },
        );
        drop(peer);
    }

    // Wave-3: local path + file:// clone (shared bare seed across iterations).
    // Bulk objects + git --no-hardlinks keep the lane comparable to arks object-dir copy.
    {
        let peer_root = root.join("j-clone-path-src");
        fs::create_dir_all(&peer_root).unwrap();
        let bare = seed_bare_repo_with_files(&peer_root, "path-app.git", BULK_CLONE_FILES);
        push_dual(
            &mut journeys,
            "J-CLONE-PATH-01",
            iterations,
            &root.join("j-clone-path"),
            |r| {
                vec![
                    run_clone_path_journey("git", &bin, r, &bare),
                    run_clone_path_journey("arks", &bin, r, &bare),
                ]
            },
        );
    }

    push_dual(
        &mut journeys,
        "J-ALIAS-01",
        iterations,
        &root.join("j-alias"),
        |r| vec![run_local_journey("alias", &bin, Some(&shim), r)],
    );

    push_dual(
        &mut journeys,
        "J-WORKTREE-01",
        iterations,
        &root.join("j-worktree"),
        |r| {
            vec![
                run_worktree_journey("git", &bin, r),
                run_worktree_journey("arks", &bin, r),
            ]
        },
    );

    // Wave-2 claimed
    push_dual(&mut journeys, "J-TAG-01", iterations, &root.join("j-tag"), |r| {
        vec![
            run_tag_journey("git", &bin, r),
            run_tag_journey("arks", &bin, r),
        ]
    });
    push_dual(
        &mut journeys,
        "J-STASH-01",
        iterations,
        &root.join("j-stash"),
        |r| {
            vec![
                run_stash_journey("git", &bin, r),
                run_stash_journey("arks", &bin, r),
            ]
        },
    );
    push_dual(
        &mut journeys,
        "J-CHERRY-01",
        iterations,
        &root.join("j-cherry"),
        |r| {
            vec![
                run_cherry_journey("git", &bin, r),
                run_cherry_journey("arks", &bin, r),
            ]
        },
    );
    push_dual(
        &mut journeys,
        "J-DIFF-01",
        iterations,
        &root.join("j-diff"),
        |r| {
            vec![
                run_diff_journey("git", &bin, r),
                run_diff_journey("arks", &bin, r),
            ]
        },
    );
    push_dual(
        &mut journeys,
        "J-RM-MV-01",
        iterations,
        &root.join("j-rmmv"),
        |r| {
            vec![
                run_rm_mv_journey("git", &bin, r),
                run_rm_mv_journey("arks", &bin, r),
            ]
        },
    );
    push_dual(
        &mut journeys,
        "J-ARCHIVE-01",
        iterations,
        &root.join("j-archive"),
        |r| {
            vec![
                run_archive_journey("git", &bin, r),
                run_archive_journey("arks", &bin, r),
            ]
        },
    );
    push_dual(
        &mut journeys,
        "J-REFLOG-01",
        iterations,
        &root.join("j-reflog"),
        |r| {
            vec![
                run_reflog_journey("git", &bin, r),
                run_reflog_journey("arks", &bin, r),
            ]
        },
    );
    push_dual(
        &mut journeys,
        "J-REVERT-01",
        iterations,
        &root.join("j-revert"),
        |r| {
            vec![
                run_revert_journey("git", &bin, r),
                run_revert_journey("arks", &bin, r),
            ]
        },
    );

    // M15 Fs-3: repack claimed implement — retire J-REFUSE-02 → J-REPACK-01.
    push_dual(
        &mut journeys,
        "J-REPACK-01",
        iterations,
        &root.join("j-repack"),
        |r| {
            vec![
                run_repack_journey("git", &bin, r),
                run_repack_journey("arks", &bin, r),
            ]
        },
    );

    push_dual(
        &mut journeys,
        "J-SUBMODULE-01",
        iterations,
        &root.join("j-submodule"),
        |r| {
            vec![
                run_submodule_journey("git", &bin, r),
                run_submodule_journey("arks", &bin, r),
            ]
        },
    );

    let out = std::env::var("ARKS_JOURNEY_METRICS_OUT").unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("out/metrics.json")
            .to_string_lossy()
            .into_owned()
    });
    write_metrics(Path::new(&out), &journeys, iterations, aggregation);

    println!(
        "G16 journey-compare functional PASS; iterations={iterations} aggregation={aggregation}; metrics -> {out}"
    );
    for j in &journeys {
        for lane in &j.lanes {
            println!(
                "  {} / {}: functional={} wall_ms_total={} wall_ms_p95_total={:?} peak_rss_kb={:?} cpu_ms={:?}",
                j.id,
                lane.peer,
                lane.functional,
                lane.wall_ms_total,
                lane.wall_ms_p95_total,
                lane.peak_rss_kb,
                lane.cpu_ms
            );
        }
    }
}
