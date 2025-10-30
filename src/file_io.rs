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

use std::path::Path;

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
    LoadConfig,
    SaveGenerationConfig,
    SaveMapConfig,
    SaveMap,
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

    #[cfg(not(target_arch = "wasm32"))]
    native_dialog: egui_file_dialog::FileDialog,

    #[cfg(target_arch = "wasm32")]
    waiting_for_file: bool,
}

impl FileDialog {
    /// Create a new file dialog for a specific operation type
    pub fn new(operation_type: FileOperationType) -> Self {
        Self {
            operation_type,
            #[cfg(not(target_arch = "wasm32"))]
            native_dialog: egui_file_dialog::FileDialog::new(),
            #[cfg(target_arch = "wasm32")]
            waiting_for_file: false,
        }
    }

    /// Open a save file dialog
    pub fn save_file(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.native_dialog.save_file();
        }

        #[cfg(target_arch = "wasm32")]
        {
            // On WASM, save operations return immediately with a default filename
            self.waiting_for_file = true;
            wasm_impl::queue_save_operation(self.operation_type);
        }
    }

    /// Open a file picker dialog to load a file
    pub fn pick_file(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.native_dialog.pick_file();
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.waiting_for_file = true;
            wasm_impl::set_pending_operation(self.operation_type);
            wasm_impl::open_file_picker_impl();
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

    /// Take the picked file path/data if available
    pub fn take_picked(&mut self) -> Option<LoadedFile> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = self.native_dialog.take_picked() {
                let path_str = path.to_string_lossy();
                if let Ok(data) = std::fs::read(&*path) {
                    return Some(native_impl::create_loaded_file_from_path(&path_str, data));
                }
            }
            None
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Check if this is a queued save operation
            if self.waiting_for_file {
                if let Some(queued) = wasm_impl::take_queued_save() {
                    if queued == self.operation_type {
                        self.waiting_for_file = false;
                        // Return a synthetic LoadedFile with default filename
                        return Some(LoadedFile {
                            filename: "save".to_string(), // Will be replaced by caller
                            config_name: "default".to_string(),
                            data: Vec::new(), // Not used for saves
                        });
                    }
                }

                // Check if this dialog opened a file picker and file is ready
                if wasm_impl::is_pending_operation(self.operation_type) {
                    if let Some(loaded) = wasm_impl::take_loaded_file() {
                        self.waiting_for_file = false;
                        wasm_impl::clear_pending_operation();
                        return Some(loaded);
                    }
                }
            }
            None
        }
    }
}

// ============================================================================
// WASM-specific implementation
// ============================================================================

#[cfg(target_arch = "wasm32")]
mod wasm_impl {
    use super::{extract_config_name, FileOperationType, LoadedFile};
    use std::sync::Mutex;

    // FFI declarations for JavaScript functions
    extern "C" {
        pub fn wasm_download_file(
            filename_ptr: *const u8,
            filename_len: usize,
            data_ptr: *const u8,
            data_len: usize,
        );

        pub fn wasm_open_file_picker();
    }

    // Global storage for loaded file data
    static LOADED_FILE: Mutex<Option<LoadedFile>> = Mutex::new(None);
    static PENDING_OPERATION: Mutex<Option<FileOperationType>> = Mutex::new(None);
    static QUEUED_SAVE: Mutex<Option<FileOperationType>> = Mutex::new(None);

    pub fn set_pending_operation(op: FileOperationType) {
        let mut pending = PENDING_OPERATION.lock()
            .expect("Mutex poisoned - should be impossible in single-threaded WASM");
        *pending = Some(op);
    }

    pub fn is_pending_operation(op: FileOperationType) -> bool {
        let pending = PENDING_OPERATION.lock()
            .expect("Mutex poisoned - should be impossible in single-threaded WASM");
        *pending == Some(op)
    }

    pub fn clear_pending_operation() {
        let mut pending = PENDING_OPERATION.lock()
            .expect("Mutex poisoned - should be impossible in single-threaded WASM");
        *pending = None;
    }

    pub fn queue_save_operation(op: FileOperationType) {
        let mut queued = QUEUED_SAVE.lock()
            .expect("Mutex poisoned - should be impossible in single-threaded WASM");
        *queued = Some(op);
    }

    pub fn take_queued_save() -> Option<FileOperationType> {
        let mut queued = QUEUED_SAVE.lock()
            .expect("Mutex poisoned - should be impossible in single-threaded WASM");
        queued.take()
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
                log::warn!("Filename contains invalid UTF-8, using lossy conversion");
            }

            // Extract config name from filename
            let config_name = extract_config_name(&filename);

            // Read file data from memory
            let data_slice = std::slice::from_raw_parts(data_ptr, data_len);
            let data = data_slice.to_vec();

            log::info!("File loaded: {} ({} bytes) → config: {}", filename, data_len, config_name);

            // Store the loaded file data
            let mut loaded = LOADED_FILE.lock()
                .expect("Mutex poisoned - should be impossible in single-threaded WASM");
            *loaded = Some(LoadedFile {
                filename: filename.to_string(),
                config_name,
                data,
            });
        }
    }

    /// Save a file (downloads it in the browser)
    pub fn save_file_impl(filename: &str, data: &[u8]) -> Result<(), String> {
        unsafe {
            wasm_download_file(
                filename.as_ptr(),
                filename.len(),
                data.as_ptr(),
                data.len(),
            );
        }
        Ok(())
    }

    /// Open file picker dialog (asynchronous operation)
    pub fn open_file_picker_impl() {
        unsafe {
            wasm_open_file_picker();
        }
    }

    /// Check if a file has been loaded and retrieve it (consuming the stored file)
    pub fn take_loaded_file() -> Option<LoadedFile> {
        let mut loaded = LOADED_FILE.lock()
            .expect("Mutex poisoned - should be impossible in single-threaded WASM");
        loaded.take()
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
        let mut file = File::create(&path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        file.write_all(data)
            .map_err(|e| format!("Failed to write file: {}", e))?;
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
