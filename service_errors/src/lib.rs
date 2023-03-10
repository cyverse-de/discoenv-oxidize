use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use debuff::header;
use debuff::svcerror::{ErrorCode, ServiceError};

use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum DiscoError {
    #[error("unset: {0}")]
    Unset(String),

    #[error("unspecified error:: {0}")]
    Unspecified(String),

    #[error("internal server error: {0}")]
    Internal(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("marshal failure: {0}")]
    MarshalFailure(String),

    #[error("unmarshal failure: {0}")]
    UnmarshalFailure(String),

    #[error("parameter missing: {0}")]
    ParameterMissing(String),

    #[error("parameter invalid: {0}")]
    ParameterInvalid(String),
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
            DiscoError::UnmarshalFailure(_) => ErrorCode::BadRequest,
            DiscoError::ParameterMissing(_) => ErrorCode::BadRequest,
            DiscoError::ParameterInvalid(_) => ErrorCode::BadRequest,
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
        }
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
        let msg = self.to_string().clone();
        let status_code: StatusCode = self.into();
        (status_code, msg).into_response()
    }
}

impl Clone for DiscoError {
    fn clone(&self) -> DiscoError {
        match self {
            DiscoError::Unset(m) => DiscoError::Unset(m.to_owned()),
            DiscoError::Unspecified(m) => DiscoError::Unspecified(m.to_owned()),
            DiscoError::Internal(m) => DiscoError::Internal(m.to_owned()),
            DiscoError::NotFound(m) => DiscoError::NotFound(m.to_owned()),
            DiscoError::BadRequest(m) => DiscoError::BadRequest(m.to_owned()),
            DiscoError::MarshalFailure(m) => DiscoError::MarshalFailure(m.to_owned()),
            DiscoError::UnmarshalFailure(m) => DiscoError::UnmarshalFailure(m.to_owned()),
            DiscoError::ParameterMissing(m) => DiscoError::ParameterMissing(m.to_owned()),
            DiscoError::ParameterInvalid(m) => DiscoError::ParameterMissing(m.to_owned()),
        }
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
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e.to_string(), "not found: this is a test".to_owned()),
        };
    }
}
