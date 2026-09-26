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

#[derive(Serialize, Deserialize)]
struct PencilFile {
    books: Vec<BookRecord>,
}

pub fn load_registry(config_file: &Path) -> Result<Vec<BookRecord>, ApiError> {
    ensure_file(config_file)?;
    let raw = fs::read_to_string(config_file)?;
    if raw.trim().is_empty() {
        return Err(ApiError::new(500, "config file is empty"));
    }
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).map_err(|err| ApiError::new(500, err.to_string()))?;
    let books = parsed
        .get("books")
        .ok_or_else(|| ApiError::new(500, "config file must contain a books array"))?;
    if !books.is_array() {
        return Err(ApiError::new(500, "config file must contain a books array"));
    }
    serde_json::from_value(books.clone()).map_err(|err| ApiError::new(500, err.to_string()))
}

pub fn save_registry(config_file: &Path, records: &[BookRecord]) -> Result<(), ApiError> {
    ensure_file(config_file)?;
    let body = serde_json::to_string_pretty(&PencilFile {
        books: records.to_vec(),
    })
    .map_err(|err| ApiError::new(500, err.to_string()))?;
    fs::write(config_file, format!("{body}\n"))?;
    Ok(())
}

/// Create a config file containing one book. `folder_path` must be an existing directory.
pub fn create_with_book(config_file: &Path, folder_path: &str) -> Result<BookRecord, ApiError> {
    let abs = resolve_path(folder_path);
    let meta = fs::metadata(&abs).ok();
    if meta.as_ref().is_none_or(|meta| !meta.is_dir()) {
        return Err(ApiError::new(
            400,
            format!("Not a directory: {}", abs.display()),
        ));
    }
    if let Some(parent) = config_file.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let abs_str = path_string(&abs);
    let name = abs
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Book".to_string());
    let record = BookRecord {
        id: book_id_for(&abs_str),
        name,
        path: abs_str,
        order: 0,
    };
    save_registry(config_file, &[record.clone()])?;
    Ok(record)
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
        fs::write(books_file, "{\n  \"books\": []\n}\n")?;
    }
    Ok(())
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_file_wraps_books() {
        let dir = std::env::temp_dir().join(format!("pencil-config-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let folder = dir.join("Notes");
        fs::create_dir_all(&folder).unwrap();
        let file = dir.join("pencil.json");

        let record = create_with_book(&file, folder.to_str().unwrap()).unwrap();
        assert_eq!(record.name, "Notes");
        assert_eq!(record.order, 0);

        let raw = fs::read_to_string(&file).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed["books"][0]["path"], record.path);
        assert_eq!(load_registry(&file).unwrap(), vec![record]);

        let _ = fs::remove_dir_all(&dir);
    }
}
