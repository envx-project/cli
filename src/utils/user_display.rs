use super::{
    config::Config,
    messaging::{safe, Pin},
    state::StateStore,
};
use anyhow::Result;
use std::collections::HashMap;

/// Presentation only: aliases never replace IDs used for authorization or trust.
pub struct UserDisplay {
    aliases: HashMap<String, String>,
}
impl UserDisplay {
    pub fn new(config: &Config) -> Result<Self> {
        Self::from_state(&StateStore::open(config)?)
    }
    pub fn from_state(state: &StateStore) -> Result<Self> {
        Ok(Self {
            aliases: state
                .list::<Pin>("friend")?
                .into_iter()
                .filter_map(|(_, pin)| {
                    pin.alias.map(|alias| (pin.user_id, alias))
                })
                .collect(),
        })
    }
    pub fn name(&self, id: &str, username: &str) -> String {
        match self.aliases.get(id) {
            Some(alias) if alias != username && username != id => {
                format!("{} (@{})", safe(alias), safe(username))
            }
            Some(alias) => safe(alias),
            None => safe(username),
        }
    }
    pub fn row(&self, id: &str, username: &str, verbose: bool) -> String {
        let name = self.name(id, username);
        let id = safe(id);
        format!(
            "{} · {}",
            name,
            if verbose {
                id
            } else {
                id.chars().take(8).collect()
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aliases_are_scoped_and_do_not_change_identity() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.sdk_url = Some("https://one.example".into());
        let state = StateStore::open_at(dir.path(), &config).unwrap();
        let id = "12345678-1111-4111-8111-111111111111";
        state
            .put(
                "friend",
                id,
                &Pin {
                    user_id: id.into(),
                    fingerprint: "pin".into(),
                    public_key: "key".into(),
                    alias: Some("Teammate".into()),
                },
            )
            .unwrap();
        let display = UserDisplay::from_state(&state).unwrap();
        assert_eq!(
            display.row(id, "account", false),
            "Teammate (@account) · 12345678"
        );
        assert!(display.row(id, "account", true).ends_with(id));
        assert_eq!(display.name(id, id), "Teammate");
        assert!(!display
            .name("unknown", "bad\x1b[31m\nname")
            .contains('\x1b'));
        config.sdk_url = Some("https://two.example".into());
        let other = StateStore::open_at(dir.path(), &config).unwrap();
        assert_eq!(
            UserDisplay::from_state(&other).unwrap().name(id, "account"),
            "account"
        );
    }
}
