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
        apply_env_overrides(
            &mut vars,
            vec!["DSN=postgres://host/db?opt=1".into()],
        )
        .unwrap();
        assert_eq!(vars["DSN"], "postgres://host/db?opt=1");
    }

    #[test]
    fn test_no_equals_is_error() {
        let mut vars = BTreeMap::new();
        let result = apply_env_overrides(&mut vars, vec!["INVALID".into()]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid override format"));
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
