# Safety & engagement rules

- Do not commit secrets, credentials, private keys, or live tokens.
- Do not invent attack / exploit tooling against third-party systems; lab and owned fixtures only.
- Do not disable sandboxes, clean-room isolation, or verification gates to force a green result.
- Prefer fail-closed when required evidence (primary citation, clean-room receipt, owning SoT) is missing.
- Treat copyrighted primary Git docs as cite-and-summarize — do not paste large verbatim text into the tree.
- Do not invent product **Claimed** / clean-room **PASS** (rules `28-evidence-honesty`, `29-stub-vs-claimed`).
- Do not `git push` while push-deferred unless the owner explicitly authorizes (rule `91-commit-hygiene`).
- Do not force-push to `main`/`master`; do not stage `.spec.local/` or secrets (rule `91` + Cursor shell hooks).
- Do not spawn stock `git`/`gh` from product paths (rule `30-no-git-gh-subprocess`; skill `review-product-subprocess`).
- Do not call demo ports **2222**/**8080** “standard Git” (rule `34-ports-normative-vs-demo`).
