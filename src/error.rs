/*
 * Copyright (c) 2024 Paul Sobolik
 * Created 2024-02-14
 */

use std::fmt::{Debug, Display, Formatter};

pub enum CliError {
    GitLib(git_lib::git_command::error::Error),
    GitApi(gitea_api::api_error::ApiError),
    Other(String),
}

impl Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            CliError::GitLib(error) => error.to_string(),
            CliError::GitApi(error) => error.to_string(),
            CliError::Other(error) => error.to_string(),
        };
        write!(f, "{}", str)
    }
}

impl Debug for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<git_lib::git_command::error::Error> for CliError {
    fn from(err: git_lib::git_command::error::Error) -> CliError {
        CliError::GitLib(err)
    }
}

impl From<gitea_api::api_error::ApiError> for CliError {
    fn from(err: gitea_api::api_error::ApiError) -> CliError {
        CliError::GitApi(err)
    }
}

impl From<String> for CliError {
    fn from(err: String) -> CliError {
        CliError::Other(err)
    }
}

impl From<&str> for CliError {
    fn from(err: &str) -> CliError {
        CliError::Other(err.to_string())
    }
}
