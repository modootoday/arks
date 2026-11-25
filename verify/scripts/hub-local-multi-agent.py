#!/usr/bin/env python3
"""Hub local multi-agent scenarios (HUB-MA-*) against one local `arks serve` forge.

SoT: .spec/sot/arks-hub-local-multi-agent-pr.sot.md. Called by
verify/scripts/run-hub-local-multi-agent.sh with the arks binary under test.

Topology: one `arks serve` process on loopback serves the forge API and smart
HTTP for the forge's Git directory. N agents are separate accounts on that
forge, each with its own HOME, token file and hosts file, logged in with its own
token through `arks auth login --with-token`. Agents run concurrently (threads,
each spawning the arks binary) and only ever invoke `arks`: never `gh` or `git`.

The shared repository local/playground moves over smart HTTP. The HUB-MA-10 soak
gives every agent its own repositories and reaches their Git directories by path,
because one smart-HTTP listener serves one repository.

Each scenario asserts the expected end state on the forge and writes its
transcript to <out>/<scenario>.log; the summary goes to <out>/summary.json.
HUB-MA-09 reports N/A: it checks the Actions refusal, not a feature.
Exit 0 only when every selected scenario passed (or, for 09, refused as required).
"""

from __future__ import annotations

import concurrent.futures as cf
import json
import os
import socket
import subprocess
import sys
import threading
import time
import traceback
import urllib.request
from pathlib import Path

REPO = "local/playground"
EMAIL = "agent@example.com"


class Fail(Exception):
    pass


def free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def wait_port(port: int, what: str) -> None:
    for _ in range(400):
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.2):
                return
        except OSError:
            time.sleep(0.05)
    raise Fail(f"{what} listener on {port} not ready")


class Log:
    """Per-scenario transcript shared by the agents' threads."""

    def __init__(self, path: Path):
        self.path = path
        self.lock = threading.Lock()
        self.fh = open(path, "w", encoding="utf-8")

    def write(self, text: str) -> None:
        with self.lock:
            self.fh.write(text if text.endswith("\n") else text + "\n")
            self.fh.flush()

    def close(self) -> None:
        self.fh.close()


class Forge:
    def __init__(self, arks: str, work: Path):
        self.arks = arks
        self.work = work
        self.data = work / "forge"
        self.bare = self.data / "git" / "local" / "playground.git"
        self.http_port = free_port()
        self.api_port = free_port()
        self.proc: subprocess.Popen | None = None

    @property
    def api(self) -> str:
        return f"http://127.0.0.1:{self.api_port}"

    @property
    def git_url(self) -> str:
        return f"http://127.0.0.1:{self.http_port}"

    def start(self) -> None:
        self.bare.parent.mkdir(parents=True, exist_ok=True)
        env = dict(os.environ, ARKS_PERSONALITY="git", HOME=str(self.work / "serverhome"))
        subprocess.run(
            [self.arks, "init", "-q", "--bare", "-b", "main", str(self.bare)],
            env=env, check=True, capture_output=True,
        )
        env["ARKS_SERVE_CONFIG"] = str(self.work / "serve.conf")
        self.err = open(self.work / "server.err", "w")
        self.proc = subprocess.Popen(
            [self.arks, "serve", "--no-ssh",
             "--listen", f"127.0.0.1:{self.http_port}",
             "--forge-listen", f"127.0.0.1:{self.api_port}",
             "--forge-data", str(self.data),
             "--forge-git-url", self.git_url,
             str(self.bare)],
            env=env, stdout=self.err, stderr=subprocess.STDOUT,
        )
        wait_port(self.http_port, "smart HTTP")
        wait_port(self.api_port, "forge API")

    def stop(self) -> None:
        if self.proc and self.proc.poll() is None:
            self.proc.terminate()
            try:
                self.proc.wait(10)
            except subprocess.TimeoutExpired:
                self.proc.kill()

    def health(self) -> dict:
        with urllib.request.urlopen(self.api + "/health", timeout=5) as r:
            return json.loads(r.read())


class Agent:
    def __init__(self, forge: Forge, name: str, token: str):
        self.forge = forge
        self.name = name
        self.token = token
        self.home = forge.work / "agents" / name
        cfg = self.home / ".config" / "arks"
        cfg.mkdir(parents=True, exist_ok=True)
        self.env = {
            k: v for k, v in os.environ.items()
            if not k.startswith("ARKS_") and k not in ("GIT_DIR", "GIT_WORK_TREE")
        }
        self.env.update({
            "HOME": str(self.home),
            "ARKS_HUB_API": forge.api,
            "ARKS_HUB_REPO": REPO,
            "ARKS_HUB_TOKEN_FILE": str(cfg / "hub-token"),
            "ARKS_HUB_HOSTS_FILE": str(cfg / "hub-hosts.json"),
            "ARKS_HUB_CLIENT_STATE": str(cfg / "hub-client.json"),
            "GIT_AUTHOR_NAME": name, "GIT_AUTHOR_EMAIL": EMAIL,
            "GIT_COMMITTER_NAME": name, "GIT_COMMITTER_EMAIL": EMAIL,
        })
        self.clone = self.home / "work"
        self.log: Log | None = None

    def run(self, args, *, expect=0, stdin=None, cwd=None, personality=None, env=None):
        e = dict(self.env)
        if personality:
            e["ARKS_PERSONALITY"] = personality
        if env:
            e.update(env)
        t0 = time.monotonic()
        p = subprocess.run(
            [self.forge.arks, *args], input=stdin, cwd=cwd or self.home, env=e,
            capture_output=True, text=True, timeout=120,
        )
        dt = time.monotonic() - t0
        if self.log:
            self.log.write(
                f"[{self.name}] $ arks {' '.join(args)}  -> exit {p.returncode} ({dt:.2f}s)\n"
                + "".join(f"    | {line}\n" for line in (p.stdout + p.stderr).splitlines()[:40])
            )
        if expect is not None and p.returncode != expect:
            raise Fail(
                f"{self.name}: arks {' '.join(args)} exit {p.returncode} (want {expect})\n"
                f"{p.stdout}{p.stderr}"
            )
        return p

    def out(self, args, **kw) -> str:
        return self.run(args, **kw).stdout

    def json(self, args, **kw):
        return json.loads(self.out(args, **kw) or "null")

    def git(self, args, **kw):
        kw.setdefault("cwd", self.clone)
        return self.run(args, personality="git", **kw)

    def login(self, login=None, token=None) -> None:
        self.run(
            ["auth", "login", "--with-token"], stdin=(token or self.token) + "\n",
            env={"ARKS_HUB_LOGIN": login or self.name},
        )


def concurrently(agents, fn, *, barrier=True):
    """Run fn(agent) for every agent at once; re-raise the first failure."""
    gate = threading.Barrier(len(agents)) if barrier else None

    def body(a):
        if gate:
            gate.wait(30)
        return fn(a)

    with cf.ThreadPoolExecutor(max_workers=len(agents)) as ex:
        futs = [ex.submit(body, a) for a in agents]
        results, errors = [], []
        for f in futs:
            try:
                results.append(f.result())
            except Exception as e:  # noqa: BLE001 - collected and re-raised
                errors.append(e)
    if errors:
        raise errors[0]
    return results


def check(cond, msg):
    if not cond:
        raise Fail(msg)


def tail_number(url: str) -> int:
    return int(url.strip().splitlines()[-1].rstrip("/").rsplit("/", 1)[-1])


# --------------------------------------------------------------------------- scenarios


class Run:
    def __init__(self, arks: str, work: Path, out: Path, n: int):
        self.forge = Forge(arks, work)
        self.out = out
        self.n = n
        self.agents: list[Agent] = []
        self.facts: dict[str, dict] = {}

    def setup(self) -> None:
        self.forge.start()
        self.agents = [
            Agent(self.forge, f"agent-{k:02d}", f"tok-agent-{k:02d}-{os.getpid()}")
            for k in range(1, self.n + 1)
        ]

    # P1: loopback smoke with one agent.
    def smoke(self, log: Log) -> dict:
        a = self.agents[0]
        h = self.forge.health()
        check(h.get("ok") is True, f"health {h}")
        a.login()
        user = a.json(["api", "/user"])
        check(user["login"] == a.name, f"/user login {user}")
        repo = a.json(["repo", "view", REPO, "--json", "nameWithOwner,isEmpty"])
        check(repo["nameWithOwner"] == REPO, f"repo view {repo}")
        issues = a.json(["issue", "list", "--json", "number"])
        check(issues == [], f"fresh forge has issues {issues}")
        # Seed main through smart HTTP so later scenarios have Git content.
        url = a.out(["repo", "clone", REPO]).split("clone_url=", 1)[1].split()[0]
        check(url == f"{self.forge.git_url}/{REPO}.git", f"clone_url {url}")
        a.run(["clone", "-q", url, str(a.clone)], personality="git")
        (a.clone / "README.md").write_text("# playground\n")
        (a.clone / "shared.txt").write_text("base line\n")
        a.git(["add", "README.md", "shared.txt"])
        a.git(["commit", "-q", "-m", "Seed playground"])
        a.git(["push", "-q", "origin", "main"])
        top = a.json(["repo", "view", REPO, "--json", "isEmpty,defaultBranchRef"])
        check(top["isEmpty"] is False, f"seeded repo still empty {top}")
        return {"agent": a.name, "health": h, "clone_url": url, "seeded": True}

    def login_all(self, log: Log) -> None:
        concurrently(self.agents, lambda a: a.login())

    # HUB-MA-01: N agents open distinct issues on one repo.
    def ma01(self, log: Log) -> dict:
        per = 3
        self.login_all(log)

        def work(a: Agent):
            nums = []
            for j in range(per):
                url = a.out(["issue", "create", "-t", f"{a.name} issue {j}",
                             "-b", f"opened by {a.name}"])
                nums.append(tail_number(url))
            return a.name, nums

        made = dict(concurrently(self.agents, work))
        allnums = sorted(x for v in made.values() for x in v)
        check(allnums == list(range(1, self.n * per + 1)),
              f"issue numbers not distinct/contiguous: {allnums}")
        reader = self.agents[-1]
        listed = reader.json(["issue", "list", "--state", "all", "-L", "500",
                              "--json", "number,title,author"])
        check(len(listed) == self.n * per, f"issue list count {len(listed)}")
        by_num = {i["number"]: i for i in listed}
        for name, nums in made.items():
            for j, num in enumerate(nums):
                i = by_num[num]
                check(i["author"]["login"] == name, f"#{num} author {i['author']}")
                check(i["title"].startswith(name + " issue "), f"#{num} title {i['title']}")
        # Each agent views another agent's issue.
        def view(a: Agent):
            k = self.agents.index(a)
            other = self.agents[(k + 1) % self.n].name
            num = made[other][0]
            v = a.json(["issue", "view", str(num), "--json", "number,title,author,state"])
            check(v["author"]["login"] == other and v["state"] == "OPEN", f"view #{num} {v}")
        concurrently(self.agents, view)
        return {"agents": self.n, "issues": len(listed), "numbers": allnums}

    # HUB-MA-02: N agents comment on the same issue.
    def ma02(self, log: Log) -> dict:
        per = 4
        self.login_all(log)
        owner = self.agents[0]
        num = tail_number(owner.out(["issue", "create", "-t", "contended thread", "-b", "talk"]))

        def work(a: Agent):
            for j in range(per):
                a.out(["issue", "comment", str(num), "-b", f"{a.name} comment {j}"])

        concurrently(self.agents, work)
        v = self.agents[-1].json(["issue", "view", str(num), "--json", "comments"])
        comments = v["comments"]
        check(len(comments) == self.n * per, f"comment count {len(comments)}")
        got = [(c["author"]["login"], c["body"]) for c in comments]
        want = {(a.name, f"{a.name} comment {j}") for a in self.agents for j in range(per)}
        check(set(got) == want and len(set(got)) == len(got), f"comments {got}")
        for a in self.agents:
            mine = [b for (who, b) in got if who == a.name]
            check(mine == [f"{a.name} comment {j}" for j in range(per)],
                  f"{a.name} comment order {mine}")
        ids = [c.get("id") for c in comments]
        check(len(set(ids)) == len(ids), f"comment ids not unique {ids}")
        return {"issue": num, "comments": len(comments)}

    # HUB-MA-03: N agents open PRs from divergent heads.
    def ma03(self, log: Log) -> dict:
        self.login_all(log)
        url = f"{self.forge.git_url}/{REPO}.git"

        def work(a: Agent):
            k = self.agents.index(a) + 1
            if not (a.clone / ".git").exists():
                a.run(["clone", "-q", url, str(a.clone)], personality="git")
            a.git(["fetch", "-q", "origin"])
            branch = f"{a.name}/feature"
            a.git(["checkout", "-q", "-B", branch, "origin/main"])
            (a.clone / "agents").mkdir(exist_ok=True)
            (a.clone / "agents" / f"{a.name}.txt").write_text(f"work of {a.name}\n")
            paths = [f"agents/{a.name}.txt"]
            if k <= 2:
                # The first two agents edit the same line: a real conflict pair.
                (a.clone / "shared.txt").write_text(f"{a.name} line\n")
                paths.append("shared.txt")
            a.git(["add", *paths])
            a.git(["commit", "-q", "-m", f"Add work of {a.name}"])
            a.git(["push", "-q", "origin", branch])
            out = a.out(["pr", "create", "-H", branch, "-B", "main",
                         "-t", f"Work of {a.name}", "-b", f"from {a.name}"])
            return a.name, (tail_number(out), branch, paths)

        made = dict(concurrently(self.agents, work))
        nums = sorted(v[0] for v in made.values())
        check(nums == list(range(1, self.n + 1)), f"PR numbers {nums}")
        listed = self.agents[0].json(["pr", "list", "--json",
                                      "number,headRefName,baseRefName,author,state"])
        check(len(listed) == self.n, f"pr list count {len(listed)}")
        by = {p["number"]: p for p in listed}
        for name, (num, branch, paths) in made.items():
            p = by[num]
            check(p["headRefName"] == branch and p["baseRefName"] == "main"
                  and p["author"]["login"] == name and p["state"] == "OPEN", f"#{num} {p}")
        # Each agent inspects the next agent's PR: one commit, only its own files.
        def inspect(a: Agent):
            k = self.agents.index(a)
            other = self.agents[(k + 1) % self.n].name
            num, branch, paths = made[other]
            v = a.json(["pr", "view", str(num), "--json",
                        "number,headRefOid,mergeable,files,commits,author"])
            check(v["author"]["login"] == other, f"#{num} author {v['author']}")
            check(v["mergeable"] == "MERGEABLE", f"#{num} mergeable {v['mergeable']}")
            check(sorted(f["path"] for f in v["files"]) == sorted(paths), f"#{num} files {v['files']}")
            check(len(v["commits"]) == 1, f"#{num} commits {len(v['commits'])}")
            names = a.out(["pr", "diff", str(num), "--name-only"]).split()
            check(sorted(names) == sorted(paths), f"#{num} diff names {names}")
        concurrently(self.agents, inspect)
        self.facts["prs"] = {k: list(v) for k, v in made.items()}
        return {"prs": nums, "conflict_pair": [self.agents[0].name, self.agents[1].name]}

    # HUB-MA-04: cross-agent review + comment load, checks, then merges with a conflict.
    def ma04(self, log: Log) -> dict:
        if "prs" not in self.facts:
            self.ma03(log)
        self.login_all(log)
        prs = self.facts["prs"]
        names = [a.name for a in self.agents]

        def review(a: Agent):
            k = names.index(a.name)
            for other in names:
                if other == a.name:
                    continue
                num = prs[other][0]
                a.out(["pr", "comment", str(num), "-b", f"{a.name} looked at {other}"])
                if other == names[(k + 1) % self.n]:
                    a.out(["pr", "review", str(num), "--approve", "-b", f"{a.name} approves"])
                else:
                    a.out(["pr", "review", str(num), "--comment", "-b", f"{a.name} notes"])

        concurrently(self.agents, review)
        # A status per PR head, posted by its author, then `pr checks` read by another agent.
        def status(a: Agent):
            num = prs[a.name][0]
            sha = a.out(["pr", "view", str(num), "--json", "headRefOid", "--jq", ".headRefOid"]).strip()
            a.out(["api", "-X", "POST", f"repos/{REPO}/statuses/{sha}",
                   "-f", "state=success", "-f", f"context=ci/{a.name}"])
        concurrently(self.agents, status)
        for a in self.agents:
            num = prs[a.name][0]
            v = a.json(["pr", "view", str(num), "--json", "comments,reviews"])
            check(len(v["comments"]) == self.n - 1, f"#{num} comments {len(v['comments'])}")
            rv = [(r["author"]["login"], r["state"]) for r in v["reviews"]]
            want = {(o, "APPROVED" if names[(names.index(o) + 1) % self.n] == a.name else "COMMENTED")
                    for o in names if o != a.name}
            check(set(rv) == want and len(rv) == self.n - 1, f"#{num} reviews {rv}")
            reader = self.agents[(names.index(a.name) + 2) % self.n]
            chk = reader.out(["pr", "checks", str(num)])
            check(f"ci/{a.name}\tpass" in chk, f"#{num} checks {chk}")
        # Every author merges its own PR at once; the conflict pair cannot both land.
        def merge(a: Agent):
            num = prs[a.name][0]
            p = a.run(["pr", "merge", str(num), "--merge"], expect=None)
            return a.name, p.returncode, p.stdout + p.stderr

        res = {n_: (rc, txt) for n_, rc, txt in concurrently(self.agents, merge)}
        pair = names[:2]
        failed = [n_ for n_, (rc, _) in res.items() if rc != 0]
        check(len(failed) == 1 and failed[0] in pair, f"merge results {res}")
        loser = failed[0]
        check("not mergeable" in res[loser][1], f"{loser} merge refusal {res[loser][1]}")
        winner = pair[0] if loser == pair[1] else pair[1]
        la = self.agents[names.index(loser)]
        v = la.json(["pr", "view", str(prs[loser][0]), "--json", "mergeable,state"])
        check(v["state"] == "OPEN" and v["mergeable"] == "CONFLICTING", f"loser PR {v}")
        # The loser merges main into its branch, resolves the conflict and pushes.
        la.git(["fetch", "-q", "origin"])
        p = la.git(["merge", "--no-edit", "origin/main"], expect=None)
        check(p.returncode != 0, f"local merge should conflict: {p.stdout}{p.stderr}")
        (la.clone / "shared.txt").write_text(f"{winner} line\n{loser} line\n")
        la.git(["add", "shared.txt"])
        la.git(["commit", "-q", "--no-edit"])
        la.git(["push", "-q", "origin", prs[loser][1]])
        v = la.json(["pr", "view", str(prs[loser][0]), "--json", "mergeable"])
        check(v["mergeable"] == "MERGEABLE", f"loser PR after resolve {v}")
        la.out(["pr", "merge", str(prs[loser][0]), "--merge"])
        listed = self.agents[-1].json(["pr", "list", "--state", "merged", "--json", "number,mergedBy"])
        check(sorted(p["number"] for p in listed) == list(range(1, self.n + 1)), f"merged {listed}")
        # The final main holds every agent's file and the resolved shared.txt.
        w = self.agents[-1]
        w.git(["fetch", "-q", "origin"])
        tree = w.out(["ls-tree", "-r", "--name-only", "origin/main"], personality="git",
                     cwd=w.clone).split()
        for n_ in names:
            check(f"agents/{n_}.txt" in tree, f"main misses agents/{n_}.txt: {tree}")
        shared = w.out(["show", "origin/main:shared.txt"], personality="git", cwd=w.clone)
        check(shared == f"{winner} line\n{loser} line\n", f"shared.txt {shared!r}")
        return {"merged": self.n, "conflict_loser": loser, "conflict_winner": winner,
                "reviews_per_pr": self.n - 1, "comments_per_pr": self.n - 1}

    # HUB-MA-05: label storm on one issue and one PR.
    def ma05(self, log: Log) -> dict:
        self.login_all(log)
        a0 = self.agents[0]
        issue = tail_number(a0.out(["issue", "create", "-t", "label storm", "-b", "x"]))
        a0.git(["fetch", "-q", "origin"]) if (a0.clone / ".git").exists() else a0.run(
            ["clone", "-q", f"{self.forge.git_url}/{REPO}.git", str(a0.clone)], personality="git")
        a0.git(["checkout", "-q", "-B", "label-storm", "origin/main"])
        (a0.clone / "storm.txt").write_text("storm\n")
        a0.git(["add", "storm.txt"])
        a0.git(["commit", "-q", "-m", "Label storm target"])
        a0.git(["push", "-q", "origin", "label-storm"])
        pr = tail_number(a0.out(["pr", "create", "-H", "label-storm", "-B", "main",
                                 "-t", "label storm", "-b", "x"]))

        # Every agent creates its own label; all race to create one shared label.
        def create(a: Agent):
            a.out(["label", "create", f"ma-{a.name}", "-c", "00ff00", "-d", a.name])
            p = a.run(["label", "create", "ma-shared", "-c", "ff0000"], expect=None)
            return p.returncode

        rcs = concurrently(self.agents, create)
        check(rcs.count(0) == 1, f"shared label create exits {rcs} (want exactly one success)")
        labels = {x["name"] for x in a0.json(["label", "list", "-L", "500", "--json", "name"])}
        want_all = {f"ma-{a.name}" for a in self.agents} | {"ma-shared"}
        check(want_all <= labels, f"label list {labels}")

        def add(a: Agent):
            for _ in range(2):
                a.out(["issue", "edit", str(issue), "--add-label", f"ma-{a.name}",
                       "--add-label", "ma-shared"])
                a.out(["pr", "edit", str(pr), "--add-label", f"ma-{a.name}",
                       "--add-label", "ma-shared"])

        concurrently(self.agents, add)
        got_i = {x["name"] for x in a0.json(["issue", "view", str(issue), "--json", "labels"])["labels"]}
        got_p = {x["name"] for x in a0.json(["pr", "view", str(pr), "--json", "labels"])["labels"]}
        check(got_i == want_all, f"issue labels after adds {sorted(got_i)}")
        check(got_p == want_all, f"PR labels after adds {sorted(got_p)}")
        odd = [a for k, a in enumerate(self.agents) if k % 2 == 1]

        def remove(a: Agent):
            mine = ["--remove-label", "ma-shared"]
            if a in odd:
                mine += ["--remove-label", f"ma-{a.name}"]
            a.out(["issue", "edit", str(issue), *mine])
            a.out(["pr", "edit", str(pr), *mine])

        concurrently(self.agents, remove)
        want_end = {f"ma-{a.name}" for a in self.agents if a not in odd}
        got_i = {x["name"] for x in a0.json(["issue", "view", str(issue), "--json", "labels"])["labels"]}
        got_p = {x["name"] for x in a0.json(["pr", "view", str(pr), "--json", "labels"])["labels"]}
        check(got_i == want_end, f"issue labels after removes {sorted(got_i)}")
        check(got_p == want_end, f"PR labels after removes {sorted(got_p)}")
        filt = a0.json(["issue", "list", "-l", f"ma-{self.agents[0].name}", "--json", "number"])
        check([x["number"] for x in filt] == [issue], f"issue list -l {filt}")
        return {"issue": issue, "pr": pr, "final_labels": sorted(want_end),
                "shared_create_successes": 1}

    # HUB-MA-07: token isolation across agents.
    def ma07(self, log: Log) -> dict:
        self.login_all(log)
        rounds = 5

        def probe(a: Agent):
            for _ in range(rounds):
                who = a.json(["api", "/user"])["login"]
                check(who == a.name, f"{a.name} saw /user {who}")
                tok = a.out(["auth", "token"]).strip()
                check(tok == a.token, f"{a.name} auth token {tok}")
                st = a.out(["auth", "status", "--json", "hosts", "--jq",
                            ".hosts[][] | select(.active) | .login"]).split()
                check(st == [a.name], f"{a.name} auth status {st}")

        concurrently(self.agents, probe)
        # Agent 1 adds a second account and switches while the others keep working.
        a1, a2 = self.agents[0], self.agents[1]
        alt = f"{a1.name}-alt"
        a1.login(login=alt, token=a1.token + "-alt")

        def mixed(a: Agent):
            num = tail_number(a.out(["issue", "create", "-t", f"auth probe {a.name}", "-b", "x"]))
            return a.name, num

        made = dict(concurrently(self.agents, mixed))
        v = a2.json(["issue", "view", str(made[a1.name]), "--json", "author"])
        check(v["author"]["login"] == alt, f"switched account authored as {v}")
        for a in self.agents[1:]:
            v = a1.json(["issue", "view", str(made[a.name]), "--json", "author"])
            check(v["author"]["login"] == a.name, f"{a.name} issue author {v}")
        a1.run(["auth", "switch", "-u", a1.name])
        who = a1.json(["api", "/user"])["login"]
        check(who == a1.name, f"after switch back /user {who}")
        # An unregistered token is refused; logging one agent out leaves the others alone.
        bad = a2.run(["api", "/user"], expect=None,
                     env={"ARKS_HUB_TOKEN_FILE": str(a2.home / "bogus-token")})
        check(bad.returncode != 0, "missing token file accepted")
        (a2.home / "junk-token").write_text("not-a-registered-token\n")
        bad = a2.run(["api", "/user"], expect=None,
                     env={"ARKS_HUB_TOKEN_FILE": str(a2.home / "junk-token")})
        check(bad.returncode != 0 and "authentication" in (bad.stdout + bad.stderr).lower(),
              f"junk token accepted: {bad.stdout}{bad.stderr}")
        a2.run(["auth", "logout", "-u", a2.name])
        p = a2.run(["api", "/user"], expect=None)
        check(p.returncode != 0, f"logged-out agent still authenticated: {p.stdout}")
        concurrently(self.agents[2:] + [a1], lambda a: check(
            a.json(["api", "/user"])["login"] == a.name, f"{a.name} lost its session"))
        a2.login()
        return {"agents": self.n, "rounds": rounds, "switched_account": alt,
                "logout_isolated": a2.name}

    # Shared helper: one agent pushes a branch with one file and opens a PR on it.
    def branch_pr(self, a: Agent, branch: str, path: str, text: str, title: str,
                  extra=()) -> int:
        url = f"{self.forge.git_url}/{REPO}.git"
        if not (a.clone / ".git").exists():
            a.run(["clone", "-q", url, str(a.clone)], personality="git")
        a.git(["fetch", "-q", "origin"])
        a.git(["checkout", "-q", "-B", branch, "origin/main"])
        (a.clone / path).parent.mkdir(parents=True, exist_ok=True)
        (a.clone / path).write_text(text)
        a.git(["add", path])
        a.git(["commit", "-q", "-m", title])
        a.git(["push", "-q", "origin", branch])
        return tail_number(a.out(["pr", "create", "-H", branch, "-B", "main", "-t", title,
                                  "-b", text, *extra]))

    # HUB-MA-06: release cut while agents keep commenting and reading.
    def ma06(self, log: Log) -> dict:
        self.login_all(log)
        rel, rival, *talkers = self.agents
        issue = tail_number(rel.out(["issue", "create", "-t", "release train", "-b", "x"]))
        pr = self.branch_pr(rel, "release-train", "train/notes.md", "train\n", "Release train")
        asset = self.forge.work / "tool.bin"
        asset.write_bytes(os.urandom(4096))
        rounds = 6

        def act(a: Agent):
            if a in (rel, rival):
                p = a.run(["release", "create", "v1.0.0", f"{asset}#Tool", "--target", "main",
                           "-t", f"v1.0.0 by {a.name}", "--generate-notes"], expect=None)
                return a.name, p.returncode
            for j in range(rounds):
                a.out(["issue", "comment", str(issue), "-b", f"{a.name} release talk {j}"])
                a.out(["pr", "comment", str(pr), "-b", f"{a.name} pr talk {j}"])
                a.json(["issue", "list", "--json", "number"])
                a.json(["pr", "view", str(pr), "--json", "number,state"])
                a.run(["release", "list"])
            return a.name, 0

        res = dict(concurrently(self.agents, act))
        winners = [n_ for n_ in (rel.name, rival.name) if res[n_] == 0]
        check(len(winners) == 1, f"release create race {res} (want exactly one winner)")
        winner = winners[0]
        # Talk landed completely while the release was cut.
        v = rel.json(["issue", "view", str(issue), "--json", "comments"])
        check(len(v["comments"]) == len(talkers) * rounds, f"issue comments {len(v['comments'])}")
        v = rel.json(["pr", "view", str(pr), "--json", "comments"])
        check(len(v["comments"]) == len(talkers) * rounds, f"pr comments {len(v['comments'])}")
        # Every agent sees the one release; its tag is main's tip; the asset round-trips.
        def see(a: Agent):
            r = a.json(["release", "view", "v1.0.0", "--json", "tagName,name,assets,isDraft,author"])
            check(r["tagName"] == "v1.0.0" and r["name"] == f"v1.0.0 by {winner}"
                  and r["isDraft"] is False, f"{a.name} release view {r}")
            check([x["label"] for x in r["assets"]] == ["Tool"], f"assets {r['assets']}")
            dest = a.home / "dl"
            a.out(["release", "download", "v1.0.0", "-p", "tool*", "-D", str(dest), "--clobber"])
            check((dest / "tool.bin").read_bytes() == asset.read_bytes(), f"{a.name} asset bytes")
            lst = a.out(["release", "list"])
            check(lst.count("v1.0.0") >= 1, f"{a.name} release list {lst}")
        concurrently(self.agents, see)
        w = self.agents[-1]
        if not (w.clone / ".git").exists():
            w.run(["clone", "-q", f"{self.forge.git_url}/{REPO}.git", str(w.clone)], personality="git")
        # ls-remote: the arks smart-HTTP fetch does not store tags yet (separate task).
        refs = dict(reversed(line.split("\t")) for line in w.out(
            ["ls-remote", "origin"], personality="git", cwd=w.clone).splitlines())
        tag, main = refs.get("refs/tags/v1.0.0"), refs.get("refs/heads/main")
        check(tag is not None and tag == main, f"tag v1.0.0 {tag} != main {main}")
        return {"release_winner": winner, "talkers": len(talkers), "rounds": rounds,
                "issue": issue, "pr": pr, "tag": tag}

    # HUB-MA-08: search / status fan-out while a writer keeps adding noise.
    def ma08(self, log: Log) -> dict:
        self.login_all(log)
        token = f"needle{os.getpid()}"
        made: dict[str, list[int]] = {}

        names = [a.name for a in self.agents]
        nxt = {x: names[(k + 1) % self.n] for k, x in enumerate(names)}
        prv = {v: k for k, v in nxt.items()}

        # Each agent assigns its first issue to, and requests review from, the next agent.
        def seed(a: Agent):
            nums = [tail_number(a.out(["issue", "create", "-t", f"{token} {a.name} {j}", "-b", "q",
                                       *(["-a", nxt[a.name]] if j == 0 else [])]))
                    for j in range(2)]
            pr = self.branch_pr(a, f"{a.name}/fanout", f"fanout/{a.name}.txt",
                                f"{token} code of {a.name}\n", f"{token} change by {a.name}",
                                extra=["-r", nxt[a.name]])
            return a.name, (nums, pr)

        made = dict(concurrently(self.agents, seed))
        # Code and commit search index the default branch (as on GitHub): land one index.
        a0 = self.agents[0]
        a0.git(["fetch", "-q", "origin"])
        a0.git(["checkout", "-q", "-B", "main", "origin/main"])
        (a0.clone / "fanout").mkdir(exist_ok=True)
        (a0.clone / "fanout" / "index.txt").write_text(
            "".join(f"{token} code of {a.name}\n" for a in self.agents))
        a0.git(["add", "fanout/index.txt"])
        a0.git(["commit", "-q", "-m", f"{token} index"])
        a0.git(["push", "-q", "origin", "main"])
        index_sha = a0.out(["rev-parse", "HEAD"], personality="git", cwd=a0.clone).strip()
        want_issues = sorted(x for v in made.values() for x in v[0])
        want_prs = sorted(v[1] for v in made.values())
        writer, *readers = self.agents
        stop = threading.Event()
        noise = []

        def write():
            while not stop.is_set():
                noise.append(tail_number(writer.out(["issue", "create", "-t", "noise", "-b", "n"])))
                writer.out(["issue", "comment", str(want_issues[0]), "-b", "noise"])

        rounds = 4

        def read(a: Agent):
            for _ in range(rounds):
                got = a.json(["search", "issues", token, "--repo", REPO, "-L", "200",
                              "--json", "number"])
                check(sorted(x["number"] for x in got) == want_issues, f"{a.name} search issues {got}")
                mine = a.json(["search", "issues", f"{token} {a.name}", "--repo", REPO,
                               "--json", "number"])
                check(sorted(x["number"] for x in mine) == sorted(made[a.name][0]),
                      f"{a.name} own search {mine}")
                prs = a.json(["search", "prs", token, "--repo", REPO, "-L", "200", "--json", "number"])
                check(sorted(x["number"] for x in prs) == want_prs, f"{a.name} search prs {prs}")
                code = a.out(["search", "code", f"code of {a.name}", "--repo", REPO])
                check("local/playground:fanout/index.txt:" in code, f"{a.name} search code {code}")
                st = a.out(["pr", "status"])
                own = st.split("Created by you", 1)[-1].split("Requesting a code review", 1)[0]
                check(f"#{made[a.name][1]}" in own, f"{a.name} pr status {st}")
                asked = st.split("Requesting a code review from you", 1)[-1]
                check(f"#{made[prv[a.name]][1]}" in asked, f"{a.name} pr status review {st}")
                hs = a.out(["hub", "status"])
                assigned = hs.split("Assigned Issues", 1)[-1].split("Assigned Pull Requests", 1)[0]
                review = hs.split("Review Requests", 1)[-1]
                check(f"#{made[prv[a.name]][0][0]}" in assigned, f"{a.name} hub status issues {hs}")
                check(f"#{made[prv[a.name]][1]}" in review, f"{a.name} hub status reviews {hs}")

        with cf.ThreadPoolExecutor(max_workers=1) as ex:
            wf = ex.submit(write)
            try:
                concurrently(readers, read)
            finally:
                stop.set()
                wf.result()
        commits = readers[0].json(["search", "commits", token, "--repo", REPO, "--json", "sha"])
        # The forge indexes commits on every branch: the index plus one commit per PR head.
        heads = {readers[0].out(["pr", "view", str(v[1]), "--json", "headRefOid", "--jq",
                                 ".headRefOid"]).strip() for v in made.values()}
        check({c["sha"] for c in commits} == heads | {index_sha} and len(commits) == self.n + 1,
              f"search commits {commits}")
        agg = readers[0].out(["hub", "status"])
        return {"readers": len(readers), "rounds": rounds, "issues": len(want_issues),
                "prs": len(want_prs), "noise_issues": len(noise), "status_bytes": len(agg)}

    # HUB-MA-09: Actions are GitHub-platform-only (adr:root/20261119230000 item 3), so the
    # scenario is N/A. What the runner checks is the refusal: every agent's `run` / `workflow`
    # call exits 2 without reaching a forge (the API points at a closed port).
    def ma09(self, log: Log) -> dict:
        closed = {"ARKS_HUB_API": f"http://127.0.0.1:{free_port()}"}

        def refuse(a: Agent):
            for argv in (["run", "list"], ["workflow", "list"], ["hub", "run", "view", "1"],
                         ["hub", "workflow", "run", "ci.yml"]):
                p = a.run(argv, expect=2, env=closed)
                check("out of scope" in (p.stdout + p.stderr).lower()
                      or "platform" in (p.stdout + p.stderr).lower(),
                      f"{a.name} {argv} refusal text {p.stderr}")

        concurrently(self.agents, refuse)
        return {"disposition": "N/A", "reason": "adr:root/20261119230000 item 3",
                "refusals": 4 * self.n}

    # HUB-MA-10: full v1 matrix soak, all agents at once, over several rounds.
    def ma10(self, log: Log) -> dict:
        rounds = int(os.environ.get("ARKS_HUB_MA_SOAK_ROUNDS", "2"))
        soak = Soak(self)
        for r in range(1, rounds + 1):
            concurrently(self.agents, lambda a: soak.journey(a, r))
        want = v1_targets()
        missing = sorted(want - set(soak.touched))
        check(not missing, f"v1 leaves not touched by the soak: {missing}")
        # Repositories, releases and accounts of different agents stayed apart.
        reader = self.agents[-1]
        for a in self.agents:
            for r in range(1, rounds + 1):
                v = reader.json(["hub", "repo", "view", f"{a.name}/soak{r}", "--json",
                                 "description"])
                check(v["description"] == f"soak by {a.name}", f"{a.name}/soak{r} {v}")
        return {"rounds": rounds, "agents": self.n, "v1_targets": len(want),
                "touched": len(set(soak.touched) & want),
                "calls": sum(soak.touched.values())}


def ssh_pub(comment: str) -> str:
    """A fresh, well-formed ssh-ed25519 public key line (random point bytes)."""
    import base64
    import struct
    blob = b"".join(struct.pack(">I", len(x)) + x for x in (b"ssh-ed25519", os.urandom(32)))
    return f"ssh-ed25519 {base64.b64encode(blob).decode()} {comment}\n"


GPG_FIXTURE = """-----BEGIN PGP PUBLIC KEY BLOCK-----

mDMEav9wixYJKwYBBAHaRw8BAQdAQjLeQBeoT4aDP0hMcp8V0S5KliR4Wo7OTC0l
qsx/yCK0JUZpeHR1cmUgS2V5IDxmaXh0dXJlQGV4YW1wbGUuaW52YWxpZD6IkwQT
FgoAOxYhBI7bByOhOpVqZC07Mi8wt6QweLGoBQJq/3CLAhsDBQsJCAcCAiICBhUK
CQgLAgQWAgMBAh4HAheAAAoJEC8wt6QweLGoQmcA/j1iFqpGmluCZTSL06ANxSXg
tb3A038eqruIHmAhLWu5AQDLo/2RiEm7W4TOdcPttVtdoGWKtQK53SBDHVsTh/EZ
AA==
=SSXi
-----END PGP PUBLIC KEY BLOCK-----
"""
GPG_FIXTURE_ID = "2F30B7A43078B1A8"

# Group rows of the v1 matrix are touched through their help.
SOAK_GROUPS = [
    "auth", "pr", "issue", "issue artifact", "release", "repo", "repo gitignore",
    "repo license", "repo autolink", "repo deploy-key", "label", "search", "org", "config",
    "alias", "gist", "discussion", "project", "ssh-key", "gpg-key", "ruleset", "milestone",
]


def v1_targets() -> set[str]:
    import csv
    path = Path(__file__).resolve().parents[1] / "matrices" / "hub-gh-arks-e2e-matrix.csv"
    with open(path, encoding="utf-8-sig") as f:
        return {r["arks_hub_target"] for r in csv.DictReader(f) if r["phase"] == "v1"}


class Soak:
    """HUB-MA-10: every agent walks the whole v1 leaf set in its own repositories,
    all agents at once, for several rounds; a ledger records each leaf touched."""

    def __init__(self, run: "Run"):
        self.run = run
        self.lock = threading.Lock()
        self.touched: dict[str, int] = {}

    def s(self, a: Agent, key: str, argv, *, expect=0, repo=None, **kw) -> str:
        env = dict(kw.pop("env", {}))
        if repo:
            env["ARKS_HUB_REPO"] = repo
        p = a.run(["hub", *argv], expect=expect, env=env, **kw)
        with self.lock:
            self.touched[key] = self.touched.get(key, 0) + 1
        return p.stdout

    def journey(self, a: Agent, r: int) -> None:
        s, me, forge = self.s, a.name, self.run.forge
        R, D = f"{me}/soak{r}", f"{me}/dest{r}"
        k = self.run.agents.index(a)
        for g in SOAK_GROUPS:
            s(a, g, [*g.split(), "--help"])
        # accounts: a second account, switch to it and back, log it out
        a.login()
        with self.lock:
            self.touched["auth login"] = self.touched.get("auth login", 0) + 1
        alt = f"{me}-alt{r}"
        a.login(login=alt, token=f"{a.token}-alt{r}")
        s(a, "auth switch", ["auth", "switch", "-u", me])
        st = s(a, "auth status", ["auth", "status", "--json", "hosts", "--jq",
                                  ".hosts[][] | select(.active) | .login"])
        check(st.split() == [me], f"{me} active account {st}")
        check(s(a, "auth token", ["auth", "token"]).strip() == a.token, f"{me} token")
        s(a, "auth refresh", ["auth", "refresh"])
        s(a, "auth logout", ["auth", "logout", "-u", alt])
        # repositories with Git content
        s(a, "repo gitignore list", ["repo", "gitignore", "list"])
        s(a, "repo gitignore view", ["repo", "gitignore", "view", "Rust"])
        s(a, "repo license list", ["repo", "license", "list"])
        s(a, "repo license view", ["repo", "license", "view", "mit"])
        s(a, "repo create", ["repo", "create", f"soak{r}", "--public", "--add-readme",
                             "--gitignore", "Rust", "--license", "mit", "-d", "soak"])
        s(a, "repo create", ["repo", "create", f"dest{r}", "--public", "-d", "dest"])
        lst = s(a, "repo list", ["repo", "list", me, "--json", "nameWithOwner", "--jq",
                                 ".[].nameWithOwner"])
        check(R in lst.split() and D in lst.split(), f"{me} repo list {lst}")
        s(a, "repo view", ["repo", "view", R, "--json", "nameWithOwner"])
        s(a, "repo edit", ["repo", "edit", R, "--description", f"soak by {me}"])
        s(a, "repo set-default", ["repo", "set-default", R])
        s(a, "repo read-dir", ["repo", "read-dir"], repo=R)
        s(a, "repo read-file", ["repo", "read-file", "LICENSE"], repo=R)
        s(a, "repo clone", ["repo", "clone", R])
        bare = forge.data / "git" / me / f"soak{r}.git"
        wd = a.home / f"soak{r}"
        a.run(["clone", "-q", str(bare), str(wd)], personality="git")
        for br, path in (("feature", "f.txt"), ("topic", "t.txt")):
            a.git(["checkout", "-q", "-B", br, "origin/main"], cwd=wd)
            (wd / path).write_text(f"{br} by {me}\n")
            a.git(["add", path], cwd=wd)
            a.git(["commit", "-q", "-m", f"Add {path}"], cwd=wd)
            a.git(["push", "-q", "origin", br], cwd=wd)
        # labels and milestones
        s(a, "label create", ["label", "create", "soak", "-c", "00ff00", "-d", "soak"], repo=R)
        s(a, "label edit", ["label", "edit", "soak", "-d", "soak label"], repo=R)
        s(a, "label create", ["label", "create", "tmp", "-c", "ff0000"], repo=R)
        s(a, "label delete", ["label", "delete", "tmp", "--yes"], repo=R)
        s(a, "label list", ["label", "list"], repo=R)
        s(a, "label clone", ["label", "clone", R], repo=D)
        s(a, "milestone create", ["milestone", "create", "m1", "-d", "first"], repo=R)
        s(a, "milestone create", ["milestone", "create", "m2"], repo=R)
        s(a, "milestone list", ["milestone", "list"], repo=R)
        s(a, "milestone edit", ["milestone", "edit", "m2", "--title", "m2b"], repo=R)
        s(a, "milestone close", ["milestone", "close", "m1"], repo=R)
        # issues
        i1 = tail_number(s(a, "issue create", ["issue", "create", "-t", f"soak {me}", "-b", "b",
                                               "-l", "soak", "-m", "m2b", "-a", "@me"], repo=R))
        i2 = tail_number(s(a, "issue create", ["issue", "create", "-t", "move me", "-b", "b"], repo=R))
        i3 = tail_number(s(a, "issue create", ["issue", "create", "-t", "drop me", "-b", "b"], repo=R))
        s(a, "issue list", ["issue", "list"], repo=R)
        s(a, "issue view", ["issue", "view", str(i1)], repo=R)
        s(a, "issue status", ["issue", "status"], repo=R)
        s(a, "issue comment", ["issue", "comment", str(i1), "-b", "c1"], repo=R)
        s(a, "issue edit", ["issue", "edit", str(i1), "-t", f"soak {me} ed", "--add-label", "x"], repo=R)
        s(a, "issue close", ["issue", "close", str(i1)], repo=R)
        s(a, "issue reopen", ["issue", "reopen", str(i1)], repo=R)
        s(a, "issue lock", ["issue", "lock", str(i1)], repo=R)
        s(a, "issue unlock", ["issue", "unlock", str(i1)], repo=R)
        s(a, "issue pin", ["issue", "pin", str(i1)], repo=R)
        s(a, "issue unpin", ["issue", "unpin", str(i1)], repo=R)
        s(a, "issue develop", ["issue", "develop", str(i1), "--name", f"fix-{r}"], repo=R)
        plan = a.home / f"plan{r}.md"
        plan.write_text("# Plan\nsteps\n")
        s(a, "issue artifact create", ["issue", "artifact", "create", str(i1), f"{plan}#Plan",
                                       "https://runbooks.example.com/soak"], repo=R)
        arts = json.loads(s(a, "issue artifact list", ["issue", "artifact", "list", str(i1),
                                                       "--json", "number,name"], repo=R))
        check(len(arts) == 2, f"{me} artifacts {arts}")
        first, second = sorted(x["number"] for x in arts)
        s(a, "issue artifact edit", ["issue", "artifact", "edit", str(i1), str(first),
                                     "--body", "steps v2"], repo=R)
        s(a, "issue artifact view", ["issue", "artifact", "view", str(i1), str(first)], repo=R)
        s(a, "issue artifact download", ["issue", "artifact", "download", str(i1), "--dir",
                                         str(a.home / f"art{r}")], repo=R)
        s(a, "issue artifact delete", ["issue", "artifact", "delete", str(i1), str(second),
                                       "--yes"], repo=R)
        s(a, "issue transfer", ["issue", "transfer", str(i2), D], repo=R)
        s(a, "issue delete", ["issue", "delete", str(i3), "--yes"], repo=R)
        s(a, "milestone delete", ["milestone", "delete", "m2b", "--yes"], repo=R)
        # pull requests
        p1 = tail_number(s(a, "pr create", ["pr", "create", "-H", "feature", "-B", "main",
                                            "-t", "Add f", "-b", "soak", "-l", "soak", "--draft"],
                           repo=R))
        s(a, "pr ready", ["pr", "ready", str(p1)], repo=R)
        s(a, "pr list", ["pr", "list"], repo=R)
        s(a, "pr view", ["pr", "view", str(p1)], repo=R)
        s(a, "pr diff", ["pr", "diff", str(p1)], repo=R)
        s(a, "pr status", ["pr", "status"], repo=R, cwd=wd)
        s(a, "pr edit", ["pr", "edit", str(p1), "--add-label", "x", "-t", "Add f ed"], repo=R)
        s(a, "pr comment", ["pr", "comment", str(p1), "-b", "pc1"], repo=R)
        s(a, "pr review", ["pr", "review", str(p1), "--comment", "-b", "looks fine"], repo=R)
        s(a, "pr lock", ["pr", "lock", str(p1)], repo=R)
        s(a, "pr unlock", ["pr", "unlock", str(p1)], repo=R)
        s(a, "pr close", ["pr", "close", str(p1)], repo=R)
        s(a, "pr reopen", ["pr", "reopen", str(p1)], repo=R)
        sha = s(a, "pr view", ["pr", "view", str(p1), "--json", "headRefOid", "--jq",
                               ".headRefOid"], repo=R).strip()
        s(a, "api", ["api", "-X", "POST", f"repos/{R}/statuses/{sha}", "-f", "state=success",
                     "-f", "context=soak"])
        s(a, "pr checks", ["pr", "checks", str(p1)], repo=R)
        s(a, "pr checkout", ["pr", "checkout", str(p1)], repo=R, cwd=wd)
        s(a, "co", ["co", str(p1)], repo=R, cwd=wd)
        # rulesets: required check "soak" is green, so the merge passes
        rs = a.home / f"rs{r}.json"
        rs.write_text(json.dumps({
            "name": f"soak-{me}", "enforcement": "active",
            "conditions": {"ref_name": {"include": ["~DEFAULT_BRANCH"], "exclude": []}},
            "rules": [{"type": "required_status_checks", "parameters": {
                "required_status_checks": [{"context": "soak"}]}}]}))
        made = json.loads(s(a, "api", ["api", "-X", "POST", f"repos/{R}/rulesets", "--input",
                                       str(rs)]))
        s(a, "ruleset list", ["ruleset", "list"], repo=R)
        s(a, "ruleset view", ["ruleset", "view", str(made["id"])], repo=R)
        s(a, "rs", ["rs", "view", str(made["id"])], repo=R)
        s(a, "ruleset check", ["ruleset", "check", "--default"], repo=R)
        s(a, "pr merge", ["pr", "merge", str(p1), "--merge"], repo=R)
        s(a, "pr revert", ["pr", "revert", str(p1)], repo=R)
        p3 = tail_number(s(a, "pr create", ["pr", "create", "-H", "topic", "-B", "main",
                                            "-t", "Add t", "-b", "soak"], repo=R))
        s(a, "pr update-branch", ["pr", "update-branch", str(p3)], repo=R)
        # releases
        asset = a.home / f"tool{r}.bin"
        asset.write_bytes(os.urandom(512))
        extra = a.home / f"extra{r}.bin"
        extra.write_bytes(b"extra\n")
        s(a, "release create", ["release", "create", f"v{r}.0.0", f"{asset}#Tool", "-t",
                                f"soak {r}", "--generate-notes"], repo=R)
        s(a, "release upload", ["release", "upload", f"v{r}.0.0", str(extra)], repo=R)
        s(a, "release list", ["release", "list"], repo=R)
        s(a, "release view", ["release", "view", f"v{r}.0.0"], repo=R)
        s(a, "release download", ["release", "download", f"v{r}.0.0", "-p", "tool*", "-D",
                                  str(a.home / f"dl{r}"), "--clobber"], repo=R)
        check((a.home / f"dl{r}" / asset.name).read_bytes() == asset.read_bytes(), f"{me} asset")
        s(a, "release edit", ["release", "edit", f"v{r}.0.0", "--notes", "edited"], repo=R)
        s(a, "release delete-asset", ["release", "delete-asset", f"v{r}.0.0", extra.name, "-y"],
          repo=R)
        s(a, "release create", ["release", "create", f"v{r}.0.1", "-t", "tmp", "--notes", "n"],
          repo=R)
        s(a, "release delete", ["release", "delete", f"v{r}.0.1", "-y"], repo=R)
        # search, status, api, org, browse
        s(a, "search issues", ["search", "issues", f"soak {me}", "--repo", R])
        s(a, "search prs", ["search", "prs", "Add", "--repo", R])
        s(a, "search repos", ["search", "repos", f"soak{r}", "--owner", me])
        s(a, "search code", ["search", "code", "feature", "--repo", R])
        s(a, "search commits", ["search", "commits", "Add", "--repo", R])
        s(a, "status", ["status"])
        s(a, "api", ["api", "/user"])
        s(a, "org list", ["org", "list"])
        s(a, "browse", ["browse", str(i1), "--no-browser"], repo=R)
        # config, aliases, completion (per-agent client state)
        s(a, "config set", ["config", "set", "editor", f"ed-{me}"])
        got = s(a, "config get", ["config", "get", "editor"]).strip()
        check(got == f"ed-{me}", f"{me} config get {got}")
        s(a, "config list", ["config", "list"])
        s(a, "config clear-cache", ["config", "clear-cache"])
        s(a, "alias set", ["alias", "set", f"st{r}", "status"])
        aliases = a.home / f"aliases{r}.yml"
        aliases.write_text(f"im{r}: issue list\n")
        s(a, "alias import", ["alias", "import", str(aliases)])
        s(a, "alias list", ["alias", "list"])
        s(a, "alias delete", ["alias", "delete", f"st{r}"])
        s(a, "completion", ["completion", "-s", "bash"])
        # gists and discussions
        g1 = a.home / f"g{r}.txt"
        g1.write_text("alpha\n")
        g2 = a.home / f"g{r}-v2.txt"
        g2.write_text("alpha v2\n")
        gid = s(a, "gist create", ["gist", "create", str(g1), "-d", f"gist {me}"]).strip()
        gid = gid.rstrip("/").rsplit("/", 1)[-1]
        s(a, "gist list", ["gist", "list"])
        s(a, "gist view", ["gist", "view", gid])
        s(a, "gist edit", ["gist", "edit", gid, str(g2), "-f", g1.name])
        s(a, "gist rename", ["gist", "rename", gid, g1.name, f"renamed{r}.txt"])
        s(a, "gist clone", ["gist", "clone", gid, str(a.home / f"gist{r}")])
        s(a, "gist delete", ["gist", "delete", gid, "--yes"])
        dn = tail_number(s(a, "discussion create", ["discussion", "create", "-t", "plan", "-b",
                                                    "body", "-c", "Ideas"], repo=R))
        s(a, "discussion list", ["discussion", "list"], repo=R)
        s(a, "discussion edit", ["discussion", "edit", str(dn), "-t", "plan v2"], repo=R)
        s(a, "discussion comment", ["discussion", "comment", str(dn), "-b", "hello"], repo=R)
        s(a, "discussion view", ["discussion", "view", str(dn), "--comments"], repo=R)
        # projects (per-owner numbering)
        pn = tail_number(s(a, "project create", ["project", "create", "--title", f"Road {me}"]))
        s(a, "project edit", ["project", "edit", str(pn), "-d", "short"])
        s(a, "project field-create", ["project", "field-create", str(pn), "--name", "Note",
                                      "--data-type", "TEXT"])
        fields = json.loads(s(a, "project field-list", ["project", "field-list", str(pn),
                                                        "--format", "json"]))
        fid = next(f["id"] for f in fields["fields"] if f["name"] == "Note")
        s(a, "project item-add", ["project", "item-add", str(pn), "--url", f"{R}#{i1}"])
        item = json.loads(s(a, "project item-create", ["project", "item-create", str(pn),
                                                       "--title", "draft", "--format", "json"]))
        s(a, "project item-edit", ["project", "item-edit", "--id", str(item["id"]),
                                   "--project-id", str(pn), "--field-id", str(fid), "--text", "hi"])
        s(a, "project item-list", ["project", "item-list", str(pn)])
        s(a, "project item-archive", ["project", "item-archive", str(pn), "--id", str(item["id"])])
        cp = tail_number(s(a, "project copy", ["project", "copy", str(pn), "--title", "Copy"]))
        s(a, "project item-delete", ["project", "item-delete", str(pn), "--id", str(item["id"])])
        s(a, "project field-delete", ["project", "field-delete", "--id", str(fid)])
        s(a, "project link", ["project", "link", str(pn), "-R", R])
        s(a, "project unlink", ["project", "unlink", str(pn), "-R", R])
        s(a, "project mark-template", ["project", "mark-template", str(pn)])
        s(a, "project close", ["project", "close", str(pn)])
        s(a, "project list", ["project", "list", "--closed"])
        s(a, "project view", ["project", "view", str(pn)])
        s(a, "project delete", ["project", "delete", str(cp)])
        # keys: a fresh SSH key per agent and round; the one GPG fixture rotates
        key = a.home / f"id{r}.pub"
        key.write_text(ssh_pub(f"{me}@example.com"))
        s(a, "ssh-key add", ["ssh-key", "add", str(key), "-t", f"key{r}"])
        keys = json.loads(a.out(["hub", "api", "/user/keys"]))
        kid = next(x["id"] for x in keys if x["title"] == f"key{r}")
        s(a, "ssh-key list", ["ssh-key", "list"])
        s(a, "ssh-key delete", ["ssh-key", "delete", str(kid), "--yes"])
        s(a, "gpg-key list", ["gpg-key", "list"])
        if k == (r - 1) % self.run.n:
            pub = a.home / "pub.asc"
            pub.write_text(GPG_FIXTURE)
            s(a, "gpg-key add", ["gpg-key", "add", str(pub)])
            s(a, "gpg-key delete", ["gpg-key", "delete", GPG_FIXTURE_ID, "--yes"])
        dk = a.home / f"dk{r}.pub"
        dk.write_text(ssh_pub(f"deploy-{me}@example.com"))
        s(a, "repo deploy-key add", ["repo", "deploy-key", "add", str(dk), "-t", "ci"], repo=R)
        dks = json.loads(s(a, "repo deploy-key list", ["repo", "deploy-key", "list", "--json",
                                                       "id"], repo=R))
        s(a, "repo deploy-key delete", ["repo", "deploy-key", "delete", str(dks[0]["id"])], repo=R)
        s(a, "repo autolink create", ["repo", "autolink", "create", "TICKET-",
                                      "https://tracker.example.com/t/<num>"], repo=R)
        als = json.loads(s(a, "repo autolink list", ["repo", "autolink", "list", "--json", "id"],
                           repo=R))
        s(a, "repo autolink view", ["repo", "autolink", "view", str(als[0]["id"])], repo=R)
        s(a, "repo autolink delete", ["repo", "autolink", "delete", str(als[0]["id"]), "--yes"],
          repo=R)
        # forks, sync, rename, archive, delete
        s(a, "repo fork", ["repo", "fork", R, "--org", f"forks-{me}"])
        s(a, "repo sync", ["repo", "sync", f"forks-{me}/soak{r}"])
        s(a, "repo archive", ["repo", "archive", "--yes"], repo=D)
        s(a, "repo unarchive", ["repo", "unarchive", "--yes"], repo=D)
        s(a, "repo rename", ["repo", "rename", f"dest{r}x", "--yes"], repo=D)
        s(a, "repo delete", ["repo", "delete", f"{me}/dest{r}x", "--yes"])
        # the transitional hub serve alias on its own port (one agent per run)
        if k == 0 and r == 1:
            port = free_port()
            e = dict(a.env, ARKS_PERSONALITY="")
            proc = subprocess.Popen([forge.arks, "hub", "serve", "--listen", f"127.0.0.1:{port}",
                                     "--data", str(a.home / "hs-data")], env=e,
                                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            try:
                wait_port(port, "hub serve")
                with urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=5) as resp:
                    check(json.loads(resp.read()).get("ok") is True, "hub serve health")
                with self.lock:
                    self.touched["serve"] = self.touched.get("serve", 0) + 1
            finally:
                proc.terminate()
                proc.wait(10)


SCENARIOS = {
    "SMOKE": "smoke",
    "HUB-MA-01": "ma01",
    "HUB-MA-02": "ma02",
    "HUB-MA-03": "ma03",
    "HUB-MA-04": "ma04",
    "HUB-MA-05": "ma05",
    "HUB-MA-06": "ma06",
    "HUB-MA-07": "ma07",
    "HUB-MA-08": "ma08",
    "HUB-MA-09": "ma09",
    "HUB-MA-10": "ma10",
}


def main() -> int:
    arks, work, out = sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3])
    n = int(os.environ.get("ARKS_HUB_MA_AGENTS", "6"))
    want = os.environ.get("ARKS_HUB_MA_SCENARIOS", ",".join(SCENARIOS)).split(",")
    if "SMOKE" not in want:
        want.insert(0, "SMOKE")
    out.mkdir(parents=True, exist_ok=True)
    work.mkdir(parents=True, exist_ok=True)
    run = Run(arks, work, out, n)
    summary = {"agents": n, "scenarios": {}}
    ok = True
    try:
        run.setup()
        for sid in want:
            meth = getattr(run, SCENARIOS[sid])
            log = Log(out / f"{sid}.log")
            for a in run.agents:
                a.log = log
            t0 = time.monotonic()
            try:
                facts = meth(log)
                # An N/A scenario passes its refusal check; it is not a Claimed feature.
                status = "N/A" if facts.get("disposition") == "N/A" else "PASS"
            except Exception as e:  # noqa: BLE001 - recorded per scenario
                facts = {"error": str(e)[:4000]}
                log.write(traceback.format_exc())
                status = "FAIL"
                ok = False
            dt = round(time.monotonic() - t0, 2)
            log.write(f"== {sid} {status} ({dt}s)")
            log.close()
            summary["scenarios"][sid] = {"status": status, "seconds": dt, "facts": facts}
            print(f"hub-local-multi-agent: {sid} {status} ({dt}s)", flush=True)
            if status == "FAIL":
                print(facts["error"], file=sys.stderr)
    finally:
        run.forge.stop()
        try:
            (out / "server.err").write_text((work / "server.err").read_text())
        except OSError:
            pass
    (out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
