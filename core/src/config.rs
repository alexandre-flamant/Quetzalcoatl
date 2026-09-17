use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "lowercase", deny_unknown_fields)]
pub enum AuthConfig {
    /// Key-based auth: recommended for the tool's owner's own repeated/
    /// scripted use.
    Key {
        private_key_path: PathBuf,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        passphrase: Option<String>,
    },
    /// Password auth: kept fully first-class (not a secondary fallback),
    /// since the planned GUI's whole point is letting other users point
    /// this tool at their own tablet with just an IP and the password
    /// shown on their device's Settings screen.
    Password { password: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    pub auth: AuthConfig,
    /// Preferred cipher names (OpenSSH-style identifiers, e.g.
    /// `aes128-gcm@openssh.com`), preferred first. The tablet's weak
    /// Cortex-A7 CPU doing SSH crypto -- not the USB link -- is the actual
    /// transfer-speed bottleneck, so this lets a lighter cipher be preferred
    /// over whatever gets negotiated by default.
    #[serde(default)]
    pub ciphers: Vec<String>,
    /// Default target folder path for `import`, `//`-separated. Empty means
    /// the tablet's root -- not any specific folder.
    #[serde(default)]
    pub default_folder: String,
}

fn default_port() -> u16 {
    22
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|e| ConfigError::Read {
            path: path.to_path_buf(),
            source: e,
        })?;
        let mut config: Config = toml::from_str(&text).map_err(|e| ConfigError::Parse {
            path: path.to_path_buf(),
            source: e,
        })?;
        config.apply_env_overrides();
        Ok(config)
    }

    /// Applies `QUETZALCOATL_*` environment variable overrides on top of
    /// whatever was loaded from the config file, for one-off overrides
    /// without editing it.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("QUETZALCOATL_HOST") {
            self.host = v;
        }
        if let Ok(v) = std::env::var("QUETZALCOATL_PORT") {
            if let Ok(port) = v.parse() {
                self.port = port;
            }
        }
        if let Ok(v) = std::env::var("QUETZALCOATL_USERNAME") {
            self.username = v;
        }
        if let Ok(password) = std::env::var("QUETZALCOATL_PASSWORD") {
            self.auth = AuthConfig::Password { password };
        }
        if let Ok(private_key_path) = std::env::var("QUETZALCOATL_PRIVATE_KEY_PATH") {
            self.auth = AuthConfig::Key {
                private_key_path: PathBuf::from(private_key_path),
                passphrase: None,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Config::load` reads `QUETZALCOATL_*` env vars as a process-global
    /// side effect, so any test that touches them (directly, or indirectly
    /// via a prior test still holding one set) must serialize against every
    /// other test in this module -- cargo runs tests in parallel threads
    /// within one process, and env vars are not thread-local.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn loads_password_config() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile_dir();
        let path = dir.join("quetzalcoatl.toml");
        std::fs::write(
            &path,
            r#"
            host = "10.11.99.1"
            username = "root"
            [auth]
            method = "password"
            password = "hunter2"
            "#,
        )
        .unwrap();

        let config = Config::load(&path).unwrap();
        assert_eq!(config.host, "10.11.99.1");
        assert_eq!(config.port, 22);
        assert!(matches!(config.auth, AuthConfig::Password { .. }));
    }

    #[test]
    fn loads_key_config_with_ciphers_and_default_folder() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile_dir();
        let path = dir.join("quetzalcoatl.toml");
        std::fs::write(
            &path,
            r#"
            host = "10.11.99.1"
            port = 2222
            username = "root"
            ciphers = ["aes128-gcm@openssh.com"]
            default_folder = "Livres//mangas"
            [auth]
            method = "key"
            private_key_path = "~/.ssh/id_rsa"
            "#,
        )
        .unwrap();

        let config = Config::load(&path).unwrap();
        assert_eq!(config.port, 2222);
        assert_eq!(config.default_folder, "Livres//mangas");
        assert!(matches!(config.auth, AuthConfig::Key { .. }));
    }

    #[test]
    fn missing_required_field_is_rejected() {
        let dir = tempfile_dir();
        let path = dir.join("quetzalcoatl.toml");
        std::fs::write(&path, "port = 22\n").unwrap();
        assert!(Config::load(&path).is_err());
    }

    #[test]
    fn env_override_takes_precedence() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile_dir();
        let path = dir.join("quetzalcoatl.toml");
        std::fs::write(
            &path,
            r#"
            host = "10.11.99.1"
            username = "root"
            [auth]
            method = "password"
            password = "hunter2"
            "#,
        )
        .unwrap();

        // SAFETY: mutating process env vars is inherently racy against other
        // threads; `ENV_LOCK` above serializes this against every other test
        // in this module that reads them via `Config::load`.
        unsafe {
            std::env::set_var("QUETZALCOATL_HOST", "192.168.1.50");
        }
        let config = Config::load(&path).unwrap();
        unsafe {
            std::env::remove_var("QUETZALCOATL_HOST");
        }

        assert_eq!(config.host, "192.168.1.50");
    }

    fn tempfile_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "quetzalcoatl-core-tests-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
