use serde::Deserialize;

use std::path::{Path, PathBuf};

type Result<Type> = std::result::Result<Type, Box<dyn std::error::Error>>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LauncherInstalled {
    installation_list: Vec<Installation>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Installation {
    install_location: String,
    app_name: String
}

pub fn find_rocketleague_dir() -> Result<String> {
    let program_data = std::env::var("PROGRAMDATA")?;
    let launcher_installed_path: PathBuf = [&program_data, "Epic", "UnrealEngineLauncher", "LauncherInstalled.dat"].into_iter().collect();

    if !Path::exists(&launcher_installed_path) {
        panic!("couldn't find LauncherInstalled.dat");
    }

    let content = std::fs::read_to_string(launcher_installed_path)?;
    log::info!("{}", &content);

    let launcher_installed: LauncherInstalled = serde_json::from_str(&content)?;
    if let Some(installation) = launcher_installed.installation_list.into_iter().find(|x| x.app_name == "Sugar") {
        return Ok(installation.install_location)
    }

    panic!("couldn't find rocket league installation folder")
}