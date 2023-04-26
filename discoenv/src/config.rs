use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
pub struct Cli {
    /// Whether to include the user domain if it's missing from requests.
    #[arg(short, long, default_value_t = true)]
    pub append_user_domain: bool,

    /// Path to the configuration file.
    #[arg(short, long, default_value_t = String::from("/etc/cyverse/de/configs/service.yml"))]
    pub config: String,

    /// Path to the TLS cert PEM file.
    #[arg(long, default_value_t = String::from("/etc/cyverse/de/tls/cert.pem"))]
    pub cert: String,

    /// Path to the TLS key PEM file.
    #[arg(long, default_value_t = String::from("/etc/cyverse/de/tls/key.pem"))]
    pub key: String,

    /// The listen port.
    #[arg(short, long, default_value_t = 60000)]
    pub port: u16,

    /// Whether to use TLS.
    #[arg(short, long)]
    pub no_tls: bool,
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
