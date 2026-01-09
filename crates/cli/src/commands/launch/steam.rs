use std::{collections::HashMap, fs::File, io, ops::Deref, path::Path};

use serde::Deserialize;
use serde_repr::Deserialize_repr;
use steamlocate::SteamDir;

pub fn steamdir() -> Option<SteamDir> {
    SteamDir::locate_multiple()
        .ok()?
        .into_iter()
        .find(|dir| dir.library_paths().is_ok())
}

#[derive(Deserialize, Debug)]
pub struct SteamUsers(HashMap<u64, SteamUserData>);

impl Deref for SteamUsers {
    type Target = HashMap<u64, SteamUserData>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct SteamUser<'a> {
    steam_id: u64,
    data: &'a SteamUserData,
}

#[repr(u8)]
#[derive(Clone, Copy, Deserialize_repr, Debug, PartialEq, Eq)]
pub enum SteamInputConfig {
    ForceOff = 0,
    Default = 1,
    ForceOn = 2,
}

#[derive(Deserialize, Debug)]
pub struct SteamUserAppConfig {
    #[serde(rename = "UseSteamControllerConfig")]
    pub use_steam_controller_config: Option<SteamInputConfig>,
}

#[derive(Deserialize, Debug)]
pub struct SteamUserConfig {
    pub apps: HashMap<u32, SteamUserAppConfig>,
}

impl SteamUserConfig {
    pub fn open(steam: impl AsRef<Path>, user: SteamUser) -> io::Result<Self> {
        let steamid3 = user.steamid3();
        let config_path = steam
            .as_ref()
            .join(format!("userdata/{steamid3}/config/localconfig.vdf"));

        let config_file = File::open(config_path)?;
        let config = keyvalues_serde::from_reader(config_file).map_err(io::Error::other)?;

        Ok(config)
    }
}

impl<'a> SteamUser<'a> {
    pub fn steamid3(&self) -> u32 {
        (self.steam_id & 0xFFFFFFFF) as u32
    }
}

impl SteamUsers {
    pub fn active(&self) -> Option<SteamUser<'_>> {
        self.0.iter().find_map(|(id, data)| {
            data.most_recent.then_some(SteamUser {
                steam_id: *id,
                data,
            })
        })
    }

    pub fn open(steam: impl AsRef<Path>) -> io::Result<Self> {
        let config_path = steam.as_ref().join("config/loginusers.vdf");
        let config_file = File::open(config_path)?;
        let config = keyvalues_serde::from_reader(config_file).map_err(io::Error::other)?;

        Ok(config)
    }
}

#[derive(Deserialize, Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct SteamUserData {
    #[serde(rename = "MostRecent")]
    most_recent: bool,

    #[serde(rename = "PersonaName")]
    persona_name: String,

    #[serde(rename = "Timestamp")]
    timestamp: u32,
}

#[cfg(test)]
mod test {
    use crate::commands::launch::steam::{SteamInputConfig, SteamUserConfig, SteamUsers};

    #[test]
    fn deserialize_config() {
        let contents = r#"
"UserLocalConfigStore"
{
	"system"
	{
		"PushToTalkKey"		"0"
		"NetworkingAllowShareIP"		"2"
		"OverlayLogCleanupDone"		"1"
	}

	"apps"
	{
		"2357570"
		{
			"OverlayAppEnable"		"1"
		}
		"1030300"
		{
			"UseSteamControllerConfig"		"2"
			"SteamControllerRumble"		"-1"
			"SteamControllerRumbleIntensity"		"320"
		}
	}
	
	"ControllerTypesUsed"		"controller_ps4,controller_generic,"
	"GameRecording"
	{
		"BackgroundRecordMode"		"1"
	}

}
        "#;
        let config: SteamUserConfig = keyvalues_serde::from_str(contents).unwrap();
        let app_config = &config.apps[&1030300];

        assert_eq!(
            Some(SteamInputConfig::ForceOn),
            app_config.use_steam_controller_config
        );
    }

    #[test]
    fn deserialize_users() {
        let contents = r#"
"users"
{
        "76561198216541595"
        {
                "AccountName"           "garyttierney"
                "PersonaName"           "sfix"
                "RememberPassword"              "1"
                "WantsOfflineMode"              "0"
                "SkipOfflineModeWarning"                "0"
                "AllowAutoLogin"                "1"
                "MostRecent"            "1"
                "Timestamp"             "1767810156"
        }
}"#;

        let manifest: SteamUsers = keyvalues_serde::from_str(contents).unwrap();
        let user = manifest.get(&76561198216541595).unwrap();

        assert_eq!(true, user.most_recent);
        assert_eq!("sfix", user.persona_name);
    }
}
