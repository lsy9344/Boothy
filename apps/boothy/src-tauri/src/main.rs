#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod camera;
mod culling;
mod denoising;
mod error;
mod file_management;
mod formats;
mod gpu_processing;
mod image_loader;
mod image_processing;
mod ingest;
mod inpainting;
mod logging;
mod lut_processing;
mod mask_generation;
mod mode;
mod panorama_stitching;
mod panorama_utils;
mod preset;
mod preset_converter;
mod raw_processing;
mod session;
mod tagging;
mod watcher;

use log;
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Cursor;
use std::io::Write;
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use image::codecs::jpeg::JpegEncoder;
use image::{
    DynamicImage, GenericImageView, GrayImage, ImageBuffer, ImageFormat, Luma, Rgba,
    RgbaImage, imageops,
};
use little_exif::exif_tag::ExifTag;
use little_exif::filetype::FileExtension;
use little_exif::metadata::Metadata;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{Emitter, Manager, ipc::Response};
use tempfile::NamedTempFile;
use tokio::task::JoinHandle;
use wgpu::{Texture, TextureView};

use crate::file_management::{AppSettings, load_settings, parse_virtual_path, read_file_mapped};
use crate::formats::is_raw_file;
use crate::image_loader::{load_and_composite, load_base_image_from_bytes};
use crate::image_processing::{
    Crop, GpuContext, ImageMetadata, apply_coarse_rotation, apply_cpu_default_raw_processing,
    apply_crop, apply_flip, apply_rotation, downscale_f32_image, get_all_adjustments_from_json,
    get_or_init_gpu_context, process_and_get_dynamic_image,
};
use crate::lut_processing::Lut;
use crate::mask_generation::{MaskDefinition, generate_mask_bitmap};
use crate::session::export::{
    BoothyExportChoice, ExportProgressState, build_photo_state_map, collect_session_raw_files,
    filter_export_paths, load_session_metadata,
};

#[derive(Clone)]
pub struct LoadedImage {
    path: String,
    image: DynamicImage,
    is_raw: bool,
}

#[derive(Clone)]
pub struct CachedPreview {
    image: DynamicImage,
    small_image: DynamicImage,
    transform_hash: u64,
    scale: f32,
    unscaled_crop_offset: (f32, f32),
}

pub struct GpuImageCache {
    pub texture: Texture,
    pub texture_view: TextureView,
    pub width: u32,
    pub height: u32,
    pub transform_hash: u64,
}

pub struct GpuProcessorState {
    pub processor: crate::gpu_processing::GpuProcessor,
    pub width: u32,
    pub height: u32,
}

struct PreviewJob {
    adjustments: serde_json::Value,
    is_interactive: bool,
}

pub struct AppState {
    original_image: Mutex<Option<LoadedImage>>,
    cached_preview: Mutex<Option<CachedPreview>>,
    gpu_context: Mutex<Option<GpuContext>>,
    gpu_image_cache: Mutex<Option<GpuImageCache>>,
    gpu_processor: Mutex<Option<GpuProcessorState>>,
    export_task_handle: Mutex<Option<JoinHandle<()>>>,
    panorama_result: Arc<Mutex<Option<DynamicImage>>>,
    denoise_result: Arc<Mutex<Option<DynamicImage>>>,
    pub lut_cache: Mutex<HashMap<String, Arc<Lut>>>,
    initial_file_path: Mutex<Option<String>>,
    thumbnail_cancellation_token: Arc<AtomicBool>,
    preview_worker_tx: Mutex<Option<Sender<PreviewJob>>>,
    pub mask_cache: Mutex<HashMap<u64, GrayImage>>,
    // Boothy-specific state
    pub session_manager: session::SessionManager,
    pub session_timer: session::SessionTimer,
    pub mode_manager: mode::ModeManager,
    pub file_watcher: watcher::FileWatcher,
    pub camera_client: Mutex<Option<camera::ipc_client::CameraIpcClient>>,
    pub file_arrival_watcher: Mutex<Option<ingest::file_watcher::FileArrivalWatcher>>,
    pub preset_manager: preset::preset_manager::PresetManager,
    pub background_export_queue: Arc<session::export_queue::BackgroundExportQueue>,
}

#[derive(serde::Serialize)]
struct LoadImageResult {
    width: u32,
    height: u32,
    metadata: ImageMetadata,
    exif: HashMap<String, String>,
    is_raw: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
enum ResizeMode {
    LongEdge,
    ShortEdge,
    Width,
    Height,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct ResizeOptions {
    mode: ResizeMode,
    value: u32,
    dont_enlarge: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct ExportSettings {
    jpeg_quality: u8,
    resize: Option<ResizeOptions>,
    keep_metadata: bool,
    strip_gps: bool,
    filename_template: Option<String>,
    watermark: Option<WatermarkSettings>,
}

#[derive(Serialize)]
struct LutParseResult {
    size: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum WatermarkAnchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkSettings {
    path: String,
    anchor: WatermarkAnchor,
    scale: f32,
    spacing: f32,
    opacity: f32,
}

#[derive(serde::Serialize)]
struct ImageDimensions {
    width: u32,
    height: u32,
}

fn apply_all_transformations(
    image: &DynamicImage,
    adjustments: &serde_json::Value,
) -> (DynamicImage, (f32, f32)) {
    let start_time = std::time::Instant::now();

    let orientation_steps = adjustments["orientationSteps"].as_u64().unwrap_or(0) as u8;
    let rotation_degrees = adjustments["rotation"].as_f64().unwrap_or(0.0) as f32;
    let flip_horizontal = adjustments["flipHorizontal"].as_bool().unwrap_or(false);
    let flip_vertical = adjustments["flipVertical"].as_bool().unwrap_or(false);

    let coarse_rotated_image = apply_coarse_rotation(image.clone(), orientation_steps);
    let flipped_image = apply_flip(coarse_rotated_image, flip_horizontal, flip_vertical);
    let rotated_image = apply_rotation(&flipped_image, rotation_degrees);

    let crop_data: Option<Crop> = serde_json::from_value(adjustments["crop"].clone()).ok();
    let crop_json = serde_json::to_value(crop_data.clone()).unwrap_or(serde_json::Value::Null);
    let cropped_image = apply_crop(rotated_image, &crop_json);

    let unscaled_crop_offset = crop_data.map_or((0.0, 0.0), |c| (c.x as f32, c.y as f32));

    let duration = start_time.elapsed();
    log::info!("apply_all_transformations took: {:?}", duration);
    (cropped_image, unscaled_crop_offset)
}

fn calculate_transform_hash(adjustments: &serde_json::Value) -> u64 {
    let mut hasher = DefaultHasher::new();

    let orientation_steps = adjustments["orientationSteps"].as_u64().unwrap_or(0);
    orientation_steps.hash(&mut hasher);

    let rotation = adjustments["rotation"].as_f64().unwrap_or(0.0);
    (rotation.to_bits()).hash(&mut hasher);

    let flip_h = adjustments["flipHorizontal"].as_bool().unwrap_or(false);
    flip_h.hash(&mut hasher);

    let flip_v = adjustments["flipVertical"].as_bool().unwrap_or(false);
    flip_v.hash(&mut hasher);

    if let Some(crop_val) = adjustments.get("crop") {
        if !crop_val.is_null() {
            crop_val.to_string().hash(&mut hasher);
        }
    }

    hasher.finish()
}

fn calculate_full_job_hash(path: &str, adjustments: &serde_json::Value) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    adjustments.to_string().hash(&mut hasher);
    hasher.finish()
}

fn generate_transformed_preview(
    loaded_image: &LoadedImage,
    adjustments: &serde_json::Value,
    app_handle: &tauri::AppHandle,
) -> Result<(DynamicImage, f32, (f32, f32)), String> {
    let (transformed_full_res, unscaled_crop_offset) =
        apply_all_transformations(&loaded_image.image, adjustments);

    let settings = load_settings(app_handle.clone()).unwrap_or_default();
    let final_preview_dim = settings.editor_preview_resolution.unwrap_or(1920);

    let (full_res_w, full_res_h) = transformed_full_res.dimensions();

    let final_preview_base = if full_res_w > final_preview_dim || full_res_h > final_preview_dim {
        downscale_f32_image(&transformed_full_res, final_preview_dim, final_preview_dim)
    } else {
        transformed_full_res
    };

    let scale_for_gpu = if full_res_w > 0 {
        final_preview_base.width() as f32 / full_res_w as f32
    } else {
        1.0
    };

    Ok((final_preview_base, scale_for_gpu, unscaled_crop_offset))
}

fn read_exif_data(file_bytes: &[u8]) -> HashMap<String, String> {
    let mut exif_data = HashMap::new();
    let exif_reader = exif::Reader::new();
    if let Ok(exif) = exif_reader.read_from_container(&mut Cursor::new(file_bytes)) {
        for field in exif.fields() {
            exif_data.insert(
                field.tag.to_string(),
                field.display_value().with_unit(&exif).to_string(),
            );
        }
    }
    exif_data
}

fn get_or_load_lut(state: &tauri::State<'_, AppState>, path: &str) -> Result<Arc<Lut>, String> {
    let mut cache = state.lut_cache.lock().unwrap();
    if let Some(lut) = cache.get(path) {
        return Ok(lut.clone());
    }

    let lut = lut_processing::parse_lut_file(path).map_err(|e| e.to_string())?;
    let arc_lut = Arc::new(lut);
    cache.insert(path.to_string(), arc_lut.clone());
    Ok(arc_lut)
}

#[tauri::command]
async fn load_image(
    path: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<LoadImageResult, String> {
    let (source_path, sidecar_path) = parse_virtual_path(&path);
    let source_path_str = source_path.to_string_lossy().to_string();

    let metadata: ImageMetadata = if sidecar_path.exists() {
        let file_content = fs::read_to_string(sidecar_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&file_content).unwrap_or_default()
    } else {
        ImageMetadata::default()
    };

    let settings = load_settings(app_handle.clone()).unwrap_or_default();
    let highlight_compression = settings.raw_highlight_compression.unwrap_or(2.5);

    let path_clone = source_path_str.clone();
    let (pristine_img, exif_data) = tokio::task::spawn_blocking(move || {
        let result: Result<(DynamicImage, HashMap<String, String>), String> =
            (|| match read_file_mapped(Path::new(&path_clone)) {
                Ok(mmap) => {
                    let img = load_base_image_from_bytes(
                        &mmap,
                        &path_clone,
                        false,
                        highlight_compression,
                    )
                    .map_err(|e| e.to_string())?;
                    let exif = read_exif_data(&mmap);
                    Ok((img, exif))
                }
                Err(e) => {
                    log::warn!(
                        "Failed to memory-map file '{}': {}. Falling back to standard read.",
                        path_clone,
                        e
                    );
                    let bytes = fs::read(&path_clone).map_err(|io_err| {
                        format!("Fallback read failed for {}: {}", path_clone, io_err)
                    })?;
                    let img = load_base_image_from_bytes(
                        &bytes,
                        &path_clone,
                        false,
                        highlight_compression,
                    )
                    .map_err(|e| e.to_string())?;
                    let exif = read_exif_data(&bytes);
                    Ok((img, exif))
                }
            })();
        result
    })
    .await
    .map_err(|e| e.to_string())??;

    let (orig_width, orig_height) = pristine_img.dimensions();
    let is_raw = is_raw_file(&source_path_str);

    *state.cached_preview.lock().unwrap() = None;
    *state.gpu_image_cache.lock().unwrap() = None;
    state.mask_cache.lock().unwrap().clear();

    *state.original_image.lock().unwrap() = Some(LoadedImage {
        path: source_path_str.clone(),
        image: pristine_img,
        is_raw,
    });

    Ok(LoadImageResult {
        width: orig_width,
        height: orig_height,
        metadata,
        exif: exif_data,
        is_raw,
    })
}

#[tauri::command]
fn get_image_dimensions(path: String) -> Result<ImageDimensions, String> {
    let (source_path, _) = parse_virtual_path(&path);
    image::image_dimensions(&source_path)
        .map(|(width, height)| ImageDimensions { width, height })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn cancel_thumbnail_generation(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .thumbnail_cancellation_token
        .store(true, Ordering::SeqCst);
    Ok(())
}

fn apply_watermark(
    base_image: &mut DynamicImage,
    watermark_settings: &WatermarkSettings,
) -> Result<(), String> {
    let watermark_img = image::open(&watermark_settings.path)
        .map_err(|e| format!("Failed to open watermark image: {}", e))?;

    let (base_w, base_h) = base_image.dimensions();
    let base_min_dim = base_w.min(base_h) as f32;

    let watermark_scale_factor =
        (base_min_dim * (watermark_settings.scale / 100.0)) / watermark_img.width().max(1) as f32;
    let new_wm_w = (watermark_img.width() as f32 * watermark_scale_factor).round() as u32;
    let new_wm_h = (watermark_img.height() as f32 * watermark_scale_factor).round() as u32;

    if new_wm_w == 0 || new_wm_h == 0 {
        return Ok(());
    }

    let scaled_watermark =
        watermark_img.resize_exact(new_wm_w, new_wm_h, image::imageops::FilterType::Lanczos3);
    let mut scaled_watermark_rgba = scaled_watermark.to_rgba8();

    let opacity_factor = (watermark_settings.opacity / 100.0).clamp(0.0, 1.0);
    for pixel in scaled_watermark_rgba.pixels_mut() {
        pixel[3] = (pixel[3] as f32 * opacity_factor) as u8;
    }
    let final_watermark = DynamicImage::ImageRgba8(scaled_watermark_rgba);

    let spacing_pixels = (base_min_dim * (watermark_settings.spacing / 100.0)) as i64;
    let (wm_w, wm_h) = final_watermark.dimensions();

    let x = match watermark_settings.anchor {
        WatermarkAnchor::TopLeft | WatermarkAnchor::CenterLeft | WatermarkAnchor::BottomLeft => {
            spacing_pixels
        }
        WatermarkAnchor::TopCenter | WatermarkAnchor::Center | WatermarkAnchor::BottomCenter => {
            (base_w as i64 - wm_w as i64) / 2
        }
        WatermarkAnchor::TopRight | WatermarkAnchor::CenterRight | WatermarkAnchor::BottomRight => {
            base_w as i64 - wm_w as i64 - spacing_pixels
        }
    };

    let y = match watermark_settings.anchor {
        WatermarkAnchor::TopLeft | WatermarkAnchor::TopCenter | WatermarkAnchor::TopRight => {
            spacing_pixels
        }
        WatermarkAnchor::CenterLeft | WatermarkAnchor::Center | WatermarkAnchor::CenterRight => {
            (base_h as i64 - wm_h as i64) / 2
        }
        WatermarkAnchor::BottomLeft
        | WatermarkAnchor::BottomCenter
        | WatermarkAnchor::BottomRight => base_h as i64 - wm_h as i64 - spacing_pixels,
    };

    image::imageops::overlay(base_image, &final_watermark, x, y);

    Ok(())
}

pub fn get_cached_or_generate_mask(
    state: &tauri::State<'_, AppState>,
    def: &MaskDefinition,
    width: u32,
    height: u32,
    scale: f32,
    crop_offset: (f32, f32),
) -> Option<GrayImage> {
    let mut hasher = DefaultHasher::new();

    let def_json = serde_json::to_string(&def).unwrap_or_default();
    def_json.hash(&mut hasher);

    width.hash(&mut hasher);
    height.hash(&mut hasher);
    scale.to_bits().hash(&mut hasher);
    crop_offset.0.to_bits().hash(&mut hasher);
    crop_offset.1.to_bits().hash(&mut hasher);

    let key = hasher.finish();

    {
        let cache = state.mask_cache.lock().unwrap();
        if let Some(img) = cache.get(&key) {
            return Some(img.clone());
        }
    }

    let generated = generate_mask_bitmap(def, width, height, scale, crop_offset);

    if let Some(img) = &generated {
        let mut cache = state.mask_cache.lock().unwrap();
        if cache.len() > 50 {
            cache.clear();
        }
        cache.insert(key, img.clone());
    }

    generated
}

fn process_preview_job(
    app_handle: &tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    job: PreviewJob,
) -> Result<(), String> {
    let context = get_or_init_gpu_context(&state)?;
    let adjustments_clone = job.adjustments;

    let loaded_image_guard = state.original_image.lock().unwrap();
    let loaded_image = loaded_image_guard
        .as_ref()
        .ok_or("No original image loaded")?
        .clone();
    drop(loaded_image_guard);

    let new_transform_hash = calculate_transform_hash(&adjustments_clone);
    let settings = load_settings(app_handle.clone()).unwrap_or_default();
    let hq_live = settings.enable_high_quality_live_previews.unwrap_or(false);
    let interactive_divisor = if hq_live { 1.5 } else { 2.0 };
    let interactive_quality = if hq_live { 75 } else { 45 };

    let mut cached_preview_lock = state.cached_preview.lock().unwrap();

    let (final_preview_base, small_preview_base, scale_for_gpu, unscaled_crop_offset) =
        if let Some(cached) = &*cached_preview_lock {
            if cached.transform_hash == new_transform_hash {
                (
                    cached.image.clone(),
                    cached.small_image.clone(),
                    cached.scale,
                    cached.unscaled_crop_offset,
                )
            } else {
                *state.gpu_image_cache.lock().unwrap() = None;
                let (base, scale, offset) =
                    generate_transformed_preview(&loaded_image, &adjustments_clone, &app_handle)?;

                let final_preview_dim = settings.editor_preview_resolution.unwrap_or(1920);
                let target_size = (final_preview_dim as f32 / interactive_divisor) as u32;

                let (w, h) = base.dimensions();
                let (small_w, small_h) = if w > h {
                    let ratio = h as f32 / w as f32;
                    (target_size, (target_size as f32 * ratio) as u32)
                } else {
                    let ratio = w as f32 / h as f32;
                    ((target_size as f32 * ratio) as u32, target_size)
                };
                let small_base = image_processing::downscale_f32_image(&base, small_w, small_h);

                *cached_preview_lock = Some(CachedPreview {
                    image: base.clone(),
                    small_image: small_base.clone(),
                    transform_hash: new_transform_hash,
                    scale,
                    unscaled_crop_offset: offset,
                });
                (base, small_base, scale, offset)
            }
        } else {
            *state.gpu_image_cache.lock().unwrap() = None;
            let (base, scale, offset) =
                generate_transformed_preview(&loaded_image, &adjustments_clone, &app_handle)?;

            let final_preview_dim = settings.editor_preview_resolution.unwrap_or(1920);
            let target_size = (final_preview_dim as f32 / interactive_divisor) as u32;

            let (w, h) = base.dimensions();
            let (small_w, small_h) = if w > h {
                let ratio = h as f32 / w as f32;
                (target_size, (target_size as f32 * ratio) as u32)
            } else {
                let ratio = w as f32 / h as f32;
                ((target_size as f32 * ratio) as u32, target_size)
            };
            let small_base = image_processing::downscale_f32_image(&base, small_w, small_h);

            *cached_preview_lock = Some(CachedPreview {
                image: base.clone(),
                small_image: small_base.clone(),
                transform_hash: new_transform_hash,
                scale,
                unscaled_crop_offset: offset,
            });
            (base, small_base, scale, offset)
        };

    drop(cached_preview_lock);

    let (processing_image, effective_scale, jpeg_quality) = if job.is_interactive {
        let orig_w = final_preview_base.width() as f32;
        let small_w = small_preview_base.width() as f32;
        let scale_factor = if orig_w > 0.0 { small_w / orig_w } else { 1.0 };
        let new_scale = scale_for_gpu * scale_factor;
        (small_preview_base, new_scale, interactive_quality)
    } else {
        (final_preview_base, scale_for_gpu, 90)
    };

    let (preview_width, preview_height) = processing_image.dimensions();

    let mask_definitions: Vec<MaskDefinition> = adjustments_clone
        .get("masks")
        .and_then(|m| serde_json::from_value(m.clone()).ok())
        .unwrap_or_else(Vec::new);

    let scaled_crop_offset = (
        unscaled_crop_offset.0 * effective_scale,
        unscaled_crop_offset.1 * effective_scale,
    );

    let mask_bitmaps: Vec<ImageBuffer<Luma<u8>, Vec<u8>>> = mask_definitions
        .iter()
        .filter_map(|def| {
            get_cached_or_generate_mask(
                &state,
                def,
                preview_width,
                preview_height,
                effective_scale,
                scaled_crop_offset,
            )
        })
        .collect();

    let is_raw = loaded_image.is_raw;
    let final_adjustments = get_all_adjustments_from_json(&adjustments_clone, is_raw);
    let lut_path = adjustments_clone["lutPath"].as_str();
    let lut = lut_path.and_then(|p| get_or_load_lut(&state, p).ok());

    let final_processed_image_result = process_and_get_dynamic_image(
        &context,
        &state,
        &processing_image,
        new_transform_hash,
        final_adjustments,
        &mask_bitmaps,
        lut,
        "apply_adjustments",
    );

    if let Ok(final_processed_image) = final_processed_image_result {
        if !job.is_interactive {
            if let Ok(histogram_data) =
                image_processing::calculate_histogram_from_image(&final_processed_image)
            {
                let _ = app_handle.emit("histogram-update", histogram_data);
            }
            if let Ok(waveform_data) =
                image_processing::calculate_waveform_from_image(&final_processed_image)
            {
                let _ = app_handle.emit("waveform-update", waveform_data);
            }
        }

        let mut buf = Cursor::new(Vec::new());
        if final_processed_image
            .to_rgb8()
            .write_with_encoder(JpegEncoder::new_with_quality(&mut buf, jpeg_quality))
            .is_ok()
        {
            let _ = app_handle.emit("preview-update-final", buf.get_ref());
        }
    }

    Ok(())
}

fn start_preview_worker(app_handle: tauri::AppHandle) {
    let state = app_handle.state::<AppState>();
    let (tx, rx): (Sender<PreviewJob>, Receiver<PreviewJob>) = mpsc::channel();

    *state.preview_worker_tx.lock().unwrap() = Some(tx);

    std::thread::spawn(move || {
        while let Ok(mut job) = rx.recv() {
            while let Ok(next_job) = rx.try_recv() {
                job = next_job;
            }

            let state = app_handle.state::<AppState>();
            if let Err(e) = process_preview_job(&app_handle, state, job) {
                log::error!("Preview worker error: {}", e);
            }
        }
    });
}

#[tauri::command]
fn apply_adjustments(
    js_adjustments: serde_json::Value,
    is_interactive: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let tx_guard = state.preview_worker_tx.lock().unwrap();
    if let Some(tx) = &*tx_guard {
        let job = PreviewJob {
            adjustments: js_adjustments,
            is_interactive,
        };
        tx.send(job)
            .map_err(|e| format!("Failed to send to preview worker: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
fn generate_uncropped_preview(
    js_adjustments: serde_json::Value,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let context = get_or_init_gpu_context(&state)?;
    let adjustments_clone = js_adjustments;

    let loaded_image = state
        .original_image
        .lock()
        .unwrap()
        .clone()
        .ok_or("No original image loaded")?;

    thread::spawn(move || {
        let state = app_handle.state::<AppState>();
        let path = loaded_image.path.clone();
        let is_raw = loaded_image.is_raw;
        let unique_hash = calculate_full_job_hash(&path, &adjustments_clone);
        let patched_image = loaded_image.image;

        let orientation_steps = adjustments_clone["orientationSteps"].as_u64().unwrap_or(0) as u8;
        let coarse_rotated_image = apply_coarse_rotation(patched_image, orientation_steps);

        let settings = load_settings(app_handle.clone()).unwrap_or_default();
        let preview_dim = settings.editor_preview_resolution.unwrap_or(1920);

        let (rotated_w, rotated_h) = coarse_rotated_image.dimensions();

        let (processing_base, scale_for_gpu) = if rotated_w > preview_dim || rotated_h > preview_dim
        {
            let base = downscale_f32_image(&coarse_rotated_image, preview_dim, preview_dim);
            let scale = if rotated_w > 0 {
                base.width() as f32 / rotated_w as f32
            } else {
                1.0
            };
            (base, scale)
        } else {
            (coarse_rotated_image.clone(), 1.0)
        };

        let (preview_width, preview_height) = processing_base.dimensions();

        let mask_definitions: Vec<MaskDefinition> = adjustments_clone
            .get("masks")
            .and_then(|m| serde_json::from_value(m.clone()).ok())
            .unwrap_or_else(Vec::new);

        let mask_bitmaps: Vec<ImageBuffer<Luma<u8>, Vec<u8>>> = mask_definitions
            .iter()
            .filter_map(|def| {
                get_cached_or_generate_mask(
                    &state,
                    def,
                    preview_width,
                    preview_height,
                    scale_for_gpu,
                    (0.0, 0.0),
                )
            })
            .collect();

        let uncropped_adjustments = get_all_adjustments_from_json(&adjustments_clone, is_raw);
        let lut_path = adjustments_clone["lutPath"].as_str();
        let lut = lut_path.and_then(|p| get_or_load_lut(&state, p).ok());

        if let Ok(processed_image) = process_and_get_dynamic_image(
            &context,
            &state,
            &processing_base,
            unique_hash,
            uncropped_adjustments,
            &mask_bitmaps,
            lut,
            "generate_uncropped_preview",
        ) {
            let mut buf = Cursor::new(Vec::new());
            if processed_image
                .to_rgb8()
                .write_with_encoder(JpegEncoder::new_with_quality(&mut buf, 80))
                .is_ok()
            {
                let _ = app_handle.emit("preview-update-uncropped", buf.get_ref());
            }
        }
    });

    Ok(())
}

#[tauri::command]
fn generate_original_transformed_preview(
    js_adjustments: serde_json::Value,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<Response, String> {
    let loaded_image = state
        .original_image
        .lock()
        .unwrap()
        .clone()
        .ok_or("No original image loaded")?;

    let adjustments_clone = js_adjustments;

    let mut image_for_preview = loaded_image.image.clone();
    if loaded_image.is_raw {
        apply_cpu_default_raw_processing(&mut image_for_preview);
    }

    let (transformed_full_res, _unscaled_crop_offset) =
        apply_all_transformations(&image_for_preview, &adjustments_clone);

    let settings = load_settings(app_handle).unwrap_or_default();
    let preview_dim = settings.editor_preview_resolution.unwrap_or(1920);

    let (w, h) = transformed_full_res.dimensions();
    let transformed_image = if w > preview_dim || h > preview_dim {
        downscale_f32_image(&transformed_full_res, preview_dim, preview_dim)
    } else {
        transformed_full_res
    };

    let mut buf = Cursor::new(Vec::new());
    transformed_image
        .to_rgb8()
        .write_with_encoder(JpegEncoder::new_with_quality(&mut buf, 80))
        .map_err(|e| e.to_string())?;

    Ok(Response::new(buf.into_inner()))
}

fn get_full_image_for_processing(
    state: &tauri::State<'_, AppState>,
) -> Result<(DynamicImage, bool), String> {
    let original_image_lock = state.original_image.lock().unwrap();
    let loaded_image = original_image_lock
        .as_ref()
        .ok_or("No original image loaded")?;
    Ok((loaded_image.image.clone(), loaded_image.is_raw))
}

#[tauri::command]
fn generate_fullscreen_preview(
    js_adjustments: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<Response, String> {
    let context = get_or_init_gpu_context(&state)?;

    let adjustments_clone = js_adjustments;

    let (original_image, is_raw) = get_full_image_for_processing(&state)?;
    let path = state
        .original_image
        .lock()
        .unwrap()
        .as_ref()
        .ok_or("Original image path not found")?
        .path
        .clone();
    let unique_hash = calculate_full_job_hash(&path, &adjustments_clone);

    let (transformed_image, unscaled_crop_offset) =
        apply_all_transformations(&original_image, &adjustments_clone);
    let (img_w, img_h) = transformed_image.dimensions();

    let mask_definitions: Vec<MaskDefinition> = adjustments_clone
        .get("masks")
        .and_then(|m| serde_json::from_value(m.clone()).ok())
        .unwrap_or_else(Vec::new);

    let mask_bitmaps: Vec<ImageBuffer<Luma<u8>, Vec<u8>>> = mask_definitions
        .iter()
        .filter_map(|def| generate_mask_bitmap(def, img_w, img_h, 1.0, unscaled_crop_offset))
        .collect();

    let all_adjustments = get_all_adjustments_from_json(&adjustments_clone, is_raw);
    let lut_path = adjustments_clone["lutPath"].as_str();
    let lut = lut_path.and_then(|p| get_or_load_lut(&state, p).ok());

    let final_image = process_and_get_dynamic_image(
        &context,
        &state,
        &transformed_image,
        unique_hash,
        all_adjustments,
        &mask_bitmaps,
        lut,
        "generate_fullscreen_preview",
    )?;

    let mut buf = Cursor::new(Vec::new());
    final_image
        .to_rgb8()
        .write_with_encoder(JpegEncoder::new_with_quality(&mut buf, 92))
        .map_err(|e| e.to_string())?;

    Ok(Response::new(buf.into_inner()))
}

fn process_image_for_export(
    path: &str,
    base_image: &DynamicImage,
    js_adjustments: &Value,
    export_settings: &ExportSettings,
    context: &GpuContext,
    state: &tauri::State<'_, AppState>,
    is_raw: bool,
) -> Result<DynamicImage, String> {
    let (transformed_image, unscaled_crop_offset) =
        apply_all_transformations(&base_image, &js_adjustments);
    let (img_w, img_h) = transformed_image.dimensions();

    let mask_definitions: Vec<MaskDefinition> = js_adjustments
        .get("masks")
        .and_then(|m| serde_json::from_value(m.clone()).ok())
        .unwrap_or_else(Vec::new);

    let mask_bitmaps: Vec<ImageBuffer<Luma<u8>, Vec<u8>>> = mask_definitions
        .iter()
        .filter_map(|def| generate_mask_bitmap(def, img_w, img_h, 1.0, unscaled_crop_offset))
        .collect();

    let mut all_adjustments = get_all_adjustments_from_json(&js_adjustments, is_raw);
    all_adjustments.global.show_clipping = 0;

    let lut_path = js_adjustments["lutPath"].as_str();
    let lut = lut_path.and_then(|p| get_or_load_lut(&state, p).ok());

    let unique_hash = calculate_full_job_hash(path, js_adjustments);

    let mut final_image = process_and_get_dynamic_image(
        &context,
        &state,
        &transformed_image,
        unique_hash,
        all_adjustments,
        &mask_bitmaps,
        lut,
        "process_image_for_export",
    )?;

    if let Some(resize_opts) = &export_settings.resize {
        let (current_w, current_h) = final_image.dimensions();
        let should_resize = if resize_opts.dont_enlarge {
            match resize_opts.mode {
                ResizeMode::LongEdge => current_w.max(current_h) > resize_opts.value,
                ResizeMode::ShortEdge => current_w.min(current_h) > resize_opts.value,
                ResizeMode::Width => current_w > resize_opts.value,
                ResizeMode::Height => current_h > resize_opts.value,
            }
        } else {
            true
        };

        if should_resize {
            final_image = match resize_opts.mode {
                ResizeMode::LongEdge => {
                    let (w, h) = if current_w > current_h {
                        (
                            resize_opts.value,
                            (resize_opts.value as f32 * (current_h as f32 / current_w as f32))
                                .round() as u32,
                        )
                    } else {
                        (
                            (resize_opts.value as f32 * (current_w as f32 / current_h as f32))
                                .round() as u32,
                            resize_opts.value,
                        )
                    };
                    final_image.resize(w, h, imageops::FilterType::Lanczos3)
                }
                ResizeMode::ShortEdge => {
                    let (w, h) = if current_w < current_h {
                        (
                            resize_opts.value,
                            (resize_opts.value as f32 * (current_h as f32 / current_w as f32))
                                .round() as u32,
                        )
                    } else {
                        (
                            (resize_opts.value as f32 * (current_w as f32 / current_h as f32))
                                .round() as u32,
                            resize_opts.value,
                        )
                    };
                    final_image.resize(w, h, imageops::FilterType::Lanczos3)
                }
                ResizeMode::Width => {
                    final_image.resize(resize_opts.value, u32::MAX, imageops::FilterType::Lanczos3)
                }
                ResizeMode::Height => {
                    final_image.resize(u32::MAX, resize_opts.value, imageops::FilterType::Lanczos3)
                }
            };
        }
    }

    if let Some(watermark_settings) = &export_settings.watermark {
        apply_watermark(&mut final_image, watermark_settings)?;
    }

    Ok(final_image)
}

fn encode_image_to_bytes(
    image: &DynamicImage,
    output_format: &str,
    jpeg_quality: u8,
) -> Result<Vec<u8>, String> {
    let mut image_bytes = Vec::new();
    let mut cursor = Cursor::new(&mut image_bytes);

    match output_format.to_lowercase().as_str() {
        "jpg" | "jpeg" => {
            let rgb_image = image.to_rgb8();
            let encoder = JpegEncoder::new_with_quality(&mut cursor, jpeg_quality);
            rgb_image
                .write_with_encoder(encoder)
                .map_err(|e| e.to_string())?;
        }
        "png" => {
            let image_to_encode = if image.as_rgb32f().is_some() {
                DynamicImage::ImageRgb16(image.to_rgb16())
            } else {
                image.clone()
            };

            image_to_encode
                .write_to(&mut cursor, image::ImageFormat::Png)
                .map_err(|e| e.to_string())?;
        }
        "tiff" => {
            image
                .write_to(&mut cursor, image::ImageFormat::Tiff)
                .map_err(|e| e.to_string())?;
        }
        _ => return Err(format!("Unsupported file format: {}", output_format)),
    };
    Ok(image_bytes)
}

fn export_photo(
    source_path_str: &str,
    output_path: &Path,
    base_image: &DynamicImage,
    js_adjustments: &Value,
    export_settings: &ExportSettings,
    context: &GpuContext,
    state: &tauri::State<'_, AppState>,
    is_raw: bool,
    cancel_flag: Option<&AtomicBool>,
) -> Result<(), String> {
    if cancel_flag.map(|flag| flag.load(Ordering::SeqCst)).unwrap_or(false) {
        return Err("BACKGROUND_EXPORT_CANCELLED".to_string());
    }

    let final_image = process_image_for_export(
        source_path_str,
        base_image,
        js_adjustments,
        export_settings,
        context,
        state,
        is_raw,
    )?;

    if cancel_flag.map(|flag| flag.load(Ordering::SeqCst)).unwrap_or(false) {
        return Err("BACKGROUND_EXPORT_CANCELLED".to_string());
    }

    let extension = output_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("jpg")
        .to_lowercase();

    let mut image_bytes =
        encode_image_to_bytes(&final_image, &extension, export_settings.jpeg_quality)?;

    write_image_with_metadata(
        &mut image_bytes,
        source_path_str,
        &extension,
        export_settings.keep_metadata,
        export_settings.strip_gps,
    )?;

    if cancel_flag.map(|flag| flag.load(Ordering::SeqCst)).unwrap_or(false) {
        return Err("BACKGROUND_EXPORT_CANCELLED".to_string());
    }

    fs::write(output_path, image_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn export_image(
    original_path: String,
    output_path: String,
    js_adjustments: Value,
    export_settings: ExportSettings,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    if state.export_task_handle.lock().unwrap().is_some() {
        return Err("An export is already in progress.".to_string());
    }

    state.background_export_queue.pause_and_cancel().await;

    let context = get_or_init_gpu_context(&state)?;
    let (original_image_data, is_raw) = get_full_image_for_processing(&state)?;
    let context = Arc::new(context);
    let background_queue = Arc::clone(&state.background_export_queue);

    let task = tokio::spawn(async move {
        let state = app_handle.state::<AppState>();
        let processing_result: Result<(), String> = (|| {
            let (source_path, _) = parse_virtual_path(&original_path);
            let source_path_str = source_path.to_string_lossy().to_string();

            let output_path_obj = std::path::Path::new(&output_path);
            export_photo(
                &source_path_str,
                output_path_obj,
                &original_image_data,
                &js_adjustments,
                &export_settings,
                &context,
                &state,
                is_raw,
                None,
            )?;

            Ok(())
        })();

        if let Err(e) = processing_result {
            let _ = app_handle.emit("export-error", e);
        } else {
            let _ = app_handle.emit("export-complete", ());
        }

        background_queue.resume();
        *app_handle
            .state::<AppState>()
            .export_task_handle
            .lock()
            .unwrap() = None;
    });

    *state.export_task_handle.lock().unwrap() = Some(task);
    Ok(())
}

#[tauri::command]
async fn batch_export_images(
    output_folder: String,
    paths: Vec<String>,
    export_settings: ExportSettings,
    output_format: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    if state.export_task_handle.lock().unwrap().is_some() {
        return Err("An export is already in progress.".to_string());
    }

    state.background_export_queue.pause_and_cancel().await;

    let context = get_or_init_gpu_context(&state)?;
    let context = Arc::new(context);
    let progress_counter = Arc::new(AtomicUsize::new(0));
    let background_queue = Arc::clone(&state.background_export_queue);

    let available_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let num_threads = (available_cores / 2).clamp(1, 4);

    log::info!(
        "Starting batch export. System cores: {}, Export threads: {}",
        available_cores,
        num_threads
    );

    let task = tokio::spawn(async move {
        let state = app_handle.state::<AppState>();
        let output_folder_path = std::path::Path::new(&output_folder);
        let total_paths = paths.len();
        let settings = load_settings(app_handle.clone()).unwrap_or_default();
        let highlight_compression = settings.raw_highlight_compression.unwrap_or(2.5);

        let pool_result = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build();

        if let Err(e) = pool_result {
            let _ = app_handle.emit(
                "export-error",
                format!("Failed to initialize worker threads: {}", e),
            );
            *app_handle
                .state::<AppState>()
                .export_task_handle
                .lock()
                .unwrap() = None;
            return;
        }
        let pool = pool_result.unwrap();

        let results: Vec<Result<(), String>> = pool.install(|| {
            paths
                .par_iter()
                .enumerate()
                .map(|(global_index, image_path_str)| {
                    if app_handle
                        .state::<AppState>()
                        .export_task_handle
                        .lock()
                        .unwrap()
                        .is_none()
                    {
                        return Err("Export cancelled".to_string());
                    }

                    let current_progress = progress_counter.fetch_add(1, Ordering::SeqCst) + 1;

                    let _ = app_handle.emit(
                        "batch-export-progress",
                        serde_json::json!({
                            "current": current_progress,
                            "total": total_paths,
                            "path": image_path_str
                        }),
                    );

                    let result: Result<(), String> = (|| {
                        let (source_path, sidecar_path) = parse_virtual_path(image_path_str);
                        let source_path_str = source_path.to_string_lossy().to_string();

                        let metadata: ImageMetadata = if sidecar_path.exists() {
                            let file_content = fs::read_to_string(sidecar_path)
                                .map_err(|e| format!("Failed to read sidecar: {}", e))?;
                            serde_json::from_str(&file_content).unwrap_or_default()
                        } else {
                            ImageMetadata::default()
                        };
                        let js_adjustments = metadata.adjustments;
                        let is_raw = is_raw_file(&source_path_str);

                        let base_image = match read_file_mapped(Path::new(&source_path_str)) {
                            Ok(mmap) => load_and_composite(
                                &mmap,
                                &source_path_str,
                                &js_adjustments,
                                false,
                                highlight_compression,
                            )
                            .map_err(|e| format!("Failed to load image from mmap: {}", e))?,
                            Err(e) => {
                                log::warn!(
                                    "Failed to memory-map file '{}': {}. Falling back to standard read.",
                                    source_path_str,
                                    e
                                );
                                let bytes = fs::read(&source_path_str).map_err(|io_err| {
                                    format!("Fallback read failed for {}: {}", source_path_str, io_err)
                                })?;
                                load_and_composite(
                                    &bytes,
                                    &source_path_str,
                                    &js_adjustments,
                                    false,
                                    highlight_compression,
                                )
                                .map_err(|e| format!("Failed to load image from bytes: {}", e))?
                            }
                        };

                        let original_path = std::path::Path::new(&source_path_str);

                        let file_date: DateTime<Utc> = {
                            let mut date = None;
                            if let Ok(file) = std::fs::File::open(original_path) {
                                let mut bufreader = std::io::BufReader::new(&file);
                                let exifreader = exif::Reader::new();
                                if let Ok(exif_obj) = exifreader.read_from_container(&mut bufreader) {
                                    if let Some(field) = exif_obj.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) {
                                        let s = field.display_value().to_string().replace("\"", "");
                                        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s.trim(), "%Y:%m:%d %H:%M:%S") {
                                            date = Some(DateTime::from_naive_utc_and_offset(dt, Utc));
                                        }
                                    }
                                }
                            }

                            date.unwrap_or_else(|| {
                                fs::metadata(original_path)
                                    .ok()
                                    .and_then(|m| m.created().ok())
                                    .map(DateTime::<Utc>::from)
                                    .unwrap_or_else(Utc::now)
                            })
                        };

                        let filename_template = export_settings
                            .filename_template
                            .as_deref()
                            .unwrap_or("{original_filename}_edited");
                        let new_stem = crate::file_management::generate_filename_from_template(
                            filename_template,
                            original_path,
                            global_index + 1,
                            total_paths,
                            &file_date,
                        );
                        let new_filename = format!("{}.{}", new_stem, output_format);
                        let output_path = output_folder_path.join(new_filename);

                        export_photo(
                            &source_path_str,
                            &output_path,
                            &base_image,
                            &js_adjustments,
                            &export_settings,
                            &context,
                            &state,
                            is_raw,
                            None,
                        )?;

                        Ok(())
                    })();

                    result
                })
                .collect()
        });

        let mut error_count = 0;
        for result in results {
            if let Err(e) = result {
                error_count += 1;
                log::error!("Batch export error: {}", e);
                let _ = app_handle.emit("export-error", e);
            }
        }

        if error_count > 0 {
            let _ = app_handle.emit(
                "export-complete-with-errors",
                serde_json::json!({ "errors": error_count, "total": total_paths }),
            );
        } else {
            let _ = app_handle.emit(
                "batch-export-progress",
                serde_json::json!({ "current": total_paths, "total": total_paths, "path": "" }),
            );
            let _ = app_handle.emit("export-complete", ());
        }

        background_queue.resume();
        *app_handle
            .state::<AppState>()
            .export_task_handle
            .lock()
            .unwrap() = None;
    });

    *state.export_task_handle.lock().unwrap() = Some(task);
    Ok(())
}

fn start_boothy_batch_export(
    output_folder: String,
    paths: Vec<String>,
    export_settings: ExportSettings,
    output_format: String,
    photo_states: HashMap<String, session::export::BoothyPhotoState>,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    if state.export_task_handle.lock().unwrap().is_some() {
        return Err("An export is already in progress.".to_string());
    }

    let context = get_or_init_gpu_context(&state)?;
    let context = Arc::new(context);
    let progress_counter = Arc::new(AtomicUsize::new(0));
    let progress_state = Arc::new(Mutex::new(ExportProgressState::new(paths.len())));
    let photo_states = Arc::new(photo_states);

    let available_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let num_threads = (available_cores / 2).clamp(1, 4);

    log::info!(
        "Starting Boothy export. System cores: {}, Export threads: {}",
        available_cores,
        num_threads
    );

    let task = tokio::spawn(async move {
        let state = app_handle.state::<AppState>();
        let output_folder_path = std::path::Path::new(&output_folder);
        let total_paths = paths.len();
        let settings = load_settings(app_handle.clone()).unwrap_or_default();
        let highlight_compression = settings.raw_highlight_compression.unwrap_or(2.5);

        {
            let mut progress = progress_state.lock().unwrap();
            progress.advance(0, "");
            let _ = app_handle.emit("boothy-export-progress", progress.to_payload());
        }

        let pool_result = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build();

        if let Err(e) = pool_result {
            {
                let mut progress = progress_state.lock().unwrap();
                progress.mark_error();
            }
            let _ = app_handle.emit(
                "boothy-export-error",
                serde_json::json!({ "message": format!("Failed to initialize worker threads: {}", e) }),
            );
            *app_handle
                .state::<AppState>()
                .export_task_handle
                .lock()
                .unwrap() = None;
            return;
        }
        let pool = pool_result.unwrap();

        let results: Vec<Result<(), String>> = pool.install(|| {
            paths
                .par_iter()
                .enumerate()
                .map(|(global_index, image_path_str)| {
                    if app_handle
                        .state::<AppState>()
                        .export_task_handle
                        .lock()
                        .unwrap()
                        .is_none()
                    {
                        return Err("Export cancelled".to_string());
                    }

                    let filename = std::path::Path::new(image_path_str)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("");
                    let correlation_id = photo_states
                        .get(filename)
                        .and_then(|state| state.correlation_id.as_deref());

                    log::info!(
                        "Boothy export start: {}{}",
                        filename,
                        correlation_id
                            .map(|id| format!(" (correlation_id={})", id))
                            .unwrap_or_default()
                    );

                    {
                        let current_completed = progress_counter.load(Ordering::SeqCst);
                        let mut progress = progress_state.lock().unwrap();
                        progress.advance(current_completed, image_path_str);
                        let _ = app_handle.emit("boothy-export-progress", progress.to_payload());
                    }

                    let result: Result<(), String> = (|| {
                        let (source_path, sidecar_path) = parse_virtual_path(image_path_str);
                        let source_path_str = source_path.to_string_lossy().to_string();

                        let metadata: ImageMetadata = if sidecar_path.exists() {
                            let file_content = fs::read_to_string(sidecar_path)
                                .map_err(|e| format!("Failed to read sidecar: {}", e))?;
                            serde_json::from_str(&file_content).unwrap_or_default()
                        } else {
                            ImageMetadata::default()
                        };
                        let js_adjustments = metadata.adjustments;
                        let is_raw = is_raw_file(&source_path_str);

                        let base_image = match read_file_mapped(Path::new(&source_path_str)) {
                            Ok(mmap) => load_and_composite(
                                &mmap,
                                &source_path_str,
                                &js_adjustments,
                                false,
                                highlight_compression,
                            )
                            .map_err(|e| format!("Failed to load image from mmap: {}", e))?,
                            Err(e) => {
                                log::warn!(
                                    "Failed to memory-map file '{}': {}. Falling back to standard read.",
                                    source_path_str,
                                    e
                                );
                                let bytes = fs::read(&source_path_str).map_err(|io_err| {
                                    format!("Fallback read failed for {}: {}", source_path_str, io_err)
                                })?;
                                load_and_composite(
                                    &bytes,
                                    &source_path_str,
                                    &js_adjustments,
                                    false,
                                    highlight_compression,
                                )
                                .map_err(|e| format!("Failed to load image from bytes: {}", e))?
                            }
                        };

                        let final_image = process_image_for_export(
                            &source_path_str,
                            &base_image,
                            &js_adjustments,
                            &export_settings,
                            &context,
                            &state,
                            is_raw,
                        )?;

                        let original_path = std::path::Path::new(&source_path_str);

                        let file_date: DateTime<Utc> = {
                            let mut date = None;
                            if let Ok(file) = std::fs::File::open(original_path) {
                                let mut bufreader = std::io::BufReader::new(&file);
                                let exifreader = exif::Reader::new();
                                if let Ok(exif_obj) = exifreader.read_from_container(&mut bufreader) {
                                    if let Some(field) = exif_obj.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) {
                                        let s = field.display_value().to_string().replace("\"", "");
                                        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s.trim(), "%Y:%m:%d %H:%M:%S") {
                                            date = Some(DateTime::from_naive_utc_and_offset(dt, Utc));
                                        }
                                    }
                                }
                            }

                            date.unwrap_or_else(|| {
                                fs::metadata(original_path)
                                    .ok()
                                    .and_then(|m| m.created().ok())
                                    .map(DateTime::<Utc>::from)
                                    .unwrap_or_else(Utc::now)
                            })
                        };

                        let filename_template = export_settings
                            .filename_template
                            .as_deref()
                            .unwrap_or("{original_filename}");
                        let new_stem = crate::file_management::generate_filename_from_template(
                            filename_template,
                            original_path,
                            global_index + 1,
                            total_paths,
                            &file_date,
                        );
                        let new_filename = format!("{}.{}", new_stem, output_format);
                        let output_path = output_folder_path.join(new_filename);

                        let mut image_bytes = encode_image_to_bytes(
                            &final_image,
                            &output_format,
                            export_settings.jpeg_quality,
                        )?;

                        write_image_with_metadata(
                            &mut image_bytes,
                            &source_path_str,
                            &output_format,
                            export_settings.keep_metadata,
                            export_settings.strip_gps,
                        )?;

                        fs::write(&output_path, image_bytes)
                            .map_err(|e| format!("Failed to write output: {}", e))?;

                        Ok(())
                    })();

                    let completed = progress_counter.fetch_add(1, Ordering::SeqCst) + 1;
                    {
                        let mut progress = progress_state.lock().unwrap();
                        let current_path = if progress.current_path.is_empty() {
                            image_path_str.to_string()
                        } else {
                            progress.current_path.clone()
                        };
                        progress.advance(completed, current_path);
                        let _ = app_handle.emit("boothy-export-progress", progress.to_payload());
                    }

                    match &result {
                        Ok(_) => log::info!(
                            "Boothy export complete: {}{}",
                            filename,
                            correlation_id
                                .map(|id| format!(" (correlation_id={})", id))
                                .unwrap_or_default()
                        ),
                        Err(err) => log::error!(
                            "Boothy export failed: {}{} -> {}",
                            filename,
                            correlation_id
                                .map(|id| format!(" (correlation_id={})", id))
                                .unwrap_or_default(),
                            err
                        ),
                    }

                    result
                })
                .collect()
        });

        let mut error_count = 0;
        for result in results {
            if let Err(e) = result {
                error_count += 1;
                log::error!("Boothy export error: {}", e);
            }
        }

        if error_count > 0 {
            {
                let mut progress = progress_state.lock().unwrap();
                progress.mark_error();
            }
            let _ = app_handle.emit(
                "boothy-export-error",
                serde_json::json!({
                    "message": format!("Export completed with {} error(s).", error_count),
                }),
            );
        } else {
            {
                let mut progress = progress_state.lock().unwrap();
                progress.mark_complete();
                let _ = app_handle.emit("boothy-export-progress", progress.to_payload());
            }
            let _ = app_handle.emit("boothy-export-complete", ());
        }

        *app_handle
            .state::<AppState>()
            .export_task_handle
            .lock()
            .unwrap() = None;
    });

    *state.export_task_handle.lock().unwrap() = Some(task);
    Ok(())
}

#[tauri::command]
fn cancel_export(state: tauri::State<'_, AppState>) -> Result<(), String> {
    match state.export_task_handle.lock().unwrap().take() {
        Some(handle) => {
            handle.abort();
            println!("Export task cancellation requested.");
        }
        _ => {
            state.background_export_queue.resume();
            return Err("No export task is currently running.".to_string());
        }
    }
    state.background_export_queue.resume();
    Ok(())
}

#[tauri::command]
async fn estimate_export_size(
    js_adjustments: Value,
    export_settings: ExportSettings,
    output_format: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<usize, String> {
    let context = get_or_init_gpu_context(&state)?;
    let loaded_image = state
        .original_image
        .lock()
        .unwrap()
        .clone()
        .ok_or("No original image loaded")?;
    let is_raw = loaded_image.is_raw;

    // HYDRATE (removed for now - not needed for MVP)
    let adjustments_clone = js_adjustments.clone();
    // hydrate_adjustments(&state, &mut adjustments_clone);

    let new_transform_hash = calculate_transform_hash(&adjustments_clone);
    let cached_preview_lock = state.cached_preview.lock().unwrap();

    let (preview_image, scale, unscaled_crop_offset) = if let Some(cached) = &*cached_preview_lock {
        if cached.transform_hash == new_transform_hash {
            (
                cached.image.clone(),
                cached.scale,
                cached.unscaled_crop_offset,
            )
        } else {
            drop(cached_preview_lock);
            let (base, scale, offset) =
                generate_transformed_preview(&loaded_image, &adjustments_clone, &app_handle)?;
            (base, scale, offset)
        }
    } else {
        drop(cached_preview_lock);
        let (base, scale, offset) =
            generate_transformed_preview(&loaded_image, &adjustments_clone, &app_handle)?;
        (base, scale, offset)
    };

    let (img_w, img_h) = preview_image.dimensions();
    let mask_definitions: Vec<MaskDefinition> = adjustments_clone
        .get("masks")
        .and_then(|m| serde_json::from_value(m.clone()).ok())
        .unwrap_or_else(Vec::new);

    let scaled_crop_offset = (
        unscaled_crop_offset.0 * scale,
        unscaled_crop_offset.1 * scale,
    );

    let mask_bitmaps: Vec<ImageBuffer<Luma<u8>, Vec<u8>>> = mask_definitions
        .iter()
        .filter_map(|def| generate_mask_bitmap(def, img_w, img_h, scale, scaled_crop_offset))
        .collect();

    let all_adjustments = get_all_adjustments_from_json(&adjustments_clone, is_raw);
    let lut_path = adjustments_clone["lutPath"].as_str();
    let lut = lut_path.and_then(|p| get_or_load_lut(&state, p).ok());
    let unique_hash =
        calculate_full_job_hash(&loaded_image.path, &adjustments_clone).wrapping_add(1);

    let processed_preview = process_and_get_dynamic_image(
        &context,
        &state,
        &preview_image,
        unique_hash,
        all_adjustments,
        &mask_bitmaps,
        lut,
        "estimate_export_size",
    )?;

    let preview_bytes = encode_image_to_bytes(
        &processed_preview,
        &output_format,
        export_settings.jpeg_quality,
    )?;
    let preview_byte_size = preview_bytes.len();

    let (transformed_full_res, _unscaled_crop_offset) =
        apply_all_transformations(&loaded_image.image, &adjustments_clone);
    let (mut final_full_w, mut final_full_h) = transformed_full_res.dimensions();

    if let Some(resize_opts) = &export_settings.resize {
        let should_resize = if resize_opts.dont_enlarge {
            match resize_opts.mode {
                ResizeMode::LongEdge => final_full_w.max(final_full_h) > resize_opts.value,
                ResizeMode::ShortEdge => final_full_w.min(final_full_h) > resize_opts.value,
                ResizeMode::Width => final_full_w > resize_opts.value,
                ResizeMode::Height => final_full_h > resize_opts.value,
            }
        } else {
            true
        };

        if should_resize {
            match resize_opts.mode {
                ResizeMode::LongEdge => {
                    if final_full_w > final_full_h {
                        final_full_h = (resize_opts.value as f32
                            * (final_full_h as f32 / final_full_w as f32))
                            .round() as u32;
                        final_full_w = resize_opts.value;
                    } else {
                        final_full_w = (resize_opts.value as f32
                            * (final_full_w as f32 / final_full_h as f32))
                            .round() as u32;
                        final_full_h = resize_opts.value;
                    }
                }
                ResizeMode::ShortEdge => {
                    if final_full_w < final_full_h {
                        final_full_h = (resize_opts.value as f32
                            * (final_full_h as f32 / final_full_w as f32))
                            .round() as u32;
                        final_full_w = resize_opts.value;
                    } else {
                        final_full_w = (resize_opts.value as f32
                            * (final_full_w as f32 / final_full_h as f32))
                            .round() as u32;
                        final_full_h = resize_opts.value;
                    }
                }
                ResizeMode::Width => {
                    final_full_h = (resize_opts.value as f32
                        * (final_full_h as f32 / final_full_w as f32))
                        .round() as u32;
                    final_full_w = resize_opts.value;
                }
                ResizeMode::Height => {
                    final_full_w = (resize_opts.value as f32
                        * (final_full_w as f32 / final_full_h as f32))
                        .round() as u32;
                    final_full_h = resize_opts.value;
                }
            };
        }
    }

    let (processed_preview_w, processed_preview_h) = processed_preview.dimensions();

    let pixel_ratio = if processed_preview_w > 0 && processed_preview_h > 0 {
        (final_full_w as f64 * final_full_h as f64)
            / (processed_preview_w as f64 * processed_preview_h as f64)
    } else {
        1.0
    };

    let estimated_size = (preview_byte_size as f64 * pixel_ratio) as usize;

    Ok(estimated_size)
}

#[tauri::command]
async fn estimate_batch_export_size(
    paths: Vec<String>,
    export_settings: ExportSettings,
    output_format: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<usize, String> {
    if paths.is_empty() {
        return Ok(0);
    }
    let context = get_or_init_gpu_context(&state)?;
    let first_path = &paths[0];
    let (source_path, sidecar_path) = parse_virtual_path(first_path);
    let source_path_str = source_path.to_string_lossy().to_string();
    let is_raw = is_raw_file(&source_path_str);

    let metadata: ImageMetadata = if sidecar_path.exists() {
        let file_content = fs::read_to_string(sidecar_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&file_content).unwrap_or_default()
    } else {
        ImageMetadata::default()
    };
    let js_adjustments = metadata.adjustments;

    let settings = load_settings(app_handle.clone()).unwrap_or_default();
    let highlight_compression = settings.raw_highlight_compression.unwrap_or(2.5);

    const ESTIMATE_DIM: u32 = 1280;

    let original_image = match read_file_mapped(Path::new(&source_path_str)) {
        Ok(mmap) => {
            load_base_image_from_bytes(&mmap, &source_path_str, true, highlight_compression)
                .map_err(|e| e.to_string())?
        }
        Err(e) => {
            log::warn!(
                "Failed to memory-map file '{}': {}. Falling back to standard read.",
                source_path_str,
                e
            );
            let bytes = fs::read(&source_path_str).map_err(|io_err| io_err.to_string())?;
            load_base_image_from_bytes(&bytes, &source_path_str, true, highlight_compression)
                .map_err(|e| e.to_string())?
        }
    };

    let base_image_preview = downscale_f32_image(&original_image, ESTIMATE_DIM, ESTIMATE_DIM);

    let (transformed_preview, unscaled_crop_offset) =
        apply_all_transformations(&base_image_preview, &js_adjustments);
    let (preview_w, preview_h) = transformed_preview.dimensions();

    let mask_definitions: Vec<MaskDefinition> = js_adjustments
        .get("masks")
        .and_then(|m| serde_json::from_value(m.clone()).ok())
        .unwrap_or_else(Vec::new);

    let mask_bitmaps: Vec<ImageBuffer<Luma<u8>, Vec<u8>>> = mask_definitions
        .iter()
        .filter_map(|def| {
            generate_mask_bitmap(def, preview_w, preview_h, 1.0, unscaled_crop_offset)
        })
        .collect();

    let mut all_adjustments = get_all_adjustments_from_json(&js_adjustments, is_raw);
    all_adjustments.global.show_clipping = 0;

    let lut_path = js_adjustments["lutPath"].as_str();
    let lut = lut_path.and_then(|p| get_or_load_lut(&state, p).ok());

    let unique_hash = calculate_full_job_hash(&source_path_str, &js_adjustments).wrapping_add(1);

    let processed_preview = process_and_get_dynamic_image(
        &context,
        &state,
        &transformed_preview,
        unique_hash,
        all_adjustments,
        &mask_bitmaps,
        lut,
        "estimate_batch_export_size",
    )?;

    let preview_bytes = encode_image_to_bytes(
        &processed_preview,
        &output_format,
        export_settings.jpeg_quality,
    )?;
    let single_image_estimated_size = preview_bytes.len();

    let (transformed_full_res, _unscaled_crop_offset) =
        apply_all_transformations(&original_image, &js_adjustments);
    let (mut final_full_w, mut final_full_h) = transformed_full_res.dimensions();

    if let Some(resize_opts) = &export_settings.resize {
        let should_resize = if resize_opts.dont_enlarge {
            match resize_opts.mode {
                ResizeMode::LongEdge => final_full_w.max(final_full_h) > resize_opts.value,
                ResizeMode::ShortEdge => final_full_w.min(final_full_h) > resize_opts.value,
                ResizeMode::Width => final_full_w > resize_opts.value,
                ResizeMode::Height => final_full_h > resize_opts.value,
            }
        } else {
            true
        };

        if should_resize {
            match resize_opts.mode {
                ResizeMode::LongEdge => {
                    if final_full_w > final_full_h {
                        final_full_h = (resize_opts.value as f32
                            * (final_full_h as f32 / final_full_w as f32))
                            .round() as u32;
                        final_full_w = resize_opts.value;
                    } else {
                        final_full_w = (resize_opts.value as f32
                            * (final_full_w as f32 / final_full_h as f32))
                            .round() as u32;
                        final_full_h = resize_opts.value;
                    }
                }
                ResizeMode::ShortEdge => {
                    if final_full_w < final_full_h {
                        final_full_h = (resize_opts.value as f32
                            * (final_full_h as f32 / final_full_w as f32))
                            .round() as u32;
                        final_full_w = resize_opts.value;
                    } else {
                        final_full_w = (resize_opts.value as f32
                            * (final_full_w as f32 / final_full_h as f32))
                            .round() as u32;
                        final_full_h = resize_opts.value;
                    }
                }
                ResizeMode::Width => {
                    final_full_h = (resize_opts.value as f32
                        * (final_full_h as f32 / final_full_w as f32))
                        .round() as u32;
                    final_full_w = resize_opts.value;
                }
                ResizeMode::Height => {
                    final_full_w = (resize_opts.value as f32
                        * (final_full_w as f32 / final_full_h as f32))
                        .round() as u32;
                    final_full_h = resize_opts.value;
                }
            };
        }
    }

    let (processed_preview_w, processed_preview_h) = processed_preview.dimensions();

    let pixel_ratio = if processed_preview_w > 0 && processed_preview_h > 0 {
        (final_full_w as f64 * final_full_h as f64)
            / (processed_preview_w as f64 * processed_preview_h as f64)
    } else {
        1.0
    };

    let single_image_extrapolated_size =
        (single_image_estimated_size as f64 * pixel_ratio) as usize;

    Ok(single_image_extrapolated_size * paths.len())
}

fn write_image_with_metadata(
    image_bytes: &mut Vec<u8>,
    original_path_str: &str,
    output_format: &str,
    keep_metadata: bool,
    strip_gps: bool,
) -> Result<(), String> {
    if !keep_metadata || output_format.to_lowercase() == "tiff" {
        // FIXME: temporary solution until I find a way to write metadata to TIFF
        return Ok(());
    }

    let original_path = std::path::Path::new(original_path_str);
    if !original_path.exists() {
        return Ok(());
    }

    // Skip TIFF sources to avoid potential tag corruption issues
    let original_ext = original_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if original_ext == "tiff" || original_ext == "tif" {
        return Ok(());
    }

    let file_type = match output_format.to_lowercase().as_str() {
        "jpg" | "jpeg" => FileExtension::JPEG,
        "png" => FileExtension::PNG {
            as_zTXt_chunk: true,
        },
        "tiff" => FileExtension::TIFF,
        _ => return Ok(()),
    };

    let mut metadata = Metadata::new();

    if let Ok(file) = std::fs::File::open(original_path) {
        let mut bufreader = std::io::BufReader::new(&file);
        let exifreader = exif::Reader::new();

        if let Ok(exif_obj) = exifreader.read_from_container(&mut bufreader) {
            use little_exif::rational::{iR64, uR64};

            let to_ur64 = |val: &exif::Rational| -> uR64 {
                uR64 {
                    nominator: val.num,
                    denominator: val.denom,
                }
            };

            let to_ir64 = |val: &exif::SRational| -> iR64 {
                iR64 {
                    nominator: val.num,
                    denominator: val.denom,
                }
            };

            let get_string_val = |field: &exif::Field| -> String {
                match &field.value {
                    exif::Value::Ascii(vec) => vec
                        .iter()
                        .map(|v| {
                            String::from_utf8_lossy(v)
                                .trim_matches(char::from(0))
                                .to_string()
                        })
                        .collect::<Vec<String>>()
                        .join(" "),
                    _ => field
                        .display_value()
                        .to_string()
                        .replace("\"", "")
                        .trim()
                        .to_string(),
                }
            };

            if let Some(f) = exif_obj.get_field(exif::Tag::Make, exif::In::PRIMARY) {
                metadata.set_tag(ExifTag::Make(get_string_val(f)));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::Model, exif::In::PRIMARY) {
                metadata.set_tag(ExifTag::Model(get_string_val(f)));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::LensMake, exif::In::PRIMARY) {
                metadata.set_tag(ExifTag::LensMake(get_string_val(f)));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::LensModel, exif::In::PRIMARY) {
                metadata.set_tag(ExifTag::LensModel(get_string_val(f)));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::Artist, exif::In::PRIMARY) {
                metadata.set_tag(ExifTag::Artist(get_string_val(f)));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::Copyright, exif::In::PRIMARY) {
                metadata.set_tag(ExifTag::Copyright(get_string_val(f)));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) {
                metadata.set_tag(ExifTag::DateTimeOriginal(get_string_val(f)));
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::DateTime, exif::In::PRIMARY) {
                metadata.set_tag(ExifTag::CreateDate(get_string_val(f)));
            }

            if let Some(f) = exif_obj.get_field(exif::Tag::FNumber, exif::In::PRIMARY) {
                if let exif::Value::Rational(v) = &f.value {
                    if !v.is_empty() {
                        metadata.set_tag(ExifTag::FNumber(vec![to_ur64(&v[0])]));
                    }
                }
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::ExposureTime, exif::In::PRIMARY) {
                if let exif::Value::Rational(v) = &f.value {
                    if !v.is_empty() {
                        metadata.set_tag(ExifTag::ExposureTime(vec![to_ur64(&v[0])]));
                    }
                }
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::FocalLength, exif::In::PRIMARY) {
                if let exif::Value::Rational(v) = &f.value {
                    if !v.is_empty() {
                        metadata.set_tag(ExifTag::FocalLength(vec![to_ur64(&v[0])]));
                    }
                }
            }

            if let Some(f) = exif_obj.get_field(exif::Tag::ExposureBiasValue, exif::In::PRIMARY) {
                match &f.value {
                    exif::Value::SRational(v) if !v.is_empty() => {
                        metadata.set_tag(ExifTag::ExposureCompensation(vec![to_ir64(&v[0])]));
                    }
                    exif::Value::Rational(v) if !v.is_empty() => {
                        metadata.set_tag(ExifTag::ExposureCompensation(vec![iR64 {
                            nominator: v[0].num as i32,
                            denominator: v[0].denom as i32,
                        }]));
                    }
                    _ => {}
                }
            }

            if let Some(f) =
                exif_obj.get_field(exif::Tag::PhotographicSensitivity, exif::In::PRIMARY)
            {
                if let Some(val) = f.value.get_uint(0) {
                    metadata.set_tag(ExifTag::ISO(vec![val as u16]));
                }
            } else if let Some(f) = exif_obj.get_field(exif::Tag::ISOSpeed, exif::In::PRIMARY) {
                if let Some(val) = f.value.get_uint(0) {
                    metadata.set_tag(ExifTag::ISO(vec![val as u16]));
                }
            }

            if let Some(f) = exif_obj.get_field(exif::Tag::Flash, exif::In::PRIMARY) {
                if let Some(val) = f.value.get_uint(0) {
                    metadata.set_tag(ExifTag::Flash(vec![val as u16]));
                }
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::MeteringMode, exif::In::PRIMARY) {
                if let Some(val) = f.value.get_uint(0) {
                    metadata.set_tag(ExifTag::MeteringMode(vec![val as u16]));
                }
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::WhiteBalance, exif::In::PRIMARY) {
                if let Some(val) = f.value.get_uint(0) {
                    metadata.set_tag(ExifTag::WhiteBalance(vec![val as u16]));
                }
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::ExposureProgram, exif::In::PRIMARY) {
                if let Some(val) = f.value.get_uint(0) {
                    metadata.set_tag(ExifTag::ExposureProgram(vec![val as u16]));
                }
            }
            if let Some(f) = exif_obj.get_field(exif::Tag::FocalLengthIn35mmFilm, exif::In::PRIMARY)
            {
                if let Some(val) = f.value.get_uint(0) {
                    metadata.set_tag(ExifTag::FocalLengthIn35mmFormat(vec![val as u16]));
                }
            }

            if !strip_gps {
                if let Some(f) = exif_obj.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY) {
                    if let exif::Value::Rational(v) = &f.value {
                        if v.len() >= 3 {
                            metadata.set_tag(ExifTag::GPSLatitude(vec![
                                to_ur64(&v[0]),
                                to_ur64(&v[1]),
                                to_ur64(&v[2]),
                            ]));
                        }
                    }
                }
                if let Some(f) = exif_obj.get_field(exif::Tag::GPSLatitudeRef, exif::In::PRIMARY) {
                    metadata.set_tag(ExifTag::GPSLatitudeRef(get_string_val(f)));
                }
                if let Some(f) = exif_obj.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY) {
                    if let exif::Value::Rational(v) = &f.value {
                        if v.len() >= 3 {
                            metadata.set_tag(ExifTag::GPSLongitude(vec![
                                to_ur64(&v[0]),
                                to_ur64(&v[1]),
                                to_ur64(&v[2]),
                            ]));
                        }
                    }
                }
                if let Some(f) = exif_obj.get_field(exif::Tag::GPSLongitudeRef, exif::In::PRIMARY) {
                    metadata.set_tag(ExifTag::GPSLongitudeRef(get_string_val(f)));
                }
                if let Some(f) = exif_obj.get_field(exif::Tag::GPSAltitude, exif::In::PRIMARY) {
                    if let exif::Value::Rational(v) = &f.value {
                        if !v.is_empty() {
                            metadata.set_tag(ExifTag::GPSAltitude(vec![to_ur64(&v[0])]));
                        }
                    }
                }
                if let Some(f) = exif_obj.get_field(exif::Tag::GPSAltitudeRef, exif::In::PRIMARY) {
                    if let Some(val) = f.value.get_uint(0) {
                        metadata.set_tag(ExifTag::GPSAltitudeRef(vec![val as u8]));
                    }
                }
            }
        }
    }

    metadata.set_tag(ExifTag::Software("RapidRAW".to_string()));
    metadata.set_tag(ExifTag::Orientation(vec![1u16]));
    metadata.set_tag(ExifTag::ColorSpace(vec![1u16]));

    // little_exif has a bug where writing a Metadata object causes a panic, even if you do everything else right - see https://github.com/TechnikTobi/little_exif/issues/76
    let write_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        metadata.write_to_vec(image_bytes, file_type)
    }));

    match write_result {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => log::warn!("Failed to write metadata: {}", e),
        Err(_) => {
            log::error!("Recovered from little_exif library panic. Saving image without metadata.")
        }
    }

    Ok(())
}

#[tauri::command]
fn generate_mask_overlay(
    mask_def: MaskDefinition,
    width: u32,
    height: u32,
    scale: f32,
    crop_offset: (f32, f32),
) -> Result<String, String> {
    let scaled_crop_offset = (crop_offset.0 * scale, crop_offset.1 * scale);

    if let Some(gray_mask) =
        generate_mask_bitmap(&mask_def, width, height, scale, scaled_crop_offset)
    {
        let mut rgba_mask = RgbaImage::new(width, height);
        for (x, y, pixel) in gray_mask.enumerate_pixels() {
            let intensity = pixel[0];
            let alpha = (intensity as f32 * 0.5) as u8;
            rgba_mask.put_pixel(x, y, Rgba([255, 0, 0, alpha]));
        }

        let mut buf = Cursor::new(Vec::new());
        rgba_mask
            .write_to(&mut buf, ImageFormat::Png)
            .map_err(|e| e.to_string())?;

        let base64_str = general_purpose::STANDARD.encode(buf.get_ref());
        let data_url = format!("data:image/png;base64,{}", base64_str);

        Ok(data_url)
    } else {
        Ok("".to_string())
    }
}

#[tauri::command]
fn generate_preset_preview(
    js_adjustments: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<Response, String> {
    let context = get_or_init_gpu_context(&state)?;

    let loaded_image = state
        .original_image
        .lock()
        .unwrap()
        .clone()
        .ok_or("No original image loaded for preset preview")?;
    let original_image = loaded_image.image;
    let path = loaded_image.path;
    let is_raw = loaded_image.is_raw;
    let unique_hash = calculate_full_job_hash(&path, &js_adjustments);

    const PRESET_PREVIEW_DIM: u32 = 200;
    let preview_base = downscale_f32_image(&original_image, PRESET_PREVIEW_DIM, PRESET_PREVIEW_DIM);

    let (transformed_image, unscaled_crop_offset) =
        apply_all_transformations(&preview_base, &js_adjustments);
    let (img_w, img_h) = transformed_image.dimensions();

    let mask_definitions: Vec<MaskDefinition> = js_adjustments
        .get("masks")
        .and_then(|m| serde_json::from_value(m.clone()).ok())
        .unwrap_or_else(Vec::new);

    let mask_bitmaps: Vec<ImageBuffer<Luma<u8>, Vec<u8>>> = mask_definitions
        .iter()
        .filter_map(|def| generate_mask_bitmap(def, img_w, img_h, 1.0, unscaled_crop_offset))
        .collect();

    let all_adjustments = get_all_adjustments_from_json(&js_adjustments, is_raw);
    let lut_path = js_adjustments["lutPath"].as_str();
    let lut = lut_path.and_then(|p| get_or_load_lut(&state, p).ok());

    let processed_image = process_and_get_dynamic_image(
        &context,
        &state,
        &transformed_image,
        unique_hash,
        all_adjustments,
        &mask_bitmaps,
        lut,
        "generate_preset_preview",
    )?;

    let mut buf = Cursor::new(Vec::new());
    processed_image
        .to_rgb8()
        .write_with_encoder(JpegEncoder::new_with_quality(&mut buf, 50))
        .map_err(|e| e.to_string())?;

    Ok(Response::new(buf.into_inner()))
}

#[tauri::command]
fn update_window_effect(theme: String, window: tauri::Window) {
    apply_window_effect(theme, window);
}

#[tauri::command]
fn get_supported_file_types() -> Result<serde_json::Value, String> {
    let raw_extensions: Vec<&str> = crate::formats::RAW_EXTENSIONS
        .iter()
        .map(|(ext, _)| *ext)
        .collect();
    let non_raw_extensions: Vec<&str> = crate::formats::NON_RAW_EXTENSIONS.to_vec();

    Ok(serde_json::json!({
        "raw": raw_extensions,
        "nonRaw": non_raw_extensions
    }))
}

#[tauri::command]
async fn save_temp_file(bytes: Vec<u8>) -> Result<String, String> {
    let mut temp_file = NamedTempFile::new().map_err(|e| e.to_string())?;
    temp_file.write_all(&bytes).map_err(|e| e.to_string())?;
    let (_file, path) = temp_file.keep().map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
async fn stitch_panorama(
    paths: Vec<String>,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    if paths.len() < 2 {
        return Err("Please select at least two images to stitch.".to_string());
    }

    let source_paths: Vec<String> = paths
        .iter()
        .map(|p| parse_virtual_path(p).0.to_string_lossy().into_owned())
        .collect();

    let panorama_result_handle = state.panorama_result.clone();

    let task = tokio::task::spawn_blocking(move || {
        let panorama_result = panorama_stitching::stitch_images(source_paths, app_handle.clone());

        match panorama_result {
            Ok(panorama_image) => {
                let _ = app_handle.emit("panorama-progress", "Creating preview...");

                let (w, h) = panorama_image.dimensions();
                let (new_w, new_h) = if w > h {
                    (800, (800.0 * h as f32 / w as f32).round() as u32)
                } else {
                    ((800.0 * w as f32 / h as f32).round() as u32, 800)
                };

                let preview_f32 =
                    crate::image_processing::downscale_f32_image(&panorama_image, new_w, new_h);

                let preview_u8 = preview_f32.to_rgb8();

                let mut buf = Cursor::new(Vec::new());

                if let Err(e) = preview_u8.write_to(&mut buf, ImageFormat::Png) {
                    return Err(format!("Failed to encode panorama preview: {}", e));
                }

                let base64_str = general_purpose::STANDARD.encode(buf.get_ref());
                let final_base64 = format!("data:image/png;base64,{}", base64_str);

                *panorama_result_handle.lock().unwrap() = Some(panorama_image);

                let _ = app_handle.emit(
                    "panorama-complete",
                    serde_json::json!({
                        "base64": final_base64,
                    }),
                );
                Ok(())
            }
            Err(e) => {
                let _ = app_handle.emit("panorama-error", e.clone());
                Err(e)
            }
        }
    });

    match task.await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(join_err) => Err(format!("Panorama task failed: {}", join_err)),
    }
}

#[tauri::command]
async fn save_panorama(
    first_path_str: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let panorama_image = state
        .panorama_result
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| {
            "No panorama image found in memory to save. It might have already been saved."
                .to_string()
        })?;

    let (first_path, _) = parse_virtual_path(&first_path_str);
    let parent_dir = first_path
        .parent()
        .ok_or_else(|| "Could not determine parent directory of the first image.".to_string())?;
    let stem = first_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("panorama");

    let (output_filename, image_to_save): (String, DynamicImage) =
        if panorama_image.color().has_alpha() {
            (
                format!("{}_Pano.png", stem),
                DynamicImage::ImageRgba8(panorama_image.to_rgba8()),
            )
        } else if panorama_image.as_rgb32f().is_some() {
            (format!("{}_Pano.tiff", stem), panorama_image)
        } else {
            (
                format!("{}_Pano.png", stem),
                DynamicImage::ImageRgb8(panorama_image.to_rgb8()),
            )
        };

    let output_path = parent_dir.join(output_filename);

    image_to_save
        .save(&output_path)
        .map_err(|e| format!("Failed to save panorama image: {}", e))?;

    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
async fn apply_denoising(
    path: String,
    intensity: f32,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let (source_path, _) = parse_virtual_path(&path);
    let path_str = source_path.to_string_lossy().to_string();

    let denoise_result_handle = state.denoise_result.clone();

    tokio::task::spawn_blocking(move || {
        match denoising::denoise_image(path_str, intensity, app_handle.clone()) {
            Ok((image, _base64_ignored_in_this_handler_logic)) => {
                *denoise_result_handle.lock().unwrap() = Some(image);
            }
            Err(e) => {
                let _ = app_handle.emit("denoise-error", e);
            }
        }
    })
    .await
    .map_err(|e| format!("Denoising task failed: {}", e))
}

#[tauri::command]
async fn save_denoised_image(
    original_path_str: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let denoised_image = state.denoise_result.lock().unwrap().take().ok_or_else(|| {
        "No denoised image found in memory. It might have already been saved or cleared."
            .to_string()
    })?;

    let is_raw = crate::formats::is_raw_file(&original_path_str);

    let (first_path, _) = parse_virtual_path(&original_path_str);
    let parent_dir = first_path
        .parent()
        .ok_or_else(|| "Could not determine parent directory.".to_string())?;
    let stem = first_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("denoised");

    let (output_filename, image_to_save): (String, DynamicImage) = if is_raw {
        let filename = format!("{}_Denoised.tiff", stem);
        (filename, denoised_image)
    } else {
        let filename = format!("{}_Denoised.png", stem);
        (filename, DynamicImage::ImageRgb8(denoised_image.to_rgb8()))
    };

    let output_path = parent_dir.join(output_filename);

    image_to_save
        .save(&output_path)
        .map_err(|e| format!("Failed to save image: {}", e))?;

    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
async fn save_collage(base64_data: String, first_path_str: String) -> Result<String, String> {
    let data_url_prefix = "data:image/png;base64,";
    if !base64_data.starts_with(data_url_prefix) {
        return Err("Invalid base64 data format".to_string());
    }
    let encoded_data = &base64_data[data_url_prefix.len()..];

    let decoded_bytes = general_purpose::STANDARD
        .decode(encoded_data)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    let (first_path, _) = parse_virtual_path(&first_path_str);
    let parent_dir = first_path
        .parent()
        .ok_or_else(|| "Could not determine parent directory of the first image.".to_string())?;
    let stem = first_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("collage");

    let output_filename = format!("{}_Collage.png", stem);
    let output_path = parent_dir.join(output_filename);

    fs::write(&output_path, &decoded_bytes)
        .map_err(|e| format!("Failed to save collage image: {}", e))?;

    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
fn generate_preview_for_path(
    path: String,
    js_adjustments: Value,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<Response, String> {
    let context = get_or_init_gpu_context(&state)?;
    let (source_path, _) = parse_virtual_path(&path);
    let source_path_str = source_path.to_string_lossy().to_string();
    let is_raw = is_raw_file(&source_path_str);
    let settings = load_settings(app_handle.clone()).unwrap_or_default();
    let highlight_compression = settings.raw_highlight_compression.unwrap_or(2.5);

    let base_image = match read_file_mapped(&source_path) {
        Ok(mmap) => load_and_composite(
            &mmap,
            &source_path_str,
            &js_adjustments,
            false,
            highlight_compression,
        )
        .map_err(|e| e.to_string())?,
        Err(e) => {
            log::warn!(
                "Failed to memory-map file '{}': {}. Falling back to standard read.",
                source_path_str,
                e
            );
            let bytes = fs::read(&source_path).map_err(|io_err| io_err.to_string())?;
            load_and_composite(
                &bytes,
                &source_path_str,
                &js_adjustments,
                false,
                highlight_compression,
            )
            .map_err(|e| e.to_string())?
        }
    };

    let (transformed_image, unscaled_crop_offset) =
        apply_all_transformations(&base_image, &js_adjustments);
    let (img_w, img_h) = transformed_image.dimensions();
    let mask_definitions: Vec<MaskDefinition> = js_adjustments
        .get("masks")
        .and_then(|m| serde_json::from_value(m.clone()).ok())
        .unwrap_or_else(Vec::new);
    let mask_bitmaps: Vec<ImageBuffer<Luma<u8>, Vec<u8>>> = mask_definitions
        .iter()
        .filter_map(|def| generate_mask_bitmap(def, img_w, img_h, 1.0, unscaled_crop_offset))
        .collect();
    let all_adjustments = get_all_adjustments_from_json(&js_adjustments, is_raw);
    let lut_path = js_adjustments["lutPath"].as_str();
    let lut = lut_path.and_then(|p| get_or_load_lut(&state, p).ok());
    let unique_hash = calculate_full_job_hash(&source_path_str, &js_adjustments);
    let final_image = process_and_get_dynamic_image(
        &context,
        &state,
        &transformed_image,
        unique_hash,
        all_adjustments,
        &mask_bitmaps,
        lut,
        "generate_preview_for_path",
    )?;
    let mut buf = Cursor::new(Vec::new());
    final_image
        .to_rgb8()
        .write_with_encoder(JpegEncoder::new_with_quality(&mut buf, 92))
        .map_err(|e| e.to_string())?;

    Ok(Response::new(buf.into_inner()))
}

#[tauri::command]
async fn load_and_parse_lut(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<LutParseResult, String> {
    let lut = lut_processing::parse_lut_file(&path).map_err(|e| e.to_string())?;
    let lut_size = lut.size;

    let mut cache = state.lut_cache.lock().unwrap();
    cache.insert(path, Arc::new(lut));

    Ok(LutParseResult { size: lut_size })
}

fn apply_window_effect(theme: String, window: impl raw_window_handle::HasWindowHandle) {
    #[cfg(target_os = "windows")]
    {
        let color = match theme.as_str() {
            "light" => Some((250, 250, 250, 150)),
            "muted-green" => Some((44, 56, 54, 100)),
            _ => Some((26, 29, 27, 60)),
        };

        let info = os_info::get();

        let is_win11_or_newer = match info.version() {
            os_info::Version::Semantic(major, _, build) => *major == 10 && *build >= 22000,
            _ => false,
        };

        if is_win11_or_newer {
            window_vibrancy::apply_acrylic(&window, color)
                .expect("Failed to apply acrylic effect on Windows 11");
        } else {
            window_vibrancy::apply_blur(&window, color)
                .expect("Failed to apply blur effect on Windows 10 or older");
        }
    }

    #[cfg(target_os = "macos")]
    {
        let material = match theme.as_str() {
            "light" => window_vibrancy::NSVisualEffectMaterial::ContentBackground,
            _ => window_vibrancy::NSVisualEffectMaterial::HudWindow,
        };
        window_vibrancy::apply_vibrancy(&window, material, None, None)
            .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");
    }

    #[cfg(target_os = "linux")]
    {
        let _ = (theme, window);
    }
}

fn setup_logging(app_handle: &tauri::AppHandle, logging_initialized: bool) {
    if logging_initialized {
        if let Ok(log_dir) = logging::get_log_directory() {
            log::info!("Using existing logger. Log directory: {:?}", log_dir);
        }
    } else {
        let log_dir = match app_handle.path().app_log_dir() {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("Failed to get app log directory: {}", e);
                return;
            }
        };

        if let Err(e) = fs::create_dir_all(&log_dir) {
            eprintln!("Failed to create log directory at {:?}: {}", log_dir, e);
        }

        let log_file_path = log_dir.join("app.log");

        let log_file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(&log_file_path)
            .ok();

        let var = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
        let level: log::LevelFilter = var.parse().unwrap_or(log::LevelFilter::Info);

        let mut dispatch = fern::Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "{} [{}] {}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    record.level(),
                    message
                ))
            })
            .level(level)
            .chain(std::io::stderr());

        if let Some(file) = log_file {
            dispatch = dispatch.chain(file);
        } else {
            eprintln!(
                "Failed to open log file at {:?}. Logging to console only.",
                log_file_path
            );
        }

        if let Err(e) = dispatch.apply() {
            eprintln!("Failed to apply logger configuration: {}", e);
        } else {
            log::info!(
                "Logger initialized successfully. Log file at: {:?}",
                log_file_path
            );
        }
    }
    panic::set_hook(Box::new(|info| {
        let message = if let Some(s) = info.payload().downcast_ref::<&'static str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            format!("{:?}", info.payload())
        };
        let location = info.location().map_or_else(
            || "at an unknown location".to_string(),
            |loc| format!("at {}:{}:{}", loc.file(), loc.line(), loc.column()),
        );
        log::error!("PANIC! {} - {}", location, message.trim());
    }));
}

#[tauri::command]
fn get_log_file_path(app_handle: tauri::AppHandle) -> Result<String, String> {
    if let Ok(log_dir) = logging::get_log_directory() {
        let log_file_path = log_dir.join(format!(
            "boothy-{}.log",
            chrono::Local::now().format("%Y%m%d")
        ));
        return Ok(log_file_path.to_string_lossy().to_string());
    }

    let log_dir = app_handle.path().app_log_dir().map_err(|e| e.to_string())?;
    let log_file_path = log_dir.join("app.log");
    Ok(log_file_path.to_string_lossy().to_string())
}

fn handle_file_open(app_handle: &tauri::AppHandle, path: PathBuf) {
    if let Some(path_str) = path.to_str() {
        if let Err(e) = app_handle.emit("open-with-file", path_str) {
            log::error!("Failed to emit open-with-file event: {}", e);
        }
    }
}

#[tauri::command]
fn frontend_ready(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    log::info!("Frontend ready signal received.");
    if let Some(path) = state.initial_file_path.lock().unwrap().take() {
        log::info!(
            "Frontend is ready, emitting open-with-file for initial path: {}",
            &path
        );
        handle_file_open(&app_handle, PathBuf::from(path));
    }
    Ok(())
}

#[tauri::command]
fn boothy_log_frontend(level: String, message: String, context: Option<Value>) -> Result<(), String> {
    let context_str = context
        .as_ref()
        .and_then(|value| serde_json::to_string(value).ok())
        .unwrap_or_else(|| "null".to_string());
    match level.as_str() {
        "error" => log::error!("[frontend] {} | {}", message, context_str),
        "warn" => log::warn!("[frontend] {} | {}", message, context_str),
        "debug" => log::debug!("[frontend] {} | {}", message, context_str),
        _ => log::info!("[frontend] {} | {}", message, context_str),
    };
    Ok(())
}

// Boothy session management commands
#[tauri::command]
async fn boothy_create_or_open_session(
    session_name: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<session::BoothySession, String> {
    let session = state
        .session_manager
        .create_or_open_session(session_name, &app_handle)?;

    // Start file watcher for the session's Raw/ folder
    if let Err(e) = state
        .file_watcher
        .start_watching(session.raw_path.clone(), app_handle.clone())
    {
        log::warn!("Failed to start file watcher: {}", e);
    }

    // Initialize camera IPC client if not already created
    let client_opt = {
        let mut camera_client_guard = state.camera_client.lock().unwrap();
        if camera_client_guard.is_none() {
            let client = camera::ipc_client::CameraIpcClient::new(app_handle.clone());
            *camera_client_guard = Some(client);
        }
        camera_client_guard.as_ref().cloned()
    };

    // Start sidecar and set session destination (non-blocking - errors are logged but don't fail session creation)
    if let Some(client) = client_opt {
        let raw_path = session.raw_path.clone();
        if let Err(e) = client.start_sidecar().await {
            log::warn!("Failed to start camera sidecar: {}", e);
            let correlation_id = camera::generate_correlation_id();
            let error = error::ipc::sidecar_start_failed(e);
            let _ = app_handle.emit("boothy-camera-error", error.to_ui_payload(correlation_id));
        } else {
            // Set the session destination for incoming captures
            let correlation_id = camera::generate_correlation_id();
            let correlation_id_for_error = correlation_id.clone();
            let session_name = raw_path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            if let Err(e) = client.set_session_destination(raw_path, session_name, correlation_id).await {
                log::error!("Failed to set camera session destination: {}", e);
                let error = error::camera::setup_failed(e);
                let _ = app_handle.emit(
                    "boothy-camera-error",
                    error.to_ui_payload(correlation_id_for_error),
                );
            }
        }
    }

    // Initialize FileArrivalWatcher if not already created
    {
        let mut watcher_guard = state.file_arrival_watcher.lock().unwrap();
        if watcher_guard.is_none() {
            let watcher = ingest::file_watcher::FileArrivalWatcher::new(app_handle.clone());
            *watcher_guard = Some(watcher);
        }
    }

    state.session_timer.start_for_session(app_handle.clone());

    // NOTE: FileArrivalWatcher is initialized above and will be invoked via boothy_handle_photo_transferred command
    // The IPC client emits boothy-photo-transferred events which the UI catches and calls the command

    Ok(session)
}

#[tauri::command]
fn boothy_get_active_session(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Option<session::BoothySession>, String> {
    let session = state.session_manager.get_active_session();
    
    // Start file watcher for the session's Raw/ folder when session is retrieved
    if let Some(ref s) = session {
        if let Err(e) = state.file_watcher.start_watching(s.raw_path.clone(), app_handle) {
            log::warn!("Failed to start file watcher for active session: {}", e);
        }
    }
    
    Ok(session)
}

// Mode management commands

#[tauri::command]
fn boothy_get_mode_state(state: tauri::State<'_, AppState>) -> mode::ModeState {
    mode::ModeState {
        mode: state.mode_manager.get_mode(),
        has_admin_password: state.mode_manager.has_admin_password(),
    }
}

#[tauri::command]
fn boothy_set_admin_password(
    password: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    state.mode_manager.set_admin_password(&password)?;

    // Persist password hash to settings
    if let Ok(mut settings) = file_management::load_settings(app_handle.clone()) {
        settings.boothy_admin_password_hash = state.mode_manager.get_password_hash();
        let _ = file_management::save_settings(settings, app_handle.clone());
    }

    // First-time password setup: automatically enter admin mode
    state.mode_manager.authenticate(&password)?;

    // Emit mode change event to UI
    let _ = app_handle.emit(
        "boothy-mode-changed",
        mode::ModeState {
            mode: mode::BoothyMode::Admin,
            has_admin_password: true,
        },
    );

    Ok(())
}

#[tauri::command]
fn boothy_authenticate_admin(
    password: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<bool, String> {
    let result = state.mode_manager.authenticate(&password)?;

    if result {
        // Emit mode change event to UI
        let _ = app_handle.emit(
            "boothy-mode-changed",
            mode::ModeState {
                mode: mode::BoothyMode::Admin,
                has_admin_password: true,
            },
        );
    }

    Ok(result)
}

#[tauri::command]
fn boothy_switch_to_customer_mode(state: tauri::State<'_, AppState>, app_handle: tauri::AppHandle) {
    state.mode_manager.switch_to_customer_mode();

    // Emit mode change event to UI
    let _ = app_handle.emit(
        "boothy-mode-changed",
        mode::ModeState {
            mode: mode::BoothyMode::Customer,
            has_admin_password: state.mode_manager.has_admin_password(),
        },
    );
}

// Preset management commands

#[tauri::command]
fn boothy_set_current_preset(
    preset_id: String,
    preset_name: Option<String>,
    preset_adjustments: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let correlation_id = uuid::Uuid::new_v4().to_string();
    state.preset_manager.set_current_preset(
        preset_id,
        preset_name,
        preset_adjustments,
        &correlation_id,
    );
    Ok(())
}

#[tauri::command]
async fn boothy_handle_photo_transferred(
    path: String,
    correlation_id: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let path_buf = std::path::PathBuf::from(path);

    // Clone the watcher (it's Arc internally) and drop the lock before await
    let watcher_opt = {
        let mut guard = state.file_arrival_watcher.lock().unwrap();
        if guard.is_none() {
            *guard = Some(ingest::file_watcher::FileArrivalWatcher::new(app_handle.clone()));
        }
        guard.as_ref().cloned()
    };

    if let Some(watcher) = watcher_opt {
        watcher.handle_photo_transferred(path_buf, correlation_id).await?;
    }

    Ok(())
}

#[tauri::command]
async fn boothy_handle_export_decision(
    choice: BoothyExportChoice,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let session = state
        .session_manager
        .get_active_session()
        .ok_or("No active session available for export.".to_string())?;

    let raw_files = collect_session_raw_files(&session.raw_path)?;
    let metadata = load_session_metadata(&session.base_path);
    let photo_states = metadata
        .as_ref()
        .map(build_photo_state_map)
        .unwrap_or_default();
    let selected_paths = filter_export_paths(raw_files, Some(&photo_states), choice);

    log::info!("Boothy export decision: {:?} ({} files)", choice, selected_paths.len());

    if selected_paths.is_empty() {
        let mut progress = ExportProgressState::new(0);
        progress.mark_complete();
        let _ = app_handle.emit("boothy-export-progress", progress.to_payload());
        let _ = app_handle.emit("boothy-export-complete", ());
        return Ok(());
    }

    let export_settings = ExportSettings {
        filename_template: Some("{original_filename}".to_string()),
        jpeg_quality: 90,
        keep_metadata: true,
        resize: None,
        strip_gps: true,
        watermark: None,
    };
    let output_folder = session.jpg_path.to_string_lossy().to_string();
    let output_format = "jpg".to_string();
    let paths: Vec<String> = selected_paths
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect();

    start_boothy_batch_export(
        output_folder,
        paths,
        export_settings,
        output_format,
        photo_states,
        state,
        app_handle,
    )
}

#[tauri::command]
fn start_folder_watcher(
    path: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let path_buf = std::path::PathBuf::from(path);
    log::info!("[FolderWatcher] Starting file watcher for folder: {:?}", path_buf);
    
    state
        .file_watcher
        .start_watching(path_buf, app_handle)
}

fn main() {
    // Initialize offline-first logging for field diagnostics
    let logging_initialized = match logging::init_logging() {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Failed to initialize logging: {}", e);
            false
        }
    };

    log::info!("Boothy starting...");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            log::info!(
                "New instance launched with args: {:?}. Focusing main window.",
                argv
            );
            if let Some(window) = app.get_webview_window("main") {
                if let Err(e) = window.unminimize() {
                    log::error!("Failed to unminimize window: {}", e);
                }
                if let Err(e) = window.set_focus() {
                    log::error!("Failed to set focus on window: {}", e);
                }
            }

            if argv.len() > 1 {
                let path_str = &argv[1];
                if let Err(e) = app.emit("open-with-file", path_str) {
                    log::error!(
                        "Failed to emit open-with-file from single-instance handler: {}",
                        e
                    );
                }
            }
        }))
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .setup(move |app| {
            #[cfg(any(windows, target_os = "linux"))]
            {
                if let Some(arg) = std::env::args().nth(1) {
                    let state = app.state::<AppState>();
                    log::info!(
                        "Windows/Linux initial open: Storing path {} for later.",
                        &arg
                    );
                    *state.initial_file_path.lock().unwrap() = Some(arg);
                }
            }

            let app_handle = app.handle().clone();
            let settings: AppSettings = load_settings(app_handle.clone()).unwrap_or_default();

            unsafe {
                if let Some(backend) = &settings.processing_backend {
                    if backend != "auto" {
                        std::env::set_var("WGPU_BACKEND", backend);
                    }
                }

                if settings.linux_gpu_optimization.unwrap_or(true) {
                    #[cfg(target_os = "linux")]
                    {
                        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
                        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
                        std::env::set_var("NODEVICE_SELECT", "1");
                    }
                }
            }

            setup_logging(&app_handle, logging_initialized);
            app.state::<AppState>()
                .background_export_queue
                .start(app_handle.clone());

            if let Some(backend) = &settings.processing_backend {
                if backend != "auto" {
                    log::info!("Applied processing backend setting: {}", backend);
                }
            }
            if settings.linux_gpu_optimization.unwrap_or(false) {
                #[cfg(target_os = "linux")]
                {
                    log::info!("Applied Linux GPU optimizations.");
                }
            }

            start_preview_worker(app_handle.clone());

            // Restore last session if available (constrains library to session Raw/)
            let state = app.state::<AppState>();
            let session_restored = match state.session_manager.restore_last_session(&app_handle) {
                Ok(Some(session)) => {
                    if let Err(e) = state
                        .file_watcher
                        .start_watching(session.raw_path.clone(), app_handle.clone())
                    {
                        log::warn!("Failed to start file watcher for restored session: {}", e);
                    }
                    state.session_timer.start_for_session(app_handle.clone());
                    true
                }
                Ok(None) => false,
                Err(e) => {
                    log::warn!("Failed to restore last session: {}", e);
                    false
                }
            };

            // Load admin password hash from settings
            if let Some(hash) = settings.boothy_admin_password_hash {
                state.mode_manager.load_password_hash(hash);
            }

            // Start file watcher for lastRootPath if no session was restored
            // This ensures file watching works when app is restarted with a folder open
            if !session_restored {
                if let Some(last_root) = &settings.last_root_path {
                    let path_buf = std::path::PathBuf::from(last_root);
                    if path_buf.exists() {
                        log::info!("[Setup] Starting file watcher for lastRootPath: {:?}", path_buf);
                        if let Err(e) = state.file_watcher.start_watching(path_buf, app_handle.clone()) {
                            log::warn!("[Setup] Failed to start file watcher for lastRootPath: {}", e);
                        }
                    }
                }
            }

            let window_cfg = app.config().app.windows.get(0).unwrap().clone();
            let transparent = settings.transparent.unwrap_or(window_cfg.transparent);
            let decorations = settings.decorations.unwrap_or(window_cfg.decorations);

            let window = tauri::WebviewWindowBuilder::from_config(app.handle(), &window_cfg)
                .unwrap()
                .transparent(transparent)
                .decorations(decorations)
                .build()
                .expect("Failed to build window");

            if transparent {
                let theme = settings.theme.unwrap_or("dark".to_string());
                apply_window_effect(theme, &window);
            }

            Ok(())
        })
        .manage(AppState {
            original_image: Mutex::new(None),
            cached_preview: Mutex::new(None),
            gpu_context: Mutex::new(None),
            gpu_image_cache: Mutex::new(None),
            gpu_processor: Mutex::new(None),
            export_task_handle: Mutex::new(None),
            panorama_result: Arc::new(Mutex::new(None)),
            denoise_result: Arc::new(Mutex::new(None)),
            lut_cache: Mutex::new(HashMap::new()),
            initial_file_path: Mutex::new(None),
            thumbnail_cancellation_token: Arc::new(AtomicBool::new(false)),
            preview_worker_tx: Mutex::new(None),
            mask_cache: Mutex::new(HashMap::new()),
            session_manager: session::SessionManager::new(),
            session_timer: session::SessionTimer::new(),
            mode_manager: mode::ModeManager::new(),
            file_watcher: watcher::FileWatcher::new(),
            camera_client: Mutex::new(None),
            file_arrival_watcher: Mutex::new(None),
            preset_manager: preset::preset_manager::PresetManager::new(),
            background_export_queue: Arc::new(session::export_queue::BackgroundExportQueue::new()),
        })
        .invoke_handler(tauri::generate_handler![
            load_image,
            apply_adjustments,
            export_image,
            batch_export_images,
            cancel_export,
            estimate_export_size,
            estimate_batch_export_size,
            generate_fullscreen_preview,
            generate_preview_for_path,
            generate_original_transformed_preview,
            generate_preset_preview,
            generate_uncropped_preview,
            generate_mask_overlay,
            update_window_effect,
            get_supported_file_types,
            get_log_file_path,
            save_collage,
            stitch_panorama,
            save_panorama,
            apply_denoising,
            save_denoised_image,
            load_and_parse_lut,
            save_temp_file,
            get_image_dimensions,
            frontend_ready,
            image_processing::generate_histogram,
            image_processing::generate_waveform,
            image_processing::calculate_auto_adjustments,
            file_management::read_exif_for_paths,
            file_management::list_images_in_dir,
            file_management::list_images_recursive,
            file_management::get_folder_tree,
            file_management::get_pinned_folder_trees,
            file_management::generate_thumbnails,
            file_management::generate_thumbnails_progressive,
            cancel_thumbnail_generation,
            file_management::create_folder,
            file_management::delete_folder,
            file_management::copy_files,
            file_management::move_files,
            file_management::rename_folder,
            file_management::rename_files,
            file_management::duplicate_file,
            file_management::show_in_finder,
            file_management::delete_files_from_disk,
            file_management::delete_files_with_associated,
            file_management::save_metadata_and_update_thumbnail,
            file_management::apply_adjustments_to_paths,
            file_management::load_metadata,
            file_management::load_presets,
            file_management::save_presets,
            file_management::load_settings,
            file_management::save_settings,
            file_management::reset_adjustments_for_paths,
            file_management::apply_auto_adjustments_to_paths,
            file_management::handle_import_presets_from_file,
            file_management::handle_import_legacy_presets_from_file,
            file_management::handle_export_presets_to_file,
            file_management::clear_all_sidecars,
            file_management::clear_thumbnail_cache,
            file_management::set_color_label_for_paths,
            file_management::import_files,
            file_management::create_virtual_copy,
            tagging::clear_all_tags,
            tagging::add_tag_for_paths,
            tagging::remove_tag_for_paths,
            culling::cull_images,
            boothy_create_or_open_session,
            boothy_get_active_session,
            boothy_get_mode_state,
            boothy_set_admin_password,
            boothy_authenticate_admin,
            boothy_switch_to_customer_mode,
            boothy_set_current_preset,
            boothy_handle_photo_transferred,
            boothy_handle_export_decision,
            boothy_log_frontend,
            start_folder_watcher,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(
            #[allow(unused_variables)]
            |app_handle, event| {
                match event {
                    #[cfg(target_os = "macos")]
                    tauri::RunEvent::Opened { urls } => {
                        if let Some(url) = urls.first() {
                            if let Ok(path) = url.to_file_path() {
                                if let Some(path_str) = path.to_str() {
                                    let state = app_handle.state::<AppState>();
                                    *state.initial_file_path.lock().unwrap() =
                                        Some(path_str.to_string());
                                    log::info!(
                                        "macOS initial open: Stored path {} for later.",
                                        path_str
                                    );
                                }
                            }
                        }
                    }
                    tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit => {
                        let state = app_handle.state::<AppState>();
                        if let Some(client) = state.camera_client.lock().unwrap().as_ref() {
                            client.stop_sidecar();
                        }
                    }
                    _ => {}
                }
            },
        );
}
