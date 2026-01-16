use rayon::prelude::*;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use crate::file_management::parse_virtual_path;
use crate::image_processing::ImageMetadata;

pub const COLOR_TAG_PREFIX: &str = "color:";
#[allow(dead_code)]
pub const USER_TAG_PREFIX: &str = "user:";

fn modify_tags_for_path(
    path: &str,
    modify_fn: impl FnOnce(&mut Vec<String>),
) -> Result<(), String> {
    let (_, sidecar_path) = parse_virtual_path(path);

    let mut metadata: ImageMetadata = if sidecar_path.exists() {
        fs::read_to_string(&sidecar_path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    } else {
        ImageMetadata::default()
    };

    let mut tags = metadata.tags.unwrap_or_else(Vec::new);
    modify_fn(&mut tags);

    tags.sort_unstable();
    tags.dedup();

    if tags.is_empty() {
        metadata.tags = None;
    } else {
        metadata.tags = Some(tags);
    }

    let json_string = serde_json::to_string_pretty(&metadata).map_err(|e| e.to_string())?;
    fs::write(sidecar_path, json_string).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_tag_for_paths(paths: Vec<String>, tag: String) -> Result<(), String> {
    paths.par_iter().for_each(|path| {
        let tag_clone = tag.clone();
        if let Err(e) = modify_tags_for_path(path, |tags| {
            if !tags.contains(&tag_clone) {
                tags.push(tag_clone.clone());
            }
        }) {
            eprintln!("Failed to add tag to {}: {}", path, e);
        }
    });

    Ok(())
}

#[tauri::command]
pub fn remove_tag_for_paths(paths: Vec<String>, tag: String) -> Result<(), String> {
    paths.par_iter().for_each(|path| {
        let tag_clone = tag.clone();
        if let Err(e) = modify_tags_for_path(path, |tags| {
            tags.retain(|t| t != &tag_clone);
        }) {
            eprintln!("Failed to remove tag from {}: {}", path, e);
        }
    });

    Ok(())
}

#[tauri::command]
pub fn clear_all_tags(root_path: String) -> Result<usize, String> {
    if !Path::new(&root_path).exists() {
        return Err(format!("Root path does not exist: {}", root_path));
    }

    let mut updated_count = 0;
    let walker = WalkDir::new(root_path).into_iter();

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rrdata") {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(mut metadata) = serde_json::from_str::<ImageMetadata>(&content) {
                    if let Some(tags) = &mut metadata.tags {
                        let original_len = tags.len();
                        tags.retain(|tag| tag.starts_with(COLOR_TAG_PREFIX));

                        if tags.len() < original_len {
                            if tags.is_empty() {
                                metadata.tags = None;
                            }
                            if let Ok(json_string) = serde_json::to_string_pretty(&metadata) {
                                if fs::write(path, json_string).is_ok() {
                                    updated_count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(updated_count)
}
