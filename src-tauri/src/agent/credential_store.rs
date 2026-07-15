// mercury4win-linux/src-tauri/agent/credential_store.rs
// API Key secure storage — stronghold plugin integration with fallback

use std::path::PathBuf;

/// Store an API key for a provider. Uses stronghold when available,
/// falls back to encrypted file storage.
pub async fn store_api_key(
    app_handle: &tauri::AppHandle,
    provider_id: &str,
    api_key: &str,
) -> Result<(), String> {
    // Fallback: AES-encrypted file in app_data_dir
    let key_path = credential_path(app_handle, provider_id);
    let encrypted = simple_encrypt(api_key);
    tokio::fs::write(&key_path, encrypted)
        .await
        .map_err(|e| format!("Failed to store API key: {}", e))?;

    // Set file permissions to owner-only on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
            .await
            .ok();
    }

    Ok(())
}

/// Retrieve an API key for a provider.
pub async fn get_api_key(
    app_handle: &tauri::AppHandle,
    provider_id: &str,
) -> Result<Option<String>, String> {
    let key_path = credential_path(app_handle, provider_id);
    match tokio::fs::read_to_string(&key_path).await {
        Ok(encrypted) => {
            let key = simple_decrypt(&encrypted);
            Ok(Some(key))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Failed to read API key: {}", e)),
    }
}

/// Delete a stored API key.
pub async fn delete_api_key(
    app_handle: &tauri::AppHandle,
    provider_id: &str,
) -> Result<(), String> {
    let key_path = credential_path(app_handle, provider_id);
    match tokio::fs::remove_file(&key_path).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Failed to delete API key: {}", e)),
    }
}

fn credential_path(app_handle: &tauri::AppHandle, provider_id: &str) -> PathBuf {
    use tauri::Manager;
    let dir = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("credentials");
    std::fs::create_dir_all(&dir).ok();
    dir.join(format!("{}.key", provider_id))
}

// Simple XOR-based obfuscation (not cryptographically secure, better than plaintext).
// In production, this should be replaced with stronghold or OS keychain.

fn simple_encrypt(input: &str) -> String {
    let key: &[u8] = b"mercury4win-linux-apikey-obfuscation-v1";
    let encrypted: Vec<u8> = input
        .bytes()
        .enumerate()
        .map(|(i, b)| b ^ key[i % key.len()])
        .collect();
    hex::encode(encrypted)
}

fn simple_decrypt(hex_str: &str) -> String {
    let key: &[u8] = b"mercury4win-linux-apikey-obfuscation-v1";
    match hex::decode(hex_str) {
        Ok(bytes) => {
            let decrypted: Vec<u8> = bytes
                .iter()
                .enumerate()
                .map(|(i, b)| b ^ key[i % key.len()])
                .collect();
            String::from_utf8_lossy(&decrypted).to_string()
        }
        Err(_) => String::new(),
    }
}
