#[allow(unused_imports)]
use keyring::Entry;
use std::fs;
use std::path::PathBuf;

#[allow(dead_code)]
const SERVICE_NAME: &str = "com.sentinel.filemanager";

/// Credential manager using file storage (keychain unreliable in Tauri dev mode)
pub struct CredentialManager;

impl CredentialManager {
    /// Get the fallback file path for storing credentials
    fn get_fallback_path(provider: &str) -> Option<PathBuf> {
        dirs::config_dir().map(|dir| {
            let app_dir = dir.join("sentinel");
            app_dir.join(format!("{}_key", provider))
        })
    }

    /// Store an API key using file storage (keychain unreliable in dev mode)
    pub fn store_api_key(provider: &str, api_key: &str) -> Result<(), String> {
        // Use file storage directly - keychain is unreliable in Tauri dev mode
        if let Some(path) = Self::get_fallback_path(provider) {
            // Ensure directory exists
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create config directory: {}", e))?;
            }

            // Write the key (base64 encoded for minimal obfuscation)
            let encoded = base64_encode(api_key);
            fs::write(&path, encoded)
                .map_err(|e| format!("Failed to write API key: {}", e))?;

            eprintln!("[Credentials] Stored API key in file: {:?}", path);
            Ok(())
        } else {
            Err("Could not determine config directory".to_string())
        }
    }

    /// Get an API key from file storage
    pub fn get_api_key(provider: &str) -> Result<String, String> {
        if let Some(path) = Self::get_fallback_path(provider) {
            if path.exists() {
                let encoded = fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read API key: {}", e))?;
                let decoded = base64_decode(&encoded)?;
                eprintln!("[Credentials] Retrieved API key from file: {:?}", path);
                return Ok(decoded);
            }
        }

        Err("API key not found".to_string())
    }

    /// Delete an API key from file storage
    pub fn delete_api_key(provider: &str) -> Result<(), String> {
        if let Some(path) = Self::get_fallback_path(provider) {
            if path.exists() {
                fs::remove_file(&path)
                    .map_err(|e| format!("Failed to delete API key file: {}", e))?;
                eprintln!("[Credentials] Deleted API key file: {:?}", path);
            }
        }

        Ok(())
    }

    /// Check if an API key is configured
    pub fn has_api_key(provider: &str) -> bool {
        Self::get_api_key(provider).is_ok()
    }
}

// Simple base64 encoding/decoding for minimal obfuscation
fn base64_encode(input: &str) -> String {
    use std::io::Write;
    let mut output = Vec::new();
    {
        let mut encoder = Base64Encoder::new(&mut output);
        encoder.write_all(input.as_bytes()).unwrap();
    }
    String::from_utf8(output).unwrap()
}

fn base64_decode(input: &str) -> Result<String, String> {
    let bytes = base64_decode_bytes(input.trim())?;
    String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8: {}", e))
}

// Manual base64 implementation to avoid adding dependencies
const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

struct Base64Encoder<'a, W: std::io::Write> {
    writer: &'a mut W,
    buffer: [u8; 3],
    buffer_len: usize,
}

impl<'a, W: std::io::Write> Base64Encoder<'a, W> {
    fn new(writer: &'a mut W) -> Self {
        Self {
            writer,
            buffer: [0; 3],
            buffer_len: 0,
        }
    }

    fn flush_buffer(&mut self) -> std::io::Result<()> {
        if self.buffer_len == 0 {
            return Ok(());
        }

        let b0 = self.buffer[0] as usize;
        let b1 = if self.buffer_len > 1 { self.buffer[1] as usize } else { 0 };
        let b2 = if self.buffer_len > 2 { self.buffer[2] as usize } else { 0 };

        let c0 = BASE64_CHARS[b0 >> 2];
        let c1 = BASE64_CHARS[((b0 & 0x03) << 4) | (b1 >> 4)];
        let c2 = if self.buffer_len > 1 {
            BASE64_CHARS[((b1 & 0x0f) << 2) | (b2 >> 6)]
        } else {
            b'='
        };
        let c3 = if self.buffer_len > 2 {
            BASE64_CHARS[b2 & 0x3f]
        } else {
            b'='
        };

        self.writer.write_all(&[c0, c1, c2, c3])?;
        self.buffer_len = 0;
        Ok(())
    }
}

impl<'a, W: std::io::Write> std::io::Write for Base64Encoder<'a, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for &byte in buf {
            self.buffer[self.buffer_len] = byte;
            self.buffer_len += 1;
            if self.buffer_len == 3 {
                self.flush_buffer()?;
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.flush_buffer()
    }
}

impl<'a, W: std::io::Write> Drop for Base64Encoder<'a, W> {
    fn drop(&mut self) {
        let _ = self.flush_buffer();
    }
}

fn base64_decode_bytes(input: &str) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    let input = input.as_bytes();

    let decode_char = |c: u8| -> Result<u8, String> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            b'=' => Ok(0),
            _ => Err(format!("Invalid base64 character: {}", c as char)),
        }
    };

    let mut i = 0;
    while i < input.len() {
        if i + 4 > input.len() {
            break;
        }

        let c0 = decode_char(input[i])?;
        let c1 = decode_char(input[i + 1])?;
        let c2 = decode_char(input[i + 2])?;
        let c3 = decode_char(input[i + 3])?;

        output.push((c0 << 2) | (c1 >> 4));
        if input[i + 2] != b'=' {
            output.push((c1 << 4) | (c2 >> 2));
        }
        if input[i + 3] != b'=' {
            output.push((c2 << 6) | c3);
        }

        i += 4;
    }

    Ok(output)
}
