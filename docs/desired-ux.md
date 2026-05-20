# envx CLI — desired UX

A design target. Captures the polished version of the CLI that the website's
hero terminal mockup depicts, alongside what the CLI actually does today.
Use this when prioritizing UX work.

> Snapshot taken against `envx 2.13.0` on 2026-05-20.

---

## The vision (what the marketing hero shows)

```
~/work/api $ envx link
✔ linked to project api (env: production)

~/work/api $ envx set DATABASE_URL
? value › ********************
✔ encrypted with 8 recipient keys

~/work/api $ envx run -- cargo run
→ decrypting 14 vars
   Compiling api v0.1.0
    Finished `dev` profile [unoptimized]
     Running `target/debug/api`
listening on 0.0.0.0:3000
```

Three things make this feel good:

1. **Every command prints a one-line success summary** (`✔ linked to X`, `✔ encrypted with N keys`, `→ decrypting N vars`).
2. **The summary includes the meaningful counts** — number of recipients, number of vars decrypted — so the user can trust something happened.
3. **The project has an environment** (`api / production`), so the line carries enough context to feel safe before running commands in prod.

---

## What actually happens today (2.13.0)

### `envx link`

- Interactive `Choice::choose_project` picker.
- On success: **prints nothing**. Just exits 0.
- On already-linked: prints a 4-line "already linked, use `envx unlink` or `envx link --force`" hint.
- **No concept of environments.** A project is a flat namespace of variables.

### `envx set <KEY>`

- Interactive prompt for the value (or accepts `KEY=VALUE` positional / stdin).
- On success: `Uploaded N variables` and `IDs: [...]` (the raw debug-formatted Vec).
- **No mention of recipients, encryption, or what the value was bound to.**

### `envx run -- <cmd>`

- Decrypts in the background and execs the child process.
- **No banner**, no "decrypting N vars". The user sees only the child's stdout/stderr, exactly as if envx weren't there.
- This is great for piping into other tools, terrible for first-time confidence.

### `envx variables`

- Prints a nice unicode box-drawing table by default.
- Has `--kv` (KEY=VALUE lines), `--json`, `--filter <regex>`, `--all`.
- This one is already polished.

### `envx shell`

- Prints `Entering subshell with envx variables available. Type 'exit' to exit.`
- On exit: `Exited subshell, envx variables no longer available.`
- This is one of the few commands that already does the "narrate what happened" thing.

### `envx auth`

- Dumps an ASCII-armored PGP message to stdout. No human-readable summary.
- Looks broken to anyone who doesn't already know what they're looking at.

---

## Gaps to close (concrete checklist)

If we want the hero mockup to be honest, here's the punch list:

### Tier 1 — small, high-impact (post these and the website is no longer lying)

- [x] **`envx link`** prints `→ linked <cwd> → project <short-id>` on success. _Shipped opt-in behind `settings.loud=true` (a15ddee); see Tier 2 caveat below._
- [ ] **`envx set`** success message changes from `Uploaded N variables` + `IDs: [...]` to `✔ set N variable(s), encrypted to M recipient(s)`. Drop the IDs debug print (or gate behind `--verbose`).
- [ ] **`envx unset`** already says `Deleted N variable(s)` — keep, but colorize and ✔-prefix for consistency.
- [ ] **`envx auth`** prints `✔ authenticated as <name> <email> (<short-fpr>)` instead of dumping the PGP token. Move the token dump behind `--token` or `--json`.
- [ ] **`envx upload`** confirm success message exists; if not, add one.
- [x] **`envx gen`** prints `→ created GPG key <short-fpr>` on success. _Bonus from a15ddee, also loud-gated._

### Tier 2 — `envx run` confidence banner

- [x] **`envx run`** prints `→ injected N decrypted vars` on stderr before execing. _Shipped a15ddee, gated on `settings.loud=true`. Inverted the default (off, not on) from the original spec — the suppression knob is the config flag, not `--silent`._
- [x] Make sure the banner goes to **stderr** so it doesn't pollute piped stdout. _Verified via smoke test on the `loud` commit — child stdout stays clean._

**Caveat for marketing:** Tier 1 and Tier 2 shipped opt-in. Users following the website's getting-started won't see arrow lines unless they run `envx config set settings.loud true`. To make the website's demo accurate by default, either (a) flip the default to `true` once existing CI consumers are warned, or (b) keep the website honest by showing the loud-mode toggle in the hero or quickstart.

### Tier 3 — the environment concept

This is the biggest gap. The website / hero mockup leans on `env: production`, but envx has no environments.

Two ways to bridge:

**Option A: drop the env concept from marketing.** Position envx purely around "one project = one set of variables." Multiple environments = multiple projects. This is honest about the current model and avoids new schema work.

**Option B: add environments.** Schema becomes `project → environments[] → variables[]`. `envx link` picks both. `envx env switch <name>` rotates. Migration: an "envx 2.x" project becomes a "default" environment of an "envx 3.x" project.

Option A is the right call for v1 of the website. Add a callout in the docs: "envx doesn't model environments — make one project per environment, or use variable name prefixes." Revisit Option B for 3.0 if users actually ask for it.

### Tier 4 — nice-to-haves surfaced by the audit

- [ ] **`envx list-projects`** rows where the project has no name show as just a UUID + dash. Either fall back to "(unnamed)" or fetch+display the path it's linked to locally.
- [x] **Old config schema panic**: ~~the installed `/usr/local/bin/envx` panicked with `missing field 'keys' at line 122 column 1`~~. Resolved going the other direction in 340d6a6: the dead `keys` field was removed entirely. Legacy configs that still carry it deserialize fine because `Config` doesn't use `#[serde(deny_unknown_fields)]`; the field is silently ignored and stripped on next write. Verified against the real on-disk config.
- [ ] **Standardize on `✔` / `→` / `✖` prefixes** across all commands. Right now some use green text, some use plain text, some use nothing. _Partial: `→` is consistent within loud-mode output (a15ddee), but the non-loud default output is still mixed._

---

## How this maps back to the website

Until Tier 1 lands, the website hero either:

- **(a)** depicts an aspirational UX with a disclaimer, or
- **(b)** depicts the real UX (boring, but honest).

Recommendation: **(b)** for the launch — show `envx variables` (already polished) as the hero terminal instead of `link`/`set`/`run`. Switch back to the aspirational version once Tier 1 ships.

The website draft currently shows the aspirational version. Don't merge that hero until Tier 1 is in.
