# Ambiguity: ask, don't assume

Agents default to silent non-interactive choices when requirements are unclear. That is forbidden for architecture and public surface decisions.

- If two interpretations would produce different crate layouts, CLI surfaces, or Git wire/on-disk semantics, **ask**.
- If a change might invent Git semantics without a primary citation, **stop** and ask / file research — do not improvise.
- If work might cross the CELIR / lease boundary into Git-compat SoTs (or the reverse), confirm scope first (rule `50-celir-boundary`).
- Prefer a short assumed-bullet list only when the user is unavailable, then stop before irreversible git/publish actions.
