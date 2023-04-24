pub mod handlers {
    pub mod analyses;
    pub mod bags;
    pub mod common;
    pub mod config;
    pub mod otel;
    pub mod preferences;
    pub mod searches;
    pub mod sessions;
    pub mod tokens;
}

pub mod db {
    pub mod analyses;
    pub mod bags;
    pub mod preferences;
    pub mod searches;
    pub mod sessions;
    pub mod users;
}

pub mod app_state;
pub mod auth;
pub mod errors;
pub mod signals;
