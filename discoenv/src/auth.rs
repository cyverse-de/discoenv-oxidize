use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Token {
    jwt_claims: JWTClaims,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct JWTClaims {
    uid_domain: String,
    preferred_username: String,
    email: String,
    given_name: String,
    family_name: String,
    name: String,
    entitlement: Vec<String>,
    realm_access: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct User {
    uid_domain: String,
    short_username: String,
    username: String,
    email: String,
    first_name: String,
    last_name: String,
    common_name: String,
    entitlement: Vec<String>,
}

impl User {
    pub fn is_service_account(&self) -> bool {
        is_service_account(&self.short_username)
    }
}

impl From<JWTClaims> for User {
    fn from(claim: JWTClaims) -> Self {
        let username = format!("{}@{d}", claim.preferred_username, d = claim.uid_domain);
        User {
            uid_domain: claim.uid_domain,
            short_username: claim.preferred_username,
            username,
            email: claim.email,
            first_name: claim.given_name,
            last_name: claim.family_name,
            common_name: claim.name,
            entitlement: claim.entitlement,
        }
    }
}

impl JWTClaims {
    pub fn is_service_account(&self) -> bool {
        is_service_account(&self.preferred_username)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct OIDCKey {
    kid: String,
    kty: String,
    alg: String,

    #[serde(rename = "use")]
    why: String,

    n: String,
    e: String,
    x5c: Vec<String>,
    x5t: String,

    #[serde(rename = "x5t#S256")]
    x5t_s256: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct OIDCCert {
    keys: Vec<OIDCKey>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct OIDCToken {
    access_token: String,
    token_type: String,
    not_before_policy: u64,
    session_state: String,
    scope: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct OIDCTokenRequest {
    grant_type: String,
    client_id: String,
    client_secret: String,
    username: String,
    password: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct OIDCImpersonationTokenRequest {
    grant_type: String,
    client_id: String,
    client_secret: String,
    subject_token: String,
    requested_token_type: String,
    requested_subject: String,
}

impl OIDCImpersonationTokenRequest {
    fn new(client_id: &str, client_secret: &str, subject_token: &str, subject: &str) -> Self {
        OIDCImpersonationTokenRequest {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            subject_token: subject_token.into(),
            requested_token_type: "urn:ietf:params:oauth:token-type:access_token".into(),
            requested_subject: subject.into(),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TokenIntrospectionRequest {
    token: String,
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TokenIntrospectionResult {
    active: bool,
    exp: Option<u64>,
    iat: Option<u64>,
    jti: Option<u64>,
    iss: Option<u64>,
    sub: Option<String>,
    typ: Option<String>,
    azp: Option<String>,
    session_state: Option<String>,
    preferred_username: Option<String>,
    email_verified: Option<bool>,
    acr: Option<String>,
    scope: Option<String>,

    #[serde(rename = "DOB")]
    dob: Option<String>,
    organization: Option<String>,
    client_id: Option<String>,
    client_subject: Option<String>,
    username: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ServiceAccountInfo {
    username: String,
    roles: Vec<String>,
}

fn is_service_account(username: &str) -> bool {
    username.starts_with("service-account-")
}

pub fn token_is_valid(unparsed_token: &str) -> Result<bool, anyhow::Error> {
    //
    Ok(true)
}
