//! Platform-agnostic file I/O abstraction layer
//!
//! This module provides file save and load functionality that works on both
//! native platforms (using std::fs) and WASM (using custom JavaScript bindings).

// ============================================================================
// WASM-specific code
// ============================================================================

#[cfg(target_arch = "wasm32")]
mod wasm_impl {
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

    #[derive(Clone)]
    pub struct LoadedFile {
        pub filename: String,
        pub data: Vec<u8>,
    }

    /// Allocate a buffer in WASM memory that JavaScript can write to
    #[no_mangle]
    pub extern "C" fn wasm_alloc_buffer(size: usize) -> *mut u8 {
        let mut buf = Vec::with_capacity(size);
        let ptr = buf.as_mut_ptr();
        std::mem::forget(buf); // Prevent Rust from freeing this memory
        ptr
    }

    /// Free a previously allocated buffer
    #[no_mangle]
    pub extern "C" fn wasm_free_buffer(ptr: *mut u8, size: usize) {
        unsafe {
            let _ = Vec::from_raw_parts(ptr, size, size);
            // Vec will be dropped and memory freed
        }
    }

    /// Callback function called by JavaScript when a file is loaded
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
            let filename = String::from_utf8_lossy(filename_bytes).to_string();

            // Read file data from memory
            let data_slice = std::slice::from_raw_parts(data_ptr, data_len);
            let data = data_slice.to_vec();

            log::info!("File loaded: {} ({} bytes)", filename, data_len);

            // Store the loaded file data
            let mut loaded = LOADED_FILE.lock().unwrap();
            *loaded = Some(LoadedFile { filename, data });
        }
    }

    /// Save a file (downloads it in the browser)
    pub fn save_file(filename: &str, data: &[u8]) -> Result<(), String> {
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

    /// Open file picker dialog
    pub fn open_file_picker() {
        unsafe {
            wasm_open_file_picker();
        }
    }

    /// Check if a file has been loaded and retrieve it
    pub fn take_loaded_file() -> Option<LoadedFile> {
        let mut loaded = LOADED_FILE.lock().unwrap();
        loaded.take()
    }

    /// Check if a file is ready without consuming it
    pub fn has_loaded_file() -> bool {
        let loaded = LOADED_FILE.lock().unwrap();
        loaded.is_some()
    }
}

// ============================================================================
// Native-specific code
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
mod native_impl {
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;

    #[derive(Clone)]
    pub struct LoadedFile {
        pub filename: String,
        pub data: Vec<u8>,
    }

    /// Save a file using native filesystem
    pub fn save_file(filename: &str, data: &[u8]) -> Result<(), String> {
        let path = PathBuf::from(filename);
        let mut file = File::create(&path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        file.write_all(data)
            .map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(())
    }

    /// Open file picker dialog (uses egui-file-dialog, handled elsewhere)
    /// This is a no-op on native as we use egui-file-dialog in the GUI
    pub fn open_file_picker() {
        // No-op: native file picking is handled by egui-file-dialog in gui.rs
    }

    /// Take loaded file (no-op on native)
    pub fn take_loaded_file() -> Option<LoadedFile> {
        None // Native file loading is handled directly by egui-file-dialog
    }

    /// Check if file is loaded (always false on native)
    pub fn has_loaded_file() -> bool {
        false // Native file loading is handled directly by egui-file-dialog
    }
}

// ============================================================================
// Public API (platform-agnostic)
// ============================================================================

#[cfg(target_arch = "wasm32")]
pub use wasm_impl::*;

#[cfg(not(target_arch = "wasm32"))]
pub use native_impl::*;

/// Save file to disk (native) or trigger download (WASM)
pub fn save_file_bytes(filename: &str, data: &[u8]) -> Result<(), String> {
    save_file(filename, data)
}

/// Save a string to a file
pub fn save_file_string(filename: &str, content: &str) -> Result<(), String> {
    save_file(filename, content.as_bytes())
}

/// Load a file and return its contents as bytes
/// On WASM, this opens a file picker and the result will be available
/// via take_loaded_file() when the user selects a file
pub fn load_file_bytes() {
    open_file_picker()
}

/// Load a file and return its contents as a string
pub fn load_file_string() {
    open_file_picker()
}
