// SPDX-FileCopyrightText: 2019-2022, The Tauri Programme in the Commons Conservancy
// SPDX-FileCopyrightText: Copyright (c) 2022-2026 trobonox <hello@trobo.dev>
//
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serde::Serialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_window_state::StateFlags;
use zip::write::SimpleFileOptions;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KanriAsset {
    id: String,
    blob_path: String,
    file_name: String,
    kind: String,
    mime_type: Option<String>,
    size: Option<u64>,
}

fn sanitize_segment(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() || matches!(char, '-' | '_' | '.') {
                char
            } else {
                '_'
            }
        })
        .collect();

    if sanitized.is_empty() {
        "file".to_string()
    } else {
        sanitized
    }
}

fn assets_base(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("kanri-assets"))
}

fn board_assets_dir(app: &AppHandle, board_id: &str) -> Result<PathBuf, String> {
    Ok(assets_base(app)?
        .join("boards")
        .join(sanitize_segment(board_id))
        .join("assets"))
}

fn safe_asset_path(app: &AppHandle, blob_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(blob_path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("Invalid asset path".to_string());
    }

    let base = assets_base(app)?;
    let resolved = base.join(path);
    if !resolved.starts_with(&base) {
        return Err("Asset path escapes Kanri asset directory".to_string());
    }

    Ok(resolved)
}

fn unique_asset_path(dir: &Path, file_name: &str) -> PathBuf {
    let clean_name = sanitize_segment(file_name);
    let stem = Path::new(&clean_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    let extension = Path::new(&clean_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{value}"))
        .unwrap_or_default();
    let mut candidate = dir.join(&clean_name);
    let mut index = 1;

    while candidate.exists() {
        candidate = dir.join(format!("{stem}-{index}{extension}"));
        index += 1;
    }

    candidate
}

fn mime_type_for(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    let mime = match extension.as_str() {
        "apng" => "image/apng",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "gif" => "image/gif",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "txt" => "text/plain",
        _ => return None,
    };

    Some(mime.to_string())
}

fn build_asset(app: &AppHandle, board_id: &str, path: &Path) -> Result<KanriAsset, String> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Missing asset file name".to_string())?
        .to_string();
    let board_id = sanitize_segment(board_id);
    let blob_path = format!("boards/{board_id}/assets/{file_name}");
    let mime_type = mime_type_for(path);
    let kind = if mime_type
        .as_deref()
        .map(|value| value.starts_with("image/"))
        .unwrap_or(false)
    {
        "image"
    } else {
        "file"
    };
    let size = fs::metadata(path).ok().map(|metadata| metadata.len());
    let id = format!(
        "{}-{}",
        chrono_like_timestamp(),
        file_name.replace('.', "-")
    );

    let _ = app;
    Ok(KanriAsset {
        id,
        blob_path,
        file_name,
        kind: kind.to_string(),
        mime_type,
        size,
    })
}

fn chrono_like_timestamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[tauri::command]
fn kanri_ingest_file(app: AppHandle, board_id: String, path: String) -> Result<KanriAsset, String> {
    let source = PathBuf::from(path);
    let file_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Missing source file name".to_string())?;
    let target_dir = board_assets_dir(&app, &board_id)?;
    fs::create_dir_all(&target_dir).map_err(|error| error.to_string())?;
    let target = unique_asset_path(&target_dir, file_name);
    fs::copy(&source, &target).map_err(|error| error.to_string())?;

    build_asset(&app, &board_id, &target)
}

#[tauri::command]
fn kanri_ingest_bytes(
    app: AppHandle,
    board_id: String,
    file_name: String,
    bytes: Vec<u8>,
) -> Result<KanriAsset, String> {
    let target_dir = board_assets_dir(&app, &board_id)?;
    fs::create_dir_all(&target_dir).map_err(|error| error.to_string())?;
    let target = unique_asset_path(&target_dir, &file_name);
    fs::write(&target, bytes).map_err(|error| error.to_string())?;

    build_asset(&app, &board_id, &target)
}

#[tauri::command]
fn kanri_asset_path(app: AppHandle, blob_path: String) -> Result<String, String> {
    Ok(safe_asset_path(&app, &blob_path)?
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
fn kanri_open_asset(app: AppHandle, blob_path: String) -> Result<(), String> {
    let path = safe_asset_path(&app, &blob_path)?;
    tauri_plugin_opener::open_path(path, None::<&str>).map_err(|error| error.to_string())
}

#[tauri::command]
fn kanri_reveal_asset(app: AppHandle, blob_path: String) -> Result<(), String> {
    let path = safe_asset_path(&app, &blob_path)?;
    tauri_plugin_opener::reveal_item_in_dir(path).map_err(|error| error.to_string())
}

#[tauri::command]
fn kanri_asset_exists(app: AppHandle, blob_path: String) -> Result<bool, String> {
    Ok(safe_asset_path(&app, &blob_path)?.exists())
}

#[tauri::command]
fn kanri_delete_asset(app: AppHandle, blob_path: String) -> Result<(), String> {
    let path = safe_asset_path(&app, &blob_path)?;
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn kanri_list_board_assets(app: AppHandle, board_id: String) -> Result<Vec<String>, String> {
    let dir = board_assets_dir(&app, &board_id)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut result = Vec::new();
    for entry in fs::read_dir(dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry.path().is_file() {
            continue;
        }
        let Some(file_name) = entry.file_name().to_str().map(|value| value.to_string()) else {
            continue;
        };
        result.push(format!(
            "boards/{}/assets/{}",
            sanitize_segment(&board_id),
            file_name
        ));
    }

    Ok(result)
}

#[tauri::command]
fn kanri_copy_board_assets(
    app: AppHandle,
    source_board_id: String,
    target_board_id: String,
    blob_paths: Vec<String>,
) -> Result<HashMap<String, String>, String> {
    let mut result = HashMap::new();
    let target_dir = board_assets_dir(&app, &target_board_id)?;
    fs::create_dir_all(&target_dir).map_err(|error| error.to_string())?;

    for blob_path in blob_paths {
        let source = safe_asset_path(&app, &blob_path)?;
        if !source.exists() {
            continue;
        }

        let file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| "Missing source asset file name".to_string())?;
        let target = unique_asset_path(&target_dir, file_name);
        fs::copy(&source, &target).map_err(|error| error.to_string())?;
        let target_file_name = target
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| "Missing copied asset file name".to_string())?;
        result.insert(
            blob_path,
            format!(
                "boards/{}/assets/{}",
                sanitize_segment(&target_board_id),
                target_file_name
            ),
        );
    }

    let _ = source_board_id;
    Ok(result)
}

#[tauri::command]
fn kanri_export_bundle(
    app: AppHandle,
    json_content: String,
    board_ids: Vec<String>,
    output_path: String,
) -> Result<(), String> {
    let file = File::create(output_path).map_err(|error| error.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default();

    zip.start_file("kanri.json", options)
        .map_err(|error| error.to_string())?;
    zip.write_all(json_content.as_bytes())
        .map_err(|error| error.to_string())?;

    for board_id in board_ids {
        let dir = board_assets_dir(&app, &board_id)?;
        if !dir.exists() {
            continue;
        }
        add_zip_dir(
            &mut zip,
            &dir,
            &format!("assets/boards/{}/assets", sanitize_segment(&board_id)),
            options,
        )?;
    }

    zip.finish().map_err(|error| error.to_string())?;
    Ok(())
}

fn add_zip_dir(
    zip: &mut zip::ZipWriter<File>,
    dir: &Path,
    prefix: &str,
    options: SimpleFileOptions,
) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let name = format!(
            "{}/{}",
            prefix,
            path.file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| "Invalid asset file name".to_string())?
        );

        if path.is_dir() {
            add_zip_dir(zip, &path, &name, options)?;
        } else {
            zip.start_file(name.replace('\\', "/"), options)
                .map_err(|error| error.to_string())?;
            let mut file = File::open(path).map_err(|error| error.to_string())?;
            std::io::copy(&mut file, zip).map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

#[tauri::command]
fn kanri_import_bundle(app: AppHandle, zip_path: String) -> Result<String, String> {
    let file = File::open(zip_path).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
    let mut json_content = String::new();
    let base = assets_base(&app)?;

    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
        let Some(enclosed_name) = file.enclosed_name().map(|path| path.to_path_buf()) else {
            continue;
        };

        if enclosed_name == Path::new("kanri.json") {
            file.read_to_string(&mut json_content)
                .map_err(|error| error.to_string())?;
            continue;
        }

        if !enclosed_name.starts_with("assets") || file.is_dir() {
            continue;
        }

        let relative = enclosed_name
            .strip_prefix("assets")
            .map_err(|error| error.to_string())?;
        let target = base.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut target_file = File::create(target).map_err(|error| error.to_string())?;
        std::io::copy(&mut file, &mut target_file).map_err(|error| error.to_string())?;
    }

    if json_content.is_empty() {
        return Err("Kanri bundle does not contain kanri.json".to_string());
    }

    Ok(json_content)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    {
        if std::path::Path::new("/dev/dri").exists()
            && std::env::var("WAYLAND_DISPLAY").is_err()
            && std::env::var("XDG_SESSION_TYPE").unwrap_or_default() == "x11"
        {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                // Avoid restoring an accidentally hidden window on startup.
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            kanri_ingest_file,
            kanri_ingest_bytes,
            kanri_asset_path,
            kanri_open_asset,
            kanri_reveal_asset,
            kanri_asset_exists,
            kanri_delete_asset,
            kanri_list_board_assets,
            kanri_copy_board_assets,
            kanri_export_bundle,
            kanri_import_bundle
        ])
        .setup(|app| {
            #[cfg(desktop)]
            let _ = app
                .handle()
                .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
