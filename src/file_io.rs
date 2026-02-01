//! Platform-agnostic file I/O abstraction layer
//!
//! This module provides file save and load functionality that works on both:
//! - **Native platforms**: Uses `std::fs` for direct filesystem access
//! - **WASM**: Uses custom JavaScript bindings via miniquad plugin system for browser file operations
//!
//! ## Platform Isolation
//!
//! This module is the ONLY place that should contain `#[cfg(target_arch = "wasm32")]` checks.
//! All other modules should use the platform-agnostic API provided here.
//!
//! ## File Dialog Flow
//!
//! ### Loading a file:
//! ```text
//! User Code         FileDialog         JavaScript         Browser
//!    |                  |                  |                 |
//!    |-- pick_file() -->|                  |                 |
//!    |                  |-- JS call ------>|                 |
//!    |                  |                  |-- open picker ->|
//!    |                  |                  |                 |
//!    |                  |                  |<-- file data ---|
//!    |                  |<-- callback -----|                 |
//!    |                  | (store in state) |                 |
//!    |                  |                  |                 |
//!    |-- take_result()--|                  |                 |
//!    |<-- Loaded(file) -|                  |                 |
//! ```
//!
//! ### Saving a file (WASM):
//! ```text
//! User Code         FileDialog         JavaScript         Browser
//!    |                  |                  |                 |
//!    |-- save_file() -->|                  |                 |
//!    |                  | (return path)    |                 |
//!    |                  |                  |                 |
//!    |-- take_result()--|                  |                 |
//!    |<-- SavePath(...)-|                  |                 |
//!    |                  |                  |                 |
//!    |- save_file_bytes(...) ------------->|                 |
//!    |                  |                  |-- download ---->|
//! ```
//!
//! ## Example Usage
//!
//! ```rust
//! use file_io::{FileDialog, FileOperationType, FileFilter, FileDialogResult};
//!
//! // Create dialog with file filtering
//! let mut dialog = FileDialog::new(FileOperationType::LoadGenerationConfig)
//!     .with_filter(FileFilter::json());
//!
//! // In button handler:
//! if button_clicked {
//!     dialog.pick_file();
//! }
//!
//! // In update loop:
//! if let Some(result) = dialog.take_result() {
//!     match result {
//!         FileDialogResult::Loaded(file) => {
//!             println!("Loaded: {}", file.filename);
//!             // Use file.data
//!         }
//!         FileDialogResult::Cancelled => {
//!             println!("User cancelled");
//!         }
//!         FileDialogResult::Error(err) => {
//!             eprintln!("Error: {}", err);
//!         }
//!         _ => {}
//!     }
//! }
//! ```

#[allow(unused_imports)]
use macroquad::prelude::{error, info, warn};
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

// ============================================================================
// Shared types and utilities
// ============================================================================

/// Represents a file that has been loaded
#[derive(Clone, Debug)]
pub struct LoadedFile {
    /// Original filename (with extension)
    pub filename: String,
    /// Config name extracted from filename (without extension)
    pub config_name: String,
    /// Raw file data
    pub data: Vec<u8>,
}

/// Type of file operation being performed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOperationType {
    LoadGenerationConfig,
    LoadMapConfig,
    SaveGenerationConfig,
    SaveMapConfig,
    SaveMap,
}

/// Result from a file dialog operation
#[derive(Debug, Clone)]
pub enum FileDialogResult {
    /// File was successfully loaded (from pick_file())
    Loaded(LoadedFile),
    /// User selected save location (from save_file())
    SavePath(String),
    /// User cancelled the operation
    Cancelled,
    /// An error occurred
    Error(String),
}

/// File type filter for file picker dialogs
#[derive(Debug, Clone)]
pub struct FileFilter {
    /// Human-readable description (e.g., "JSON files")
    pub description: String,
    /// File extensions without dots (e.g., ["json", "txt"])
    pub extensions: Vec<String>,
}

impl FileFilter {
    /// Create a new file filter
    pub fn new(description: impl Into<String>, extensions: Vec<String>) -> Self {
        Self {
            description: description.into(),
            extensions,
        }
    }

    /// Commonly used filter for JSON files
    pub fn json() -> Self {
        Self::new("JSON files", vec!["json".to_string()])
    }

    /// Commonly used filter for map files
    pub fn map() -> Self {
        Self::new("Map files", vec!["map".to_string()])
    }

    /// Get accept string for HTML file input (e.g., ".json,.txt")
    pub fn to_accept_string(&self) -> String {
        self.extensions
            .iter()
            .map(|ext| format!(".{ext}"))
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// Extract config name from a file path or filename.
///
/// Removes the file extension and returns just the base name.
/// Examples:
/// - "my_config.json" → "my_config"
/// - "/path/to/config.json" → "config"
/// - "no_extension" → "no_extension"
pub fn extract_config_name(path_or_filename: &str) -> String {
    Path::new(path_or_filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(path_or_filename)
        .to_string()
}

/// Extract just the filename from a path, or use a default.
pub fn extract_filename_or_default<'a>(path: &'a str, default: &'a str) -> &'a str {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(default)
}

// ============================================================================
// Unified FileDialog
// ============================================================================

/// Platform-agnostic file dialog that works identically on native and WASM
pub struct FileDialog {
    operation_type: FileOperationType,
    file_filter: Option<FileFilter>,
    default_name: Option<String>,

    #[cfg(not(target_arch = "wasm32"))]
    native_dialog: egui_file_dialog::FileDialog,
}

impl FileDialog {
    /// Create a new file dialog for a specific operation type
    pub fn new(operation_type: FileOperationType) -> Self {
        Self {
            operation_type,
            file_filter: None,
            default_name: None,
            #[cfg(not(target_arch = "wasm32"))]
            native_dialog: egui_file_dialog::FileDialog::new(),
        }
    }

    /// Set file type filter (builder pattern)
    pub fn with_filter(mut self, filter: FileFilter) -> Self {
        self.file_filter = Some(filter);
        self
    }

    /// Set default filename for save dialogs (builder pattern)
    pub fn with_default_name(mut self, name: impl Into<String>) -> Self {
        self.default_name = Some(name.into());
        self
    }

    /// Set default filename for save dialogs (mutating method)
    pub fn set_default_name(&mut self, name: impl Into<String>) {
        self.default_name = Some(name.into());
    }

    /// Open a save file dialog
    pub fn save_file(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Reconstruct dialog with configuration
            let mut dialog = egui_file_dialog::FileDialog::new();

            if let Some(name) = &self.default_name {
                dialog = dialog.default_file_name(name);
            }

            dialog.save_file();
            self.native_dialog = dialog;
        }

        #[cfg(target_arch = "wasm32")]
        {
            let default_name = self.default_name.as_deref().unwrap_or("file");
            wasm_impl::start_save(self.operation_type, default_name);
        }
    }

    /// Open a file picker dialog to load a file
    pub fn pick_file(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Reconstruct dialog with configuration
            let mut dialog = egui_file_dialog::FileDialog::new();

            if let Some(filter) = &self.file_filter {
                // Create closure that checks if path matches any of the extensions
                let extensions = filter.extensions.clone();
                dialog = dialog.add_file_filter(
                    &filter.description,
                    Arc::new(move |path: &Path| {
                        path.extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| extensions.iter().any(|e| e == ext))
                            .unwrap_or(false)
                    }),
                );
            }

            dialog.pick_file();
            self.native_dialog = dialog;
        }

        #[cfg(target_arch = "wasm32")]
        {
            let accept = self.file_filter.as_ref().map(|f| f.to_accept_string());
            wasm_impl::start_load(self.operation_type, accept.as_deref());
        }
    }

    /// Update the dialog (must be called every frame on native, no-op on WASM)
    pub fn update(&mut self, ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.native_dialog.update(ctx);
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = ctx; // Suppress unused variable warning
        }
    }

    /// Take the dialog result if available
    pub fn take_result(&mut self) -> Option<FileDialogResult> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = self.native_dialog.take_picked() {
                let path_str = path.to_string_lossy();

                // Determine if this was a load or save based on operation type
                match self.operation_type {
                    FileOperationType::LoadGenerationConfig | FileOperationType::LoadMapConfig => {
                        // Load operation - read file
                        match std::fs::read(&*path) {
                            Ok(data) => Some(FileDialogResult::Loaded(
                                native_impl::create_loaded_file_from_path(&path_str, data),
                            )),
                            Err(e) => {
                                Some(FileDialogResult::Error(format!("Failed to read file: {e}")))
                            }
                        }
                    }
                    _ => {
                        // Save operation - return path
                        Some(FileDialogResult::SavePath(path_str.to_string()))
                    }
                }
            } else {
                None
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            wasm_impl::take_result(self.operation_type)
        }
    }
}

// ============================================================================
// WASM-specific implementation
// ============================================================================

#[cfg(target_arch = "wasm32")]
mod wasm_impl {
    use super::{error, info, warn};
    use super::{extract_config_name, FileDialogResult, FileOperationType, LoadedFile};
    use std::sync::Mutex;

    // FFI declarations for JavaScript functions
    extern "C" {
        pub fn wasm_download_file(
            filename_ptr: *const u8,
            filename_len: usize,
            data_ptr: *const u8,
            data_len: usize,
        );

        pub fn wasm_open_file_picker(accept_ptr: *const u8, accept_len: usize);
    }

    /// State for the current file dialog operation
    ///
    /// Note: Only ONE dialog can be active at a time in the browser since
    /// the file picker is modal. This significantly simplifies state management.
    #[derive(Debug)]
    enum DialogState {
        Idle,
        WaitingForFileLoad(FileOperationType),
        Ready(FileOperationType, FileDialogResult),
    }

    /// Global state - only one dialog can be active at a time in the browser
    static CURRENT_STATE: Mutex<DialogState> = Mutex::new(DialogState::Idle);

    /// Start a file load operation
    pub fn start_load(op: FileOperationType, accept: Option<&str>) {
        *CURRENT_STATE.lock().unwrap() = DialogState::WaitingForFileLoad(op);

        // Open file picker
        unsafe {
            if let Some(accept_str) = accept {
                wasm_open_file_picker(accept_str.as_ptr(), accept_str.len());
            } else {
                wasm_open_file_picker(std::ptr::null(), 0);
            }
        }
    }

    /// Start a save operation (returns immediately on WASM)
    pub fn start_save(op: FileOperationType, default_name: &str) {
        // Immediately transition to Ready with the path
        *CURRENT_STATE.lock().unwrap() =
            DialogState::Ready(op, FileDialogResult::SavePath(default_name.to_string()));
    }

    /// Take the result if ready and matches the operation type
    pub fn take_result(op: FileOperationType) -> Option<FileDialogResult> {
        let mut state = CURRENT_STATE.lock().unwrap();

        if let DialogState::Ready(ready_op, result) = &*state {
            // Only return result if it matches the requesting operation type
            if *ready_op == op {
                let result = result.clone();
                *state = DialogState::Idle;
                Some(result)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Allocate a buffer in WASM memory that JavaScript can write to.
    ///
    /// # Safety
    ///
    /// This function transfers ownership of the allocated memory to JavaScript.
    /// The caller MUST call `wasm_free_buffer` with the same pointer and size
    /// to properly deallocate the memory. Using `std::mem::forget` is safe here
    /// because JavaScript assumes responsibility for the memory lifetime.
    #[no_mangle]
    pub extern "C" fn wasm_alloc_buffer(size: usize) -> *mut u8 {
        let mut buf = Vec::with_capacity(size);
        let ptr = buf.as_mut_ptr();
        std::mem::forget(buf); // Transfer ownership to JavaScript
        ptr
    }

    /// Free a previously allocated buffer.
    ///
    /// # Safety
    ///
    /// - `ptr` must be a valid pointer returned by `wasm_alloc_buffer`
    /// - `size` must match the size passed to `wasm_alloc_buffer`
    /// - This function must be called exactly once per `wasm_alloc_buffer` call
    #[no_mangle]
    pub extern "C" fn wasm_free_buffer(ptr: *mut u8, size: usize) {
        unsafe {
            let _ = Vec::from_raw_parts(ptr, size, size);
            // Vec will be dropped and memory freed
        }
    }

    /// Callback function called by JavaScript when a file is loaded.
    ///
    /// # Safety
    ///
    /// - `filename_ptr` must point to valid UTF-8 bytes of length `filename_len`
    /// - `data_ptr` must point to valid bytes of length `data_len`
    /// - Pointers must remain valid for the duration of this function call
    /// - This function should only be called from the JavaScript file picker callback
    #[no_mangle]
    pub extern "C" fn wasm_file_loaded_callback(
        filename_ptr: *const u8,
        filename_len: usize,
        data_ptr: *const u8,
        data_len: usize,
    ) {
        unsafe {
            // Read filename from memory
            let filename_bytes = std::slice::from_raw_parts(filename_ptr, filename_len);
            let filename = String::from_utf8_lossy(filename_bytes);

            // Warn if filename contains invalid UTF-8
            if filename_bytes != filename.as_bytes() {
                warn!("Filename contains invalid UTF-8, using lossy conversion");
            }

            // Extract config name from filename
            let config_name = extract_config_name(&filename);

            // Read file data from memory
            let data_slice = std::slice::from_raw_parts(data_ptr, data_len);
            let data = data_slice.to_vec();

            info!(
                "File loaded: {} ({} bytes) → config: {}",
                filename, data_len, config_name
            );

            // Store the result with the operation type from the waiting state
            let mut state = CURRENT_STATE.lock().unwrap();
            if let DialogState::WaitingForFileLoad(op) = *state {
                *state = DialogState::Ready(
                    op,
                    FileDialogResult::Loaded(LoadedFile {
                        filename: filename.to_string(),
                        config_name,
                        data,
                    }),
                );
            }
        }
    }

    /// Callback function called by JavaScript when file picker is cancelled.
    #[no_mangle]
    pub extern "C" fn wasm_file_picker_cancelled() {
        info!("File picker cancelled");

        let mut state = CURRENT_STATE.lock().unwrap();
        if let DialogState::WaitingForFileLoad(op) = *state {
            *state = DialogState::Ready(op, FileDialogResult::Cancelled);
        }
    }

    /// Callback function called by JavaScript when an error occurs.
    ///
    /// # Safety
    ///
    /// - `error_ptr` must point to valid UTF-8 bytes of length `error_len`
    /// - Pointer must remain valid for the duration of this function call
    #[no_mangle]
    pub extern "C" fn wasm_file_error_callback(error_ptr: *const u8, error_len: usize) {
        unsafe {
            let error_bytes = std::slice::from_raw_parts(error_ptr, error_len);
            let error = String::from_utf8_lossy(error_bytes).to_string();

            error!("File operation error: {}", error);

            let mut state = CURRENT_STATE.lock().unwrap();
            if let DialogState::WaitingForFileLoad(op) = *state {
                *state = DialogState::Ready(op, FileDialogResult::Error(error));
            }
        }
    }

    /// Save a file (downloads it in the browser)
    pub fn save_file_impl(path: &str, data: &[u8]) -> Result<(), String> {
        // Extract just the filename for browser downloads (ignore directory path)
        let filename = super::extract_filename_or_default(path, "map.map");
        unsafe {
            wasm_download_file(filename.as_ptr(), filename.len(), data.as_ptr(), data.len());
        }
        Ok(())
    }
}

// ============================================================================
// Native-specific implementation
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
mod native_impl {
    use super::{extract_config_name, LoadedFile};
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;

    /// Save a file using native filesystem
    pub fn save_file_impl(filename: &str, data: &[u8]) -> Result<(), String> {
        let path = PathBuf::from(filename);
        let mut file = File::create(&path).map_err(|e| format!("Failed to create file: {e}"))?;
        file.write_all(data)
            .map_err(|e| format!("Failed to write file: {e}"))?;
        Ok(())
    }

    /// Create a LoadedFile from a path (native helper for config loading)
    pub fn create_loaded_file_from_path(path: &str, data: Vec<u8>) -> LoadedFile {
        let filename = path.to_string();
        let config_name = extract_config_name(path);
        LoadedFile {
            filename,
            config_name,
            data,
        }
    }
}

// ============================================================================
// Public API (platform-agnostic)
// ============================================================================

#[cfg(target_arch = "wasm32")]
use wasm_impl::save_file_impl as save_file;

#[cfg(not(target_arch = "wasm32"))]
use native_impl::save_file_impl as save_file;

/// Save file to disk (native) or trigger download (WASM).
///
/// On native platforms, this writes directly to the filesystem.
/// On WASM, this triggers a browser download.
pub fn save_file_bytes(filename: &str, data: &[u8]) -> Result<(), String> {
    save_file(filename, data)
}

/// Save a string to a file.
///
/// Convenience wrapper around `save_file_bytes` that converts the string to bytes.
pub fn save_file_string(filename: &str, content: &str) -> Result<(), String> {
    save_file(filename, content.as_bytes())
}
