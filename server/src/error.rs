use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use validator::ValidationErrors;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation failed")]
    Validation(#[from] ValidationErrors),
    #[error("invalid username/email or password")]
    InvalidCredentials,
    #[error("missing or invalid authentication token")]
    Unauthorized,
    #[error("insufficient permissions")]
    Forbidden,
    #[error("this account has been disabled")]
    AccountDisabled,
    #[error("{0}")]
    NotFound(&'static str),
    #[error("{0}")]
    Conflict(String),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("failed to process password")]
    Hash,
    #[error("internal server error")]
    Internal(String),
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

impl AppError {
    fn parts(&self) -> (StatusCode, &'static str) {
        match self {
            AppError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_FAILED"),
            AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS"),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            AppError::AccountDisabled => (StatusCode::FORBIDDEN, "ACCOUNT_DISABLED"),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR"),
            AppError::Hash => (StatusCode::INTERNAL_SERVER_ERROR, "PASSWORD_HASH_ERROR"),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.parts();

        match &self {
            AppError::Database(err) => tracing::error!(error = %err, code, "database error"),
            AppError::Internal(message) => tracing::error!(message, code, "internal error"),
            AppError::Hash => tracing::error!(code, "password hashing failure"),
            _ => tracing::debug!(code, message = %self, "request rejected"),
        }

        let message = match &self {
            AppError::Database(_) | AppError::Hash | AppError::Internal(_) => {
                "an internal error occurred".to_string()
            }
            AppError::Validation(errors) => format_validation_errors(errors),
            other => other.to_string(),
        };

        (
            status,
            Json(ErrorBody {
                error: ErrorDetail { code, message },
            }),
        )
            .into_response()
    }
}

fn format_validation_errors(errors: &ValidationErrors) -> String {
    errors
        .field_errors()
        .iter()
        .map(|(field, field_errors)| {
            let messages: Vec<String> = field_errors
                .iter()
                .map(|error| {
                    error
                        .message
                        .clone()
                        .map(|message| message.to_string())
                        .unwrap_or_else(|| error.code.to_string())
                })
                .collect();
            format!("{field}: {}", messages.join(", "))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use serde_json::Value;
    use std::borrow::Cow;
    use validator::ValidationError;

    async fn response_json(error: AppError) -> (StatusCode, Value) {
        let response = error.into_response();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        (
            status,
            serde_json::from_slice(&bytes).expect("valid json body"),
        )
    }

    #[tokio::test]
    async fn validation_error_reports_field_messages() {
        let mut errors = ValidationErrors::new();
        let mut field_error = ValidationError::new("length");
        field_error.message = Some(Cow::Borrowed("must be at least 8 characters"));
        errors.add("password", field_error);

        let (status, body) = response_json(AppError::Validation(errors)).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["error"]["code"], "VALIDATION_FAILED");
        assert_eq!(
            body["error"]["message"],
            "password: must be at least 8 characters"
        );
    }

    #[tokio::test]
    async fn invalid_credentials_maps_to_401() {
        let (status, body) = response_json(AppError::InvalidCredentials).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"]["code"], "INVALID_CREDENTIALS");
    }

    #[tokio::test]
    async fn not_found_maps_to_404_with_message() {
        let (status, body) = response_json(AppError::NotFound("user not found")).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"]["code"], "NOT_FOUND");
        assert_eq!(body["error"]["message"], "user not found");
    }

    #[tokio::test]
    async fn conflict_maps_to_409_with_message() {
        let (status, body) = response_json(AppError::Conflict("already taken".to_string())).await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["error"]["code"], "CONFLICT");
        assert_eq!(body["error"]["message"], "already taken");
    }

    #[tokio::test]
    async fn account_disabled_maps_to_403() {
        let (status, body) = response_json(AppError::AccountDisabled).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"]["code"], "ACCOUNT_DISABLED");
    }

    #[tokio::test]
    async fn database_error_never_leaks_internal_details() {
        let (status, body) = response_json(AppError::Database(sqlx::Error::RowNotFound)).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["error"]["code"], "DATABASE_ERROR");
        let message = body["error"]["message"].as_str().expect("message string");
        assert_eq!(message, "an internal error occurred");
        assert!(!message.contains("RowNotFound"));
    }
}
