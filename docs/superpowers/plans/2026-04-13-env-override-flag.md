# `-e` Environment Override Flag Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `-e`/`--env` flag to `envx run` and `envx shell` that overrides or injects environment variables at invocation time.

**Architecture:** Add a `parse_env_overrides` function in a new `src/utils/env_override.rs` module. Add `#[arg(short = 'e', long = "env")] env_override: Vec<String>` to both `run::Args` and `shell::Args`. After fetching API variables, apply overrides (highest precedence).

**Tech Stack:** Rust, clap v4.5 (derive macros), anyhow

**Spec:** `docs/superpowers/specs/2026-04-13-env-override-flag-design.md`

---

## File Map

- **Create:** `src/utils/env_override.rs` — parsing function + unit tests
- **Modify:** `src/utils/mod.rs` — register new module
- **Modify:** `src/commands/run.rs` — add `-e` arg field, apply overrides after fetch
- **Modify:** `src/commands/shell.rs` — add `-e` arg field, apply overrides after fetch

---

### Task 1: Parse function with tests

**Files:**
- Create: `src/utils/env_override.rs`
- Modify: `src/utils/mod.rs`

- [ ] **Step 1: Create `src/utils/env_override.rs` with failing test**

```rust
use std::collections::BTreeMap;

use anyhow::{bail, Result};

/// Parse a list of "KEY=VALUE" strings and apply them as overrides to an
/// existing variable map. Overrides win unconditionally.
pub fn apply_env_overrides(
    variables: &mut BTreeMap<String, String>,
    overrides: Vec<String>,
) -> Result<()> {
    for entry in overrides {
        let Some((key, value)) = entry.split_once('=') else {
            bail!("Invalid override format: \"{entry}\". Expected KEY=VALUE");
        };
        variables.insert(key.to_owned(), value.to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_override_existing_key() {
        let mut vars = BTreeMap::from([("FOO".into(), "old".into())]);
        apply_env_overrides(&mut vars, vec!["FOO=new".into()]).unwrap();
        assert_eq!(vars["FOO"], "new");
    }

    #[test]
    fn test_add_new_key() {
        let mut vars = BTreeMap::new();
        apply_env_overrides(&mut vars, vec!["NEW_KEY=val".into()]).unwrap();
        assert_eq!(vars["NEW_KEY"], "val");
    }

    #[test]
    fn test_empty_value_is_valid() {
        let mut vars = BTreeMap::new();
        apply_env_overrides(&mut vars, vec!["EMPTY=".into()]).unwrap();
        assert_eq!(vars["EMPTY"], "");
    }

    #[test]
    fn test_value_containing_equals() {
        let mut vars = BTreeMap::new();
        apply_env_overrides(&mut vars, vec!["DSN=postgres://host/db?opt=1".into()])
            .unwrap();
        assert_eq!(vars["DSN"], "postgres://host/db?opt=1");
    }

    #[test]
    fn test_no_equals_is_error() {
        let mut vars = BTreeMap::new();
        let result = apply_env_overrides(&mut vars, vec!["INVALID".into()]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid override format")
        );
    }

    #[test]
    fn test_multiple_overrides() {
        let mut vars = BTreeMap::from([("A".into(), "1".into())]);
        apply_env_overrides(
            &mut vars,
            vec!["A=2".into(), "B=3".into(), "C=4".into()],
        )
        .unwrap();
        assert_eq!(vars["A"], "2");
        assert_eq!(vars["B"], "3");
        assert_eq!(vars["C"], "4");
    }

    #[test]
    fn test_empty_overrides_is_noop() {
        let mut vars = BTreeMap::from([("X".into(), "y".into())]);
        apply_env_overrides(&mut vars, vec![]).unwrap();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars["X"], "y");
    }
}
```

- [ ] **Step 2: Register module in `src/utils/mod.rs`**

Add after the `pub mod config;` line:

```rust
pub mod env_override;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test env_override`

Expected: 7 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/utils/env_override.rs src/utils/mod.rs
git commit -m "[agent] feat: add env override parser with tests"
```

---

### Task 2: Wire `-e` into `run` command

**Files:**
- Modify: `src/commands/run.rs:10-17` (Args struct)
- Modify: `src/commands/run.rs:33-37` (after variable fetch)

- [ ] **Step 1: Add import and arg field to `run::Args`**

In `src/commands/run.rs`, add the import at the top (after existing `use` lines):

```rust
use crate::utils::env_override::apply_env_overrides;
```

Add the field to the `Args` struct, after the `project_id` field and before `args`:

```rust
    /// Override or add environment variables (KEY=VALUE), repeatable
    #[arg(short = 'e', long = "env")]
    env_override: Vec<String>,
```

- [ ] **Step 2: Apply overrides after the variable fetch loop**

In the `command` function, after the `for variable in variables` loop (after line 37), add:

```rust
    apply_env_overrides(&mut all_variables, args.env_override)?;
```

Note: `args` is moved into `args.args` later (line 45), so we need to extract `env_override` before that. Since `args.env_override` is consumed here and `args.args` is used later, this works because `Vec<String>` fields are independent — partial moves are fine in Rust since `args` is not used as a whole after this point.

Actually, `args` IS used as a whole on line 45 (`args.args`). Since we're only accessing individual fields, Rust handles partial moves correctly here — `args.env_override` is moved on the override line, `args.args` is moved on line 45. This is fine.

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`

Expected: Compiles without errors.

- [ ] **Step 4: Commit**

```bash
git add src/commands/run.rs
git commit -m "[agent] feat: add -e env override flag to run command"
```

---

### Task 3: Wire `-e` into `shell` command

**Files:**
- Modify: `src/commands/shell.rs:31-38` (Args struct)
- Modify: `src/commands/shell.rs:53-57` (after variable fetch)

- [ ] **Step 1: Add import and arg field to `shell::Args`**

In `src/commands/shell.rs`, add the import at the top (after existing `use` lines):

```rust
use crate::utils::env_override::apply_env_overrides;
```

Add the field to the `Args` struct, after `silent` and before the struct closing brace:

```rust
    /// Override or add environment variables (KEY=VALUE), repeatable
    #[arg(short = 'e', long = "env")]
    env_override: Vec<String>,
```

- [ ] **Step 2: Apply overrides after the variable fetch loop**

In the `command` function, after the `for variable in variables` loop (after line 57), add:

```rust
    apply_env_overrides(&mut all_variables, args.env_override)?;
```

Same partial-move situation as `run` — `args.silent` is read on line 78, but it's a `bool` (Copy), so no ownership issue.

- [ ] **Step 3: Verify it compiles and all tests pass**

Run: `cargo build && cargo test env_override`

Expected: Compiles, 7 tests pass.

- [ ] **Step 4: Run formatter and linter**

Run: `cargo fmt && cargo clippy`

Expected: No warnings or errors.

- [ ] **Step 5: Commit**

```bash
git add src/commands/shell.rs
git commit -m "[agent] feat: add -e env override flag to shell command"
```
