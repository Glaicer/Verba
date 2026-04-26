use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use uuid::Uuid;

use crate::{
    config::schema::AppConfig,
    error::{Result, VerbaError},
};

#[derive(Clone, Debug)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn default_path() -> Result<Self> {
        let config_dir = dirs::config_dir().ok_or_else(|| {
            VerbaError::Config("could not resolve XDG config directory".to_string())
        })?;
        Ok(Self::new(config_dir.join("verba").join("config.toml")))
    }

    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_or_create(&self) -> Result<AppConfig> {
        if !self.path.exists() {
            let config = AppConfig::default();
            self.save(&config)?;
            return Ok(config);
        }

        let text = fs::read_to_string(&self.path)?;
        let mut config: AppConfig = toml::from_str(&text)?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self, config: &AppConfig) -> Result<()> {
        let mut config = config.clone();
        config.validate()?;

        let parent = self
            .path
            .parent()
            .ok_or_else(|| VerbaError::MissingConfigParent(self.path.clone()))?;
        fs::create_dir_all(parent)?;

        let temp_path = parent.join(format!(".config.toml.{}.tmp", Uuid::new_v4()));
        let write_result = write_temp_config(&temp_path, &config);
        if let Err(err) = write_result {
            let _ = fs::remove_file(&temp_path);
            return Err(err);
        }

        fs::rename(&temp_path, &self.path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.path, fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    }
}

fn write_temp_config(temp_path: &Path, config: &AppConfig) -> Result<()> {
    let text = toml::to_string_pretty(config)?;
    let mut file = File::create(temp_path)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    Ok(())
}
