use serde::{Deserialize, Serialize};
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
    exp: Option<u64>,
    iat: Option<u64>,
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

impl From<TokenIntrospectionResult> for UserInfo {
    fn from(from: TokenIntrospectionResult) -> Self {
        UserInfo {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authenticator {
    base_url: Url,
    introspection_url: Url,
    token_url: Url,
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
            base_url,
            token_url,
            introspection_url,
            client_id: client_id.into(),
            client_secret: client_secret.into(),
        })
    }

    pub fn token_url(&self) -> String {
        self.token_url.to_string()
    }

    pub async fn validate_token(&self, token: &str) -> Result<UserInfo, DiscoError> {
        let client = reqwest::Client::new();
        let result = client
            .post(self.introspection_url.as_str())
            .form(&TokenIntrospectionRequest {
                token: token.into(),
                client_id: self.client_id.clone(),
                client_secret: self.client_secret.clone(),
            })
            .send()
            .await?
            .error_for_status()?
            .json::<TokenIntrospectionResult>()
            .await?;

        Ok(result.into())
    }

    pub async fn get_token(&self, username: &str, password: &str) -> Result<Token, DiscoError> {
        let client = reqwest::Client::new();
        let resp = client
            .post(self.token_url.as_str())
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
