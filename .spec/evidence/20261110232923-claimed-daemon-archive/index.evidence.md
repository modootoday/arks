# Residual note: B5 archive compare (Partial — not Claimed)

- **Gate:** B5 ([[sot:root/arks-claimed-residual-gates]])
- **Disposition:** **Partial** — do **not** treat as Claimed
- **Venue:** `dod-claimed-daemon-archive` (tip ≥ `3f1a484`)
- **What matched:** protocol `upload-archive` path; structural members (`README.txt`, `src/`, `src/run.sh`), types, modes (664/775), payload sizes
- **Mismatch class (byte-identical Missing):** ustar **header encoding** — first differ block0 offset **151** (chksum digits: stock `0x34` vs arks `0x33`); device major/minor **ASCII `0` vs NUL** (~offsets 329+). Semantic tarfile fields agreed; raw header bytes did not.
- **B5+:** **Refuse (terminal)** this residual wave (`LOG_DAEMON` Partial ≠ facility Claimed)
- **Policy:** stop host/tar hex dump loops; graduate Claimed only with byte-identical receipt + SoT amend

Artifacts: prior `receipt.json` if present is **superseded** as non-Claimed residual note.
