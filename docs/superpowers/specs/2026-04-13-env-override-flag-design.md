# Design: `-e` Environment Variable Override Flag

## Summary

Add a `-e` / `--env` flag to `envx run` and `envx shell` that allows users to override or add environment variables at invocation time. Overrides take highest precedence, above both hardcoded values and API-fetched variables.

## Motivation

Some envx-managed variables have sensible defaults (e.g., `BASH_TOOL_MODE=off`) but occasionally need temporary overrides for testing. Rather than modifying the stored variable in envx, users should be able to override at the command line without touching the persisted state.

## Usage

```
envx run -e BASH_TOOL_MODE=docker -e DEBUG=1 -- my-command
envx shell -e BASH_TOOL_MODE=aws
```

The flag is repeatable. Each instance sets one variable.

## Design

### Scope

Only `run` and `shell` commands. No changes to `variables`, `get`, `set`, `unset`, or any other command.

### Approach

Add a clap field to both `run::Args` and `shell::Args`:

```rust
/// Override or add an environment variable (KEY=VALUE), repeatable
#[arg(short = 'e', long = "env")]
env_override: Vec<String>,
```

### Precedence (lowest to highest)

1. `IN_ENVX_SHELL=true` (hardcoded)
2. API-fetched variables
3. `-e` overrides

### Parsing

After the API fetch loop inserts variables into `all_variables`, iterate over `env_override`:

- Split each string on the first `=`
- Insert the key-value pair into `all_variables` (overwrites existing keys)
- If a string contains no `=`, bail with: `Invalid override format: "<value>". Expected KEY=VALUE`

### Behavior

- Keys that don't exist in the fetched variables are created (set-or-create semantics)
- Keys that do exist are overwritten
- Empty values are valid (e.g., `-e FOO=`)

## Files Modified

- `src/commands/run.rs` — add field + parsing loop (~10 lines)
- `src/commands/shell.rs` — add field + parsing loop (~10 lines)

No new modules, no macro changes, no changes to main.rs.
