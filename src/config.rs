use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AsideConfig {
    /// Absolute path of the main repo (work-tree)
    pub work_tree: String,
    /// Remote of the aside repo
    pub remote: String,
    /// Absolute path of the aside bare repo
    pub bare_path: String,
    /// Tracked files/directories
    pub tracked: Vec<String>,
    /// Aside repo branch (default: "main")
    #[serde(default = "default_branch")]
    pub branch: String,
}

fn default_branch() -> String {
    "main".to_string()
}

/// Returns the ~/.git-asides/ directory
pub fn asides_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let dir = home.join(".git-asides");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Generates a unique ID based on the main remote URL
pub fn project_id(origin_url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(origin_url.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8]) // 16 hex chars
}

/// Returns the config file path for the current project
pub fn config_path_for(project_id: &str) -> Result<PathBuf> {
    Ok(asides_dir()?.join(project_id).join("config.toml"))
}

/// Loads the aside config for the current repo
pub fn load(main_remote: &str) -> Result<AsideConfig> {
    let id = project_id(main_remote);
    let path = config_path_for(&id)?;
    let content = std::fs::read_to_string(&path)
        .with_context(|| "Aside repo not initialized. Run: git aside init <remote> <files>".to_string())?;
    let config: AsideConfig = toml::from_str(&content)
        .context("Aside config is corrupted")?;
    Ok(config)
}

/// Saves the aside config
pub fn save(config: &AsideConfig, project_id: &str) -> Result<()> {
    let path = config_path_for(project_id)?;
    std::fs::create_dir_all(path.parent().unwrap())?;
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Removes the aside config
pub fn remove(project_id: &str) -> Result<()> {
    let dir = asides_dir()?.join(project_id);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(())
}
