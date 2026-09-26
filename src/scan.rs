use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::config::resolve_path;
use crate::error::ApiError;
use crate::ids::path_to_id;
use crate::registry::BookRecord;

const IGNORED_DIR_NAMES: &[&str] = &[".obsidian", ".git", "node_modules"];

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub id: u32,
    pub code: String,
    pub root_id: u32,
    pub name: String,
    pub order: i64,
    pub user_id: String,
    pub folder_path: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: u32,
    pub create_time: Option<()>,
    pub update_time: Option<()>,
    pub app: u32,
    pub user_id: String,
    #[serde(rename = "type")]
    pub node_type: Option<()>,
    pub book_id: u32,
    pub parent_id: Option<u32>,
    pub children: Vec<u32>,
    pub title: String,
    pub name: String,
    pub text: Option<String>,
    pub data: Option<()>,
    pub rel_path: Option<String>,
}

struct DirEntry {
    name: String,
    path: PathBuf,
}

pub fn scan_book_folder(record: &BookRecord) -> Result<(Book, Vec<Node>), ApiError> {
    let book_dir = resolve_path(&record.path);
    let meta = fs::metadata(&book_dir).ok();
    if meta.as_ref().is_none_or(|meta| !meta.is_dir()) {
        return Err(ApiError::new(
            404,
            format!("Book folder not found: {}", book_dir.display()),
        ));
    }

    let mut nodes = Vec::new();
    visit(record, &book_dir, &book_dir, None, true, &mut nodes)?;
    let root = nodes
        .iter_mut()
        .find(|node| node.parent_id.is_none())
        .ok_or_else(|| ApiError::new(500, format!("Missing root node for book {}", record.name)))?;
    root.name.clone_from(&record.name);
    root.title.clone_from(&record.name);
    let root_id = root.id;

    let book = Book {
        id: record.id,
        code: record.name.clone(),
        root_id,
        name: record.name.clone(),
        order: record.order,
        user_id: "local".to_string(),
        folder_path: book_dir.to_string_lossy().into_owned(),
    };
    Ok((book, nodes))
}

pub fn list_books_from_registry(records: &[BookRecord]) -> Vec<Book> {
    let mut sorted = records.to_vec();
    sorted.sort_by_key(|record| record.order);
    sorted
        .into_iter()
        .map(|record| match scan_book_folder(&record) {
            Ok((book, _)) => book,
            Err(err) => {
                eprintln!("Book folder unavailable: {} {err}", record.path);
                let folder = resolve_path(&record.path);
                let folder_path = folder.to_string_lossy().into_owned();
                Book {
                    id: record.id,
                    code: record.name.clone(),
                    root_id: path_to_id(&format!("{folder_path}:.")),
                    name: record.name,
                    order: record.order,
                    user_id: "local".to_string(),
                    folder_path,
                }
            }
        })
        .collect()
}

pub fn write_node_text(record: &BookRecord, node_id: u32, text: &str) -> Result<Node, ApiError> {
    let (_, nodes) = scan_book_folder(record)?;
    let mut node = nodes
        .into_iter()
        .find(|node| node.id == node_id)
        .ok_or_else(|| ApiError::new(404, "Node not found"))?;
    let rel = node
        .rel_path
        .clone()
        .ok_or_else(|| ApiError::new(400, "Cannot write: node is not a markdown file"))?;
    let book_dir = resolve_path(&record.path);
    let candidate = book_dir.join(&rel);
    let abs = assert_under_root(&book_dir, &candidate, &candidate)?;
    fs::write(abs, text)?;
    node.text = Some(text.to_string());
    Ok(node)
}

fn visit(
    record: &BookRecord,
    book_dir: &Path,
    abs_path: &Path,
    parent_id: Option<u32>,
    is_root: bool,
    nodes: &mut Vec<Node>,
) -> Result<u32, ApiError> {
    let abs_path = assert_under_root(book_dir, abs_path, abs_path)?;
    let meta = fs::metadata(&abs_path)?;
    let is_dir = meta.is_dir();
    let entry_name = abs_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = if is_root {
        record.name.clone()
    } else {
        display_name(&entry_name, !is_dir)
    };
    let rel = if is_root {
        ".".to_string()
    } else {
        relative_slash(book_dir, &abs_path)
    };
    let id_prefix = book_dir.to_string_lossy();
    let id = path_to_id(&format!("{id_prefix}:{rel}"));

    let mut children = Vec::new();
    let text = if is_dir {
        for entry in list_entries(&abs_path)? {
            let child_id = visit(record, book_dir, &entry.path, Some(id), false, nodes)?;
            children.push(child_id);
        }
        None
    } else {
        Some(fs::read_to_string(&abs_path)?)
    };

    nodes.push(Node {
        id,
        create_time: None,
        update_time: None,
        app: 1,
        user_id: "local".to_string(),
        node_type: None,
        book_id: record.id,
        parent_id,
        children,
        title: name.clone(),
        name,
        text,
        data: None,
        rel_path: if is_dir { None } else { Some(rel) },
    });
    Ok(id)
}

fn list_entries(dir: &Path) -> Result<Vec<DirEntry>, ApiError> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_ignored_name(&name) {
            continue;
        }
        let path = entry.path();
        let meta = fs::metadata(&path)?;
        if meta.is_dir() || (meta.is_file() && name.to_ascii_lowercase().ends_with(".md")) {
            entries.push(DirEntry { name, path });
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

fn is_ignored_name(name: &str) -> bool {
    IGNORED_DIR_NAMES.contains(&name) || name.starts_with('.')
}

fn display_name(entry_name: &str, is_file: bool) -> String {
    if is_file && entry_name.to_ascii_lowercase().ends_with(".md") {
        entry_name[..entry_name.len() - 3].to_string()
    } else {
        entry_name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn book_json_keeps_field_order() {
        let book = Book {
            id: 1,
            code: "Personal".to_string(),
            root_id: 2,
            name: "Personal".to_string(),
            order: 0,
            user_id: "local".to_string(),
            folder_path: "/tmp/Personal".to_string(),
        };
        let value = serde_json::to_value(&book).unwrap();
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(
            json,
            r#"{"id":1,"code":"Personal","rootId":2,"name":"Personal","order":0,"userId":"local","folderPath":"/tmp/Personal"}"#
        );
    }
}

fn relative_slash(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Keep `candidate` lexically inside `book_root`. A path escape is a 500, matching
/// the previous server, which threw a generic Error.
fn assert_under_root(
    book_root: &Path,
    candidate: &Path,
    reported: &Path,
) -> Result<PathBuf, ApiError> {
    let root = resolve_path(&book_root.to_string_lossy());
    let resolved = resolve_path(&candidate.to_string_lossy());
    let root_str = root.to_string_lossy();
    let resolved_str = resolved.to_string_lossy();
    let prefix = format!("{root_str}{}", std::path::MAIN_SEPARATOR);
    if resolved_str.as_ref() != root_str.as_ref() && !resolved_str.starts_with(prefix.as_str()) {
        return Err(ApiError::new(
            500,
            format!("Path escapes book folder: {}", reported.display()),
        ));
    }
    Ok(resolved)
}
