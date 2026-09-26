use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::resolve_path;
use crate::error::ApiError;
use crate::ids::book_id_for;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BookRecord {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub order: i64,
}

pub fn load_registry(books_file: &Path) -> Result<Vec<BookRecord>, ApiError> {
    ensure_file(books_file)?;
    let raw = fs::read_to_string(books_file)?;
    let parsed: serde_json::Value =
        serde_json::from_str(if raw.trim().is_empty() { "[]" } else { &raw })
            .map_err(|err| ApiError::new(500, err.to_string()))?;
    if !parsed.is_array() {
        return Err(ApiError::new(500, "books.json must be an array"));
    }
    serde_json::from_value(parsed).map_err(|err| ApiError::new(500, err.to_string()))
}

pub fn save_registry(books_file: &Path, records: &[BookRecord]) -> Result<(), ApiError> {
    ensure_file(books_file)?;
    let body =
        serde_json::to_string_pretty(records).map_err(|err| ApiError::new(500, err.to_string()))?;
    fs::write(books_file, format!("{body}\n"))?;
    Ok(())
}

pub fn add_book(books_file: &Path, name: &str, folder_path: &str) -> Result<BookRecord, ApiError> {
    let abs = resolve_path(folder_path);
    let abs_str = path_string(&abs);
    let mut records = load_registry(books_file)?;
    if records
        .iter()
        .any(|record| resolve_path(&record.path) == abs)
    {
        return Err(ApiError::new(409, "Folder is already linked"));
    }
    let order = records
        .iter()
        .map(|record| record.order)
        .max()
        .map_or(0, |order| order + 1);
    let record = BookRecord {
        id: book_id_for(&abs_str),
        name: name.trim().to_string(),
        path: abs_str,
        order,
    };
    records.push(record.clone());
    save_registry(books_file, &records)?;
    Ok(record)
}

pub fn update_book_name(
    books_file: &Path,
    book_id: u32,
    name: &str,
) -> Result<BookRecord, ApiError> {
    let mut records = load_registry(books_file)?;
    let record = records
        .iter_mut()
        .find(|record| record.id == book_id)
        .ok_or_else(|| ApiError::new(404, "Book not found"))?;
    record.name = name.trim().to_string();
    let updated = record.clone();
    save_registry(books_file, &records)?;
    Ok(updated)
}

pub fn remove_book(books_file: &Path, book_id: u32) -> Result<(), ApiError> {
    let records = load_registry(books_file)?;
    let next: Vec<BookRecord> = records
        .iter()
        .filter(|record| record.id != book_id)
        .cloned()
        .collect();
    if next.len() == records.len() {
        return Err(ApiError::new(404, "Book not found"));
    }
    save_registry(books_file, &next)
}

pub fn get_book_record(books_file: &Path, book_id: u32) -> Result<Option<BookRecord>, ApiError> {
    let records = load_registry(books_file)?;
    Ok(records.into_iter().find(|record| record.id == book_id))
}

fn ensure_file(books_file: &Path) -> Result<(), ApiError> {
    if let Some(parent) = books_file.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    if !books_file.exists() {
        fs::write(books_file, "[]\n")?;
    }
    Ok(())
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
