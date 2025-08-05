use crate::{AppError, AppResult};
use std::fs;

pub fn get_file_content(file_path: String) -> AppResult<String> {
    let file_content = fs::read_to_string(file_path);

    match file_content {
        Ok(content) => Ok(content),
        Err(e) => Err(AppError::IO(e)),
    }
}
