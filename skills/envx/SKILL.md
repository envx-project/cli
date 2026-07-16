---
name: envx
description: Operate the envx CLI — an end-to-end-encrypted, GPG/PGP-backed environment-variable manager. Use this skill to install and set up envx on a fresh machine, generate a device key (`envx gen`), join an existing project on a new device via invites (`envx invite create` / `envx invite accept`), run commands with a project's secrets injected (`envx run -- <cmd>`), link a directory to a project (`envx link`), read/set/unset variables, manage project membership, and wire up non-interactive passphrase handling (keyring, `unsafe-password`, `password-command`). Trigger whenever the user says "set up envx", "envx on this machine", "authenticate envx", "join the envx project", "envx invite", "envx gen", "run this with envx", "envx run", "inject the env vars", "decrypt the env", "link this dir to envx", "envx says unauthorized", or "envx keeps asking for my password" — even if they don't name the exact subcommand. Prefer this over guessing envx flags from memory; document and run only what this build (v2.13.0) actually supports.
---

# envx — encrypted env-var manager

`envx` is an end-to-end-encrypted environment-variable manager backed by GPG/PGP. Variables live on a server (`sdk_url`, default `https://api.envx.sh`) encrypted to the public keys of every device that has access. Nothing is readable without a private key.

The mental model that makes everything else click:

- **Each device has its own key.** `envx gen` creates a GPG keypair under `~/.config/envx/keys/<fingerprint>/`; config lives in `~/.config/envx/config.json`. Keys are never copied between machines.
- **A project is a flat namespace of variables** (no environments — one project = one set of vars; model prod/staging as separate projects).
- **Access is granted per-device-key.** To let a new machine read a project, an existing member re-encrypts the project's variables to the new device's public key. That happens via an **invite** (`invite create` → `invite accept`) or directly (`project add-user`). A device that isn't a recipient gets `unauthorized`.

Verify capability quickly: `envx whoami` (prints your key fingerprint + uuid) and `envx list-projects` (projects this device can read). If those work, the device is set up.

## Install

```bash
curl -fsSL https://get.envx.sh | sh      # installs to /usr/local/bin (needs sudo)
```

No-sudo / locked-down machines: grab the release tarball for your target from the [`envx-project/cli`](https://github.com/envx-project/cli/releases) GitHub releases (e.g. `envx-2.13.0-aarch64-apple-darwin.tar.gz`), extract, and drop the `envx` binary somewhere on `PATH` such as `~/.local/bin`. Self-update later with `envx update` (re-runs the install script; fails on Windows).

Confirm: `envx --version` → `envx 2.13.0`.

## First-device setup — generate a key

The first thing any new machine needs is its own device key. `gen` creates it and uploads the **public** key to the server (the private key never leaves the box):

```bash
envx gen -u <handle> -p <passphrase>
```

- `-u, --username` — a handle for the key. Do NOT use a real name or anything identifying.
- `-p, --passphrase` — passphrase that encrypts the private key at rest.
- `-f, --force` — overwrite an existing key. `--no-upload` — generate locally without uploading. `--export` — print the key.
- This build's `gen` has **no `--json`**; the flags above are the whole surface.

After `gen` the passphrase is cached in the OS keyring, so later `envx run` works without prompting (see [Passphrase & non-interactive use](#passphrase--non-interactive-use)).

The device now has a key but **no project access yet** — join one below.

## Join an existing project on a new device (the core flow)

This is the money path: a machine that just ran `gen` needs into a project that another machine already belongs to.

**On an existing member** (already authed, already in the project):

```bash
envx invite create --project-id <project-id>
```

It prints an `envx invite accept <code>` line. Hand that line to the new device.

**On the new device:**

```bash
envx invite accept <code>
```

This joins the project and re-encrypts its variables to the new device's key. Confirm with `envx list-projects`.

**Direct alternative** (skip the invite round-trip — the member grants access straight to a known device/user id):

```bash
envx project add-user <USER_ID> -p <project-id>
```

Get a device's user id from `envx whoami` (the `uuid`) on that device, or `envx project list-users -p <project-id>` to see current members.

## Everyday use

Once a device is in a project, run commands with the decrypted variables injected into the environment:

```bash
envx run -p <project-id> -- <cmd> [args...]      # e.g. envx run -p <id> -- cargo run
```

- The `--` separates envx flags from the command. Everything after it runs as a child process with the project's vars added to its environment; the child's stdout/stderr pass through untouched (great for piping).
- `-e KEY=VALUE` (repeatable) overrides or adds a var for this run only.

To avoid repeating `--project-id`, **link** the current directory to a project — afterwards `run`, `variables`, `set`, etc. default to it:

```bash
envx link -p <project-id>     # or bare `envx link` for an interactive picker; -f to relink
envx run -- <cmd>             # project inferred from the linked dir
envx unlink                   # detach the cwd
```

Other everyday commands:

```bash
envx variables            # pretty table of this project's vars (--kv, --json, --filter <regex>, -a/--all)
envx shell                # subshell with the vars exported; `exit` to leave
envx set KEY=VALUE        # set/overwrite a var (interactive prompt if value omitted)
envx unset -v KEY         # delete a var (-a to clear all)
envx get variable -k KEY  # read a single var's value
```

`link`/`run`/`set` mostly succeed **silently** — no output on success is normal, so check the exit code rather than waiting for a confirmation line.

## Managing project membership

```bash
envx project new -n <name>              # create a project (--no-link to skip linking cwd, --no-name for unnamed)
envx project list-users -p <id>         # who can read this project (-a for full info, --json)
envx project add-user <USER_ID> -p <id> # grant a device/user access (re-encrypts vars to their key)
envx project remove-user -u <USER_ID> -p <id>
envx project rename -n <new-name> -p <id>
envx project info -p <id>               # project metadata
envx project delete <id>
envx list-projects                      # projects this device can read
```

Key/identity plumbing, when you need it:

```bash
envx whoami                 # this device's key fingerprint + uuid (--json)
envx list-keys              # keys known to this config (-f for full fingerprints)
envx export -k <fpr>        # export a public key (-s for the secret key)
envx import pubkey <path>   # import an ascii-armored public key
envx upload                 # upload this device's key if it isn't in the DB yet
envx auth                   # test server auth (exit 0 = authenticated)
```

## Passphrase & non-interactive use

The device's private key is passphrase-protected, and `envx run` must decrypt with it. Three ways to keep that non-interactive, in order of preference:

1. **OS keyring (default, best).** After `gen` the passphrase is cached in the system keyring, so `run` just works. Tune lifetime with `envx config set keyring-expiry --days <n>` (`0` = never expires). Inspect/clear the cached passphrase with the interactive `envx keyring view` / `envx keyring clear`.

2. **`password-command` (recommended for CI / secret managers).** Point envx at a command that prints the passphrase to stdout. The argv is **comma-separated** (or a JSON array), and the setter **runs the command once to test it before saving**:

   ```bash
   envx config set password-command 'printf,mypassphrase'
   envx config set password-command 'op,read,op://Vault/envx/password'   # 1Password
   envx config set password-command -p                                   # print the current command
   envx config set password-command                                      # (no arg) removes it
   ```

   If the command needs interactive approval (e.g. `op` biometric prompt), set it from an interactive terminal so the test invocation can succeed.

3. **`unsafe-password` (last resort).** Stores the passphrase in **plaintext** in `config.json` — insecure; there's a confirmation prompt.

   ```bash
   envx config set unsafe-password -p <passphrase>
   envx config set unsafe-password -p ''    # unset
   ```

## Inspecting state

There is **no `envx config get`** in this build (`config` only exposes `set` and `migrate`). Inspect via the top-level `get`, or read the file:

```bash
envx get config --json          # full config as JSON  (--keys-only for just the key table)
envx get project --json -p <id> # a project's variables as JSON
cat ~/.config/envx/config.json  # raw: sdk_url, keys[], projects[], primary_key_command, primary_key_password, settings
```

## Troubleshooting

- **`unauthorized` / can't read a project** → this device's key isn't a recipient. From an existing member, run `envx invite create -p <id>` and `envx invite accept <code>` here, or `envx project add-user <this-uuid> -p <id>`. Check membership with `envx project list-users -p <id>`.
- **`envx run` prompts for a passphrase** → the keyring cache expired or was never populated (common on a fresh CI runner). Set a `password-command` (option 2 above) or raise `keyring-expiry`. Re-running `envx gen` on this device also re-primes the keyring.
- **Command "does nothing"** → `link`/`run`/`set` are quiet on success. Rely on exit codes; use `envx variables` or `envx list-projects` to confirm state changed.
- **`envx auth` dumps a wall of PGP text** → that's expected (it emits the auth token). Exit code 0 means authenticated; use `envx whoami` for a human-readable check.
- **Wrong server** → confirm `sdk_url` in `~/.config/envx/config.json` (default `https://api.envx.sh`).

## Command reference (this build, v2.13.0)

| Task | Command |
|---|---|
| Generate device key | `envx gen -u <handle> -p <pass>` |
| Create an invite | `envx invite create -p <project-id>` |
| Accept an invite | `envx invite accept <code>` |
| Grant access directly | `envx project add-user <uuid> -p <id>` |
| Run with secrets | `envx run -p <id> -- <cmd>` |
| Link cwd to project | `envx link -p <id>` / `envx unlink` |
| List / read vars | `envx variables` / `envx get variable -k KEY` |
| Set / unset var | `envx set KEY=VALUE` / `envx unset -v KEY` |
| Subshell with vars | `envx shell` |
| Projects / members | `envx list-projects` / `envx project list-users -p <id>` |
| Identity | `envx whoami` / `envx list-keys` / `envx auth` |
| Non-interactive pass | `envx config set password-command '<argv,comma,sep>'` |
| Keyring lifetime | `envx config set keyring-expiry --days <n>` |
| Inspect config | `envx get config --json` |
| Self-update | `envx update` |

Run `envx <command> --help` to confirm flags before relying on them — this reference tracks build **2.13.0**, and the surface can drift between releases.
