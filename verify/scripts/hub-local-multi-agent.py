#!/usr/bin/env python3
"""Hub local multi-agent scenarios (HUB-MA-*) against one local `arks serve` forge.

SoT: .spec/sot/arks-hub-local-multi-agent-pr.sot.md. Called by
verify/scripts/run-hub-local-multi-agent.sh with the arks binary under test.

Topology: one `arks serve` process on loopback serves the forge API and smart
HTTP for the forge's Git directory. N agents are separate accounts on that
forge, each with its own HOME, token file and hosts file, logged in with its own
token through `arks auth login --with-token`. Agents run concurrently (threads,
each spawning the arks binary) and only ever invoke `arks`: never `gh` or `git`.

Each scenario asserts the expected end state on the forge and writes its
transcript to <out>/<scenario>.log; the summary goes to <out>/summary.json.
Exit 0 only when every selected scenario passed.
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
                status = "PASS"
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
