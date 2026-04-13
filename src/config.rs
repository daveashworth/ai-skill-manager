use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillState {
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetsConfig {
    pub dirs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub targets: TargetsConfig,
    #[serde(default)]
    pub skills: BTreeMap<String, SkillState>,
}

impl Config {
    pub fn config_dir() -> PathBuf {
        dirs::home_dir()
            .expect("Could not determine home directory")
            .join(".config")
            .join("skillmanager")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn central_store() -> PathBuf {
        Self::config_dir().join("skills")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            let content = fs::read_to_string(&path).expect("Failed to read config");
            toml::from_str(&content).expect("Failed to parse config")
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir).expect("Failed to create config directory");
        let content = toml::to_string_pretty(self).expect("Failed to serialize config");
        fs::write(Self::config_path(), content).expect("Failed to write config");
    }

    pub fn expanded_target_dirs(&self) -> Vec<PathBuf> {
        let home = dirs::home_dir().expect("Could not determine home directory");
        self.targets
            .dirs
            .iter()
            .map(|d| {
                if d.starts_with("~/") {
                    home.join(&d[2..])
                } else {
                    PathBuf::from(d)
                }
            })
            .collect()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            targets: TargetsConfig {
                dirs: vec![
                    "~/.claude/skills".to_string(),
                    "~/.config/agents/skills".to_string(),
                    "~/.agents/skills".to_string(),
                    "~/.config/amp/skills".to_string(),
                    "~/.cursor/skills".to_string(),
                    "~/.codex/skills".to_string(),
                    "~/.codeium/windsurf/skills".to_string(),
                ],
            },
            skills: BTreeMap::new(),
        }
    }
}
