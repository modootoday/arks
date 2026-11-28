#!/usr/bin/env python3
"""Build PEP 427 / PEP 600 binary PyPI wheels wrapping prebuilt `arks` binaries.

Zero external Python build dependencies (stdlib only). Wraps the exact prebuilt
binaries emitted by `.github/workflows/release.yml` job `dist` so PyPI wheels
ship bit-identical executables to GitHub Releases.

Cite: .spec/sot/external-distribution-and-registry-publishing.sot.md
"""

from __future__ import annotations

import argparse
import base64
import csv
import hashlib
import io
import re
import tarfile
import zipfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

TARGET_SPECS = {
    "x86_64-unknown-linux-gnu": {
        "archive": "arks-x86_64-unknown-linux-gnu.tar.gz",
        "bin_name": "arks",
        "wheel_tag": "manylinux_2_39_x86_64",
    },
    "aarch64-apple-darwin": {
        "archive": "arks-aarch64-apple-darwin.tar.gz",
        "bin_name": "arks",
        "wheel_tag": "macosx_11_0_arm64",
    },
    "x86_64-pc-windows-msvc": {
        "archive": "arks-x86_64-pc-windows-msvc.zip",
        "bin_name": "arks.exe",
        "wheel_tag": "win_amd64",
    },
}


def read_workspace_version() -> str:
    cargo_toml = (REPO_ROOT / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'^version\s*=\s*"([^"]+)"', cargo_toml, re.MULTILINE)
    if not match:
        raise SystemExit("FAIL: unable to read workspace version from Cargo.toml")
    return match.group(1)


def extract_binary_from_archive(archive_path: Path, bin_name: str) -> bytes:
    if archive_path.suffix == ".zip":
        with zipfile.ZipFile(archive_path, "r") as zf:
            for member in zf.namelist():
                if Path(member).name == bin_name:
                    return zf.read(member)
        raise SystemExit(f"FAIL: {bin_name} not found in {archive_path}")

    with tarfile.open(archive_path, "r:gz") as tf:
        for member in tf.getmembers():
            if member.isfile() and Path(member.name).name == bin_name:
                f = tf.extractfile(member)
                if f is not None:
                    return f.read()
    raise SystemExit(f"FAIL: {bin_name} not found in {archive_path}")


def record_digest(data: bytes) -> tuple[str, str]:
    digest = hashlib.sha256(data).digest()
    encoded = base64.urlsafe_b64encode(digest).rstrip(b"=").decode("ascii")
    return f"sha256={encoded}", str(len(data))


def build_metadata(version: str, readme_text: str) -> bytes:
    headers = [
        "Metadata-Version: 2.1",
        "Name: arks",
        f"Version: {version}",
        "Summary: arks for Git - native Rust Git-compatible CLI, daemon, forge client, and self-hosted server",
        "Home-page: https://modootoday.github.io/arks/",
        "Project-URL: Repository, https://github.com/modootoday/arks",
        "Project-URL: Documentation, https://modootoday.github.io/arks/",
        "Project-URL: Changelog, https://github.com/modootoday/arks/blob/main/CHANGELOG.md",
        "Project-URL: Issues, https://github.com/modootoday/arks/issues",
        "License: Apache-2.0 OR MIT",
        "Keywords: git,vcs,cli,concurrency,arks",
        "Classifier: Development Status :: 4 - Beta",
        "Classifier: Environment :: Console",
        "Classifier: Intended Audience :: Developers",
        "Classifier: License :: OSI Approved :: Apache Software License",
        "Classifier: License :: OSI Approved :: MIT License",
        "Classifier: Operating System :: MacOS :: MacOS X",
        "Classifier: Operating System :: Microsoft :: Windows",
        "Classifier: Operating System :: POSIX :: Linux",
        "Classifier: Programming Language :: Rust",
        "Classifier: Topic :: Software Development :: Version Control :: Git",
        "Requires-Python: >=3.8",
        "Description-Content-Type: text/markdown",
        "",
        readme_text,
    ]
    return "\n".join(headers).encode("utf-8")


def build_wheel(
    *,
    version: str,
    target: str,
    binary_bytes: bytes,
    out_dir: Path,
    readme_text: str,
    license_files: dict[str, bytes],
) -> Path:
    spec = TARGET_SPECS[target]
    wheel_tag = spec["wheel_tag"]
    bin_name = spec["bin_name"]
    dist_info = f"arks-{version}.dist-info"
    data_scripts = f"arks-{version}.data/scripts"
    wheel_name = f"arks-{version}-py3-none-{wheel_tag}.whl"
    wheel_path = out_dir / wheel_name

    wheel_meta = (
        "Wheel-Version: 1.0\n"
        "Generator: arks-wheel-packer (0.1.0)\n"
        "Root-Is-Purelib: false\n"
        f"Tag: py3-none-{wheel_tag}\n"
    ).encode("utf-8")

    entries: list[tuple[str, bytes, int]] = [
        (f"{data_scripts}/{bin_name}", binary_bytes, 0o100755),
        (f"{dist_info}/METADATA", build_metadata(version, readme_text), 0o100644),
        (f"{dist_info}/WHEEL", wheel_meta, 0o100644),
    ]
    for lic_name, lic_bytes in sorted(license_files.items()):
        entries.append((f"{dist_info}/licenses/{lic_name}", lic_bytes, 0o100644))

    record_rows: list[list[str]] = []
    out_dir.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(wheel_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for arcname, payload, mode in entries:
            zi = zipfile.ZipInfo(arcname, date_time=(2026, 11, 28, 0, 0, 0))
            zi.compress_type = zipfile.ZIP_DEFLATED
            zi.external_attr = (mode & 0xFFFF) << 16
            zf.writestr(zi, payload)
            digest, size = record_digest(payload)
            record_rows.append([arcname, digest, size])

        record_name = f"{dist_info}/RECORD"
        record_rows.append([record_name, "", ""])
        buf = io.StringIO()
        writer = csv.writer(buf, lineterminator="\n")
        writer.writerows(record_rows)
        zi_rec = zipfile.ZipInfo(record_name, date_time=(2026, 11, 28, 0, 0, 0))
        zi_rec.compress_type = zipfile.ZIP_DEFLATED
        zi_rec.external_attr = (0o100644 & 0xFFFF) << 16
        zf.writestr(zi_rec, buf.getvalue().encode("utf-8"))

    return wheel_path


def main() -> int:
    parser = argparse.ArgumentParser(description="Build arks PyPI binary wheels")
    parser.add_argument("--version", default=None, help="Package version (default: Cargo.toml)")
    parser.add_argument(
        "--artifacts-dir",
        type=Path,
        default=None,
        help="Directory containing dist archives (arks-*.tar.gz / .zip)",
    )
    parser.add_argument(
        "--single-binary",
        action="append",
        default=[],
        help="Override target binary directly as TARGET=PATH (for local smoke tests)",
    )
    parser.add_argument("--out-dir", type=Path, required=True, help="Output directory for .whl files")
    args = parser.parse_args()

    version = args.version or read_workspace_version()
    readme_text = (REPO_ROOT / "README.md").read_text(encoding="utf-8")
    license_files = {
        name: (REPO_ROOT / name).read_bytes()
        for name in ("LICENSE", "LICENSE-APACHE", "LICENSE-MIT")
        if (REPO_ROOT / name).is_file()
    }

    binaries: dict[str, bytes] = {}
    for item in args.single_binary:
        target, sep, path_str = item.partition("=")
        if not sep or target not in TARGET_SPECS:
            raise SystemExit(f"FAIL: invalid --single-binary '{item}'")
        binaries[target] = Path(path_str).read_bytes()

    if args.artifacts_dir is not None:
        for target, spec in TARGET_SPECS.items():
            if target in binaries:
                continue
            archive_path = args.artifacts_dir / spec["archive"]
            if archive_path.is_file():
                binaries[target] = extract_binary_from_archive(archive_path, spec["bin_name"])

    if not binaries:
        raise SystemExit("FAIL: no binaries found (pass --artifacts-dir or --single-binary)")

    for target, payload in sorted(binaries.items()):
        whl = build_wheel(
            version=version,
            target=target,
            binary_bytes=payload,
            out_dir=args.out_dir,
            readme_text=readme_text,
            license_files=license_files,
        )
        print(f"built {whl.name} ({whl.stat().st_size} bytes)")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
