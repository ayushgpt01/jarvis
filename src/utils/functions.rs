use std::fs;

use crate::error::AppError;

pub fn get_file_content(file_path: String) -> Result<String, AppError> {
    let file_content = fs::read_to_string(file_path);

    match file_content {
        Ok(content) => Ok(content),
        Err(e) => Err(AppError::IO(e)),
    }
}
