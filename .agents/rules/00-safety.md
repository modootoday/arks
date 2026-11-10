# Safety & engagement rules

- Do not commit secrets, credentials, private keys, or live tokens.
- Do not invent attack / exploit tooling against third-party systems; lab and owned fixtures only.
- Do not disable sandboxes, clean-room isolation, or verification gates to force a green result.
- Prefer fail-closed when required evidence (primary citation, clean-room receipt, owning SoT) is missing.
- Treat copyrighted primary Git docs as cite-and-summarize — do not paste large verbatim text into the tree.
- Do not invent product **Claimed** / clean-room **PASS** (rule `28-evidence-honesty`).
- Do not `git push` while push-deferred unless the owner explicitly authorizes (rule `91-commit-hygiene`).
- Do not spawn stock `git`/`gh` from product paths (rule `30-no-git-gh-subprocess`).
