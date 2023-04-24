use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
pub struct Cli {
    /// Whether to include the user domain if it's missing from requests.
    #[arg(short, long, default_value_t = true)]
    pub append_user_domain: bool,

    /// The config file to read settings from.
    #[arg(short, long, default_value_t = String::from("/etc/cyverse/de/configs/service.yml"))]
    pub config: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigDB {
    pub uri: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigUsers {
    pub domain: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigEntitlements {
    pub admin: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigOauth {
    pub uri: String,
    pub realm: String,
    pub client_id: String,
    pub client_secret: String,
    pub entitlements: Option<ConfigEntitlements>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub db: ConfigDB,
    pub users: ConfigUsers,
    pub oauth: Option<ConfigOauth>,
}
