//! One home-directory boundary: production uses the OS home, unit tests never do.
use std::path::PathBuf;

#[cfg(not(test))]
pub fn home_dir() -> Option<PathBuf> {
    home::home_dir()
}

#[cfg(test)]
thread_local! {
    // libtest gives each test its own thread, so parallel tests neither share
    // profiles nor touch the invoking user's HOME/USERPROFILE.
    static TEST_HOME: tempfile::TempDir = tempfile::tempdir().expect("create isolated test home");
    static HOME_OVERRIDE: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub fn home_dir() -> Option<PathBuf> {
    Some(HOME_OVERRIDE.with(|value| {
        value
            .borrow()
            .clone()
            .unwrap_or_else(|| TEST_HOME.with(|home| home.path().to_owned()))
    }))
}

/// Explicitly share only a synthetic profile between concurrency-test workers.
/// No environment variable is automatically trusted as a home override.
#[cfg(test)]
pub fn with_test_home<T>(
    path: &std::path::Path,
    action: impl FnOnce() -> T,
) -> T {
    struct RestoreHome(Option<PathBuf>);
    impl Drop for RestoreHome {
        fn drop(&mut self) {
            HOME_OVERRIDE.with(|value| *value.borrow_mut() = self.0.take());
        }
    }
    let _restore = RestoreHome(
        HOME_OVERRIDE.with(|value| value.replace(Some(path.to_owned()))),
    );
    action()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_home_is_stable_and_separate_for_each_thread() {
        let this = home_dir().unwrap();
        assert_eq!(home_dir().unwrap(), this);
        let other = std::thread::spawn(|| {
            let home = home_dir().unwrap();
            std::fs::write(home.join("other-thread"), b"test").unwrap();
            home
        })
        .join()
        .unwrap();
        assert_ne!(this, other);
        assert!(!this.join("other-thread").exists());
        assert!(
            !other.exists(),
            "thread-owned temp directory is removed on exit"
        );
    }

    #[test]
    fn explicit_worker_override_is_restored() {
        let default = home_dir().unwrap();
        let shared = tempfile::tempdir().unwrap();
        with_test_home(shared.path(), || {
            assert_eq!(home_dir().unwrap(), shared.path())
        });
        assert_eq!(home_dir().unwrap(), default);
    }
}
