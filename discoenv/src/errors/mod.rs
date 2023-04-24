use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use debuff::header;
use debuff::svcerror::{ErrorCode, ServiceError};
use reqwest;
use serde_json::json;
use utoipa::ToSchema;

use thiserror;

#[derive(thiserror::Error, Clone, Debug, ToSchema, serde::Serialize, serde::Deserialize)]
pub enum DiscoError {
    /// Something was unset.
    #[error("unset: {0}")]
    #[schema(example = "unset: foo")]
    Unset(String),

    /// Something was unspecified.
    #[error("unspecified error:: {0}")]
    #[schema()]
    Unspecified(String),

    /// Internal server error.
    #[error("internal server error: {0}")]
    #[schema()]
    Internal(String),

    /// Not found.
    #[error("not found: {0}")]
    #[schema()]
    NotFound(String),

    /// Bad request.
    #[error("bad request: {0}")]
    #[schema()]
    BadRequest(String),

    /// An attempt to marshal a value failed.
    #[error("marshal failure: {0}")]
    #[schema()]
    MarshalFailure(String),

    /// An attempt to unmarshal a value failed.
    #[error("unmarshal failure: {0}")]
    #[schema()]
    UnmarshalFailure(String),

    /// A parameter was missing.
    #[error("parameter missing: {0}")]
    #[schema()]
    ParameterMissing(String),

    /// A parameter was invalid.
    #[error("parameter invalid: {0}")]
    #[schema()]
    ParameterInvalid(String),

    #[error("unauthenticated: {0}")]
    #[schema()]
    Unauthenticated(String),

    #[error("forbidden: {0}")]
    #[schema()]
    Forbidden(String),

    #[error("timed out: {0}")]
    #[schema()]
    Timeout(String),

    #[error("unsupported operation: {0}")]
    #[schema()]
    Unsupported(String),

    #[error("unimplemented: {0}")]
    #[schema()]
    Unimplemented(String),
}

impl DiscoError {
    fn error_code(&self) -> ErrorCode {
        match self {
            DiscoError::Unset(_) => ErrorCode::Internal,
            DiscoError::Unspecified(_) => ErrorCode::Internal,
            DiscoError::Internal(_) => ErrorCode::Internal,
            DiscoError::NotFound(_) => ErrorCode::NotFound,
            DiscoError::BadRequest(_) => ErrorCode::BadRequest,
            DiscoError::MarshalFailure(_) => ErrorCode::Internal,
            DiscoError::UnmarshalFailure(_) => ErrorCode::UnmarshalFailure,
            DiscoError::ParameterMissing(_) => ErrorCode::ParameterMissing,
            DiscoError::ParameterInvalid(_) => ErrorCode::ParameterInvalid,
            DiscoError::Unauthenticated(_) => ErrorCode::Unauthenticated,
            DiscoError::Forbidden(_) => ErrorCode::Forbidden,
            DiscoError::Timeout(_) => ErrorCode::Timeout,
            DiscoError::Unsupported(_) => ErrorCode::Unsupported,
            DiscoError::Unimplemented(_) => ErrorCode::Unimplemented,
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            DiscoError::Unset(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DiscoError::Unspecified(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DiscoError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DiscoError::NotFound(_) => StatusCode::NOT_FOUND,
            DiscoError::BadRequest(_) => StatusCode::BAD_REQUEST,
            DiscoError::MarshalFailure(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DiscoError::UnmarshalFailure(_) => StatusCode::BAD_REQUEST,
            DiscoError::ParameterMissing(_) => StatusCode::BAD_REQUEST,
            DiscoError::ParameterInvalid(_) => StatusCode::BAD_REQUEST,
            DiscoError::Unauthenticated(_) => StatusCode::UNAUTHORIZED,
            DiscoError::Forbidden(_) => StatusCode::FORBIDDEN,
            DiscoError::Timeout(_) => StatusCode::REQUEST_TIMEOUT,
            DiscoError::Unsupported(_) => StatusCode::METHOD_NOT_ALLOWED,
            DiscoError::Unimplemented(_) => StatusCode::NOT_IMPLEMENTED,
        }
    }

    fn to_json_string(&self) -> String {
        let s_err: ServiceError = self.create_service_error();
        json!(s_err).to_string()
    }

    pub fn create_service_error(&self) -> ServiceError {
        self.clone().into()
    }

    fn msg(&self) -> String {
        match self {
            DiscoError::Unset(m) => m.to_owned(),
            DiscoError::Unspecified(m) => m.to_owned(),
            DiscoError::Internal(m) => m.to_owned(),
            DiscoError::NotFound(m) => m.to_owned(),
            DiscoError::BadRequest(m) => m.to_owned(),
            DiscoError::MarshalFailure(m) => m.to_owned(),
            DiscoError::UnmarshalFailure(m) => m.to_owned(),
            DiscoError::ParameterMissing(m) => m.to_owned(),
            DiscoError::ParameterInvalid(m) => m.to_owned(),
            DiscoError::Unauthenticated(m) => m.to_owned(),
            DiscoError::Forbidden(m) => m.into(),
            DiscoError::Timeout(m) => m.into(),
            DiscoError::Unsupported(m) => m.into(),
            DiscoError::Unimplemented(m) => m.into(),
        }
    }
}

impl From<ServiceError> for DiscoError {
    fn from(s: ServiceError) -> Self {
        let msg = s.message.clone();
        match s.error_code() {
            ErrorCode::Unset => DiscoError::Unset(msg),
            ErrorCode::Unspecified => DiscoError::Unspecified(msg),
            ErrorCode::Internal => DiscoError::Internal(msg),
            ErrorCode::NotFound => DiscoError::NotFound(msg),
            ErrorCode::BadRequest => DiscoError::BadRequest(msg),
            ErrorCode::MarshalFailure => DiscoError::MarshalFailure(msg),
            ErrorCode::UnmarshalFailure => DiscoError::UnmarshalFailure(msg),
            ErrorCode::ParameterMissing => DiscoError::ParameterMissing(msg),
            ErrorCode::ParameterInvalid => DiscoError::ParameterInvalid(msg),
            ErrorCode::Unauthenticated => DiscoError::Unauthenticated(msg),
            ErrorCode::Forbidden => DiscoError::Forbidden(msg),
            ErrorCode::Timeout => DiscoError::Timeout(msg),
            ErrorCode::Unsupported => DiscoError::Unsupported(msg),
            ErrorCode::Unimplemented => DiscoError::Unimplemented(msg),
        }
    }
}

impl From<reqwest::Error> for DiscoError {
    fn from(e: reqwest::Error) -> Self {
        let msg = e.to_string();
        if e.status().is_none() {
            return DiscoError::Unspecified(msg);
        }
        let sc = e.status().unwrap_or_default();
        match sc {
            reqwest::StatusCode::INTERNAL_SERVER_ERROR => DiscoError::Internal(msg),
            reqwest::StatusCode::NOT_FOUND => DiscoError::NotFound(msg),
            reqwest::StatusCode::BAD_REQUEST => DiscoError::BadRequest(msg),
            reqwest::StatusCode::UNAUTHORIZED => DiscoError::Unauthenticated(msg),
            reqwest::StatusCode::FORBIDDEN => DiscoError::Forbidden(msg),
            reqwest::StatusCode::REQUEST_TIMEOUT => DiscoError::Timeout(msg),
            reqwest::StatusCode::METHOD_NOT_ALLOWED => DiscoError::Unsupported(msg),
            reqwest::StatusCode::NOT_IMPLEMENTED => DiscoError::Unimplemented(msg),
            _ => DiscoError::Unspecified(msg),
        }
    }
}

impl From<sqlx::Error> for DiscoError {
    fn from(s: sqlx::Error) -> Self {
        let msg = s.to_string();
        match s {
            sqlx::Error::RowNotFound => DiscoError::NotFound(msg),
            _ => DiscoError::Internal(msg),
        }
    }
}

impl From<DiscoError> for ServiceError {
    fn from(d: DiscoError) -> Self {
        let error_code: i32 = d.error_code().into();
        let status_code: i32 = i32::from(d.status_code().as_u16());
        let message: String = d.msg();

        ServiceError {
            header: Some(header::Header::default()),
            error_code,
            status_code,
            message,
        }
    }
}

impl From<DiscoError> for StatusCode {
    fn from(d: DiscoError) -> Self {
        d.status_code()
    }
}

impl From<DiscoError> for ErrorCode {
    fn from(d: DiscoError) -> Self {
        d.error_code()
    }
}

impl IntoResponse for DiscoError {
    fn into_response(self) -> Response {
        let msg: String = self.to_json_string();
        let status_code: StatusCode = self.into();
        (status_code, msg).into_response()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::Result;

    fn return_notfound() -> Result<bool> {
        Err(DiscoError::NotFound("this is a test".to_owned()).into())
    }

    #[test]
    fn test_discoerror() {
        let se = ServiceError {
            header: Some(header::Header::default()),
            error_code: ErrorCode::Internal.into(),
            status_code: i32::from(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            message: "this is a test internal server error".to_owned(),
        };

        // Make sure the From for DiscoError is working.
        let de: DiscoError = se.clone().into();
        assert_eq!(de.msg(), se.message);
        assert_eq!(i32::from(de.error_code()), se.error_code);
        assert_eq!(i32::from(de.status_code().as_u16()), se.status_code);

        // Make sure the conversion from DiscoError to ServiceError is working.
        let new_se: ServiceError = de.into();
        assert_eq!(new_se.message, se.message);
        assert_eq!(new_se.error_code, se.error_code);
        assert_eq!(new_se.status_code, se.status_code);
    }

    #[test]
    fn test_result_discoerror() {
        let r = return_notfound();
        assert!(r.is_err());

        match r {
            Ok(_) => panic!(),
            Err(e) => assert_eq!(e.to_string(), "not found: this is a test".to_owned()),
        };
    }
}
