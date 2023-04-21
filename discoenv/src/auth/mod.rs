pub mod middleware;

use cached::{proc_macro::cached, stores::CanExpire};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use url::{ParseError, Url};

use crate::errors::DiscoError;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Token {
    access_token: String,
    token_type: String,

    #[serde(rename = "not-before-policy")]
    not_before_policy: u64,
    session_state: String,
    scope: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct TokenRequest {
    grant_type: String,
    client_id: String,
    client_secret: String,
    username: String,
    password: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct TokenIntrospectionRequest {
    token: String,
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RealmAccess {
    roles: Option<Vec<String>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Account {
    roles: Option<Vec<String>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ResourceAccess {
    account: Option<Account>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct TokenIntrospectionResult {
    active: bool,
    exp: Option<u64>, // should be seconds since the epoch. when the token expires.
    iat: Option<u64>, // should be seconds since the epoch. when the token was granted.
    jti: Option<String>,
    iss: Option<String>,
    sub: Option<String>,
    typ: Option<String>,
    azp: Option<String>,
    session_state: Option<String>,
    preferred_username: Option<String>,
    email_verified: Option<bool>,
    acr: Option<String>,
    scope: Option<String>,
    email: Option<String>,
    name: Option<String>,
    given_name: Option<String>,
    family_name: Option<String>,

    #[serde(rename = "allowed-origins")]
    allowed_origins: Option<Vec<String>>,

    realm_access: Option<RealmAccess>,
    resource_access: Option<ResourceAccess>,
    client_id: Option<String>,
    entitlement: Option<Vec<String>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    iat: Option<u64>,
    exp: Option<u64>,
    pub active: bool,
    pub preferred_username: Option<String>,
    pub email_verified: Option<bool>,
    pub scope: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub realm_access: Option<RealmAccess>,
    pub resource_access: Option<ResourceAccess>,
    pub entitlement: Option<Vec<String>>,
}
impl CanExpire for UserInfo {
    fn is_expired(&self) -> bool {
        // Don't cache inactive token introspection results.
        // Could result in users being unable to authenticate for a day after a failure.
        if !self.active {
            return true;
        }

        // If both fields are None, then the response is uncacheable.
        if self.iat.is_none() && self.exp.is_none() {
            return true;
        }

        let n = SystemTime::now().duration_since(UNIX_EPOCH);
        if n.is_err() {
            return true;
        }

        let now_seconds = n.unwrap_or_default().as_secs(); // unwrap or 0;

        // now_seconds is 0, then something has gone wrong and the result isn't cacheable.
        if now_seconds == 0 {
            return true;
        }

        // Make sure the expiration date hasn't passed.
        if let Some(expiration) = self.exp {
            if now_seconds == 0 || (now_seconds >= expiration) {
                return true;
            }
        }

        // Make sure the token isn't more than a day old.
        if let Some(creation) = self.iat {
            if now_seconds >= (creation + 86_400) {
                return true;
            }
        }

        // If the call gets here, then the token isn't older than a day and hasn't expired.
        // It can be cached.
        false
    }
}

impl From<TokenIntrospectionResult> for UserInfo {
    fn from(from: TokenIntrospectionResult) -> Self {
        UserInfo {
            iat: from.iat,
            exp: from.exp,
            active: from.active,
            preferred_username: from.preferred_username,
            email_verified: from.email_verified,
            scope: from.scope,
            email: from.email,
            name: from.name,
            given_name: from.given_name,
            family_name: from.family_name,
            realm_access: from.realm_access,
            resource_access: from.resource_access,
            entitlement: from.entitlement,
        }
    }
}

// Takes ownership of the arguments because of the requirements
// imposed by cached.
#[cached(result = true, sync_writes = true)]
async fn check_token(
    url: String,
    token: String,
    client_id: String,
    client_secret: String,
) -> Result<TokenIntrospectionResult, reqwest::Error> {
    let client = reqwest::Client::new();
    let result = client
        .post(url)
        .form(&TokenIntrospectionRequest {
            token,
            client_id,
            client_secret,
        })
        .send()
        .await?
        .error_for_status()?
        .json::<TokenIntrospectionResult>()
        .await?;
    Ok(result)
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Authenticator {
    base_url: String,
    introspection_url: String,
    token_url: String,
    client_id: String,
    client_secret: String,
}

impl Authenticator {
    pub fn setup(
        base: &str,
        realm: &str,
        client_id: &str,
        client_secret: &str,
    ) -> Result<Self, ParseError> {
        let b = Url::parse(base)?;
        let mut base_url = b;
        base_url
            .path_segments_mut()
            .map_err(|_| ParseError::SetHostOnCannotBeABaseUrl)?
            .push("realms")
            .push(realm);

        let mut token_url = base_url.clone();
        token_url
            .path_segments_mut()
            .map_err(|_| ParseError::SetHostOnCannotBeABaseUrl)?
            .push("protocol")
            .push("openid-connect")
            .push("token");

        let mut introspection_url = token_url.clone();
        introspection_url
            .path_segments_mut()
            .map_err(|_| ParseError::SetHostOnCannotBeABaseUrl)?
            .push("introspect");

        Ok(Authenticator {
            base_url: base_url.to_string(),
            token_url: token_url.to_string(),
            introspection_url: introspection_url.to_string(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
        })
    }

    pub fn token_url(&self) -> String {
        self.token_url.to_string()
    }

    pub async fn validate_token(&self, token: &str) -> Result<UserInfo, DiscoError> {
        Ok(check_token(
            self.introspection_url.clone(),
            token.to_string(),
            self.client_id.clone(),
            self.client_secret.clone(),
        )
        .await?
        .into())
    }

    pub async fn get_token(&self, username: &str, password: &str) -> Result<Token, DiscoError> {
        let client = reqwest::Client::new();
        let resp = client
            .post(&self.token_url)
            .form(&TokenRequest {
                client_id: self.client_id.clone(),
                client_secret: self.client_secret.clone(),
                username: username.into(),
                password: password.into(),
                grant_type: "password".into(),
            })
            .send()
            .await?
            .error_for_status()?
            .json::<Token>()
            .await?;
        Ok(resp)
    }
}
