# Evidence: git-daemon ↔ arks daemon symmetry

Stamp: `20261111035226`. Tip: `fc12914`.

| File | Source |
|------|--------|
| `pin-2.39.5-git-daemon-help.txt` | Docker `arks-verify-m1:0.1.0` → `git daemon -h` |
| `pin-2.39.5-man-synopsis-head.txt` | Same image man `git-daemon.1.gz` head |
| `pin-2.39.5-flag-extract.txt` | Regex extract of `--*` tokens from man |
| `host-2.54-git-daemon-help.txt` | Host Git for Windows skew only |
| `arks-daemon-probes.txt` | Host `target/debug/arks.exe daemon …` |
| `matrix.csv` | Option → Parity/Partial/Missing/Refuse/arks-only |

Research note: `.spec/research/20261111035226-git-daemon-arks-daemon-symmetry.research.md`
