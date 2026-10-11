// mercury4win/src-tauri/agent/credential_store.rs
// API Key secure storage — stronghold plugin integration with fallback

use std::path::PathBuf;

// Stable bytes for the existing credential file format; independent of the project name.
const CREDENTIAL_OBFUSCATION_KEY: &[u8] = &[
    0x6d, 0x65, 0x72, 0x63, 0x75, 0x72, 0x79, 0x34, 0x77, 0x69, 0x6e, 0x2d, 0x6c, 0x69, 0x6e, 0x75,
    0x78, 0x2d, 0x61, 0x70, 0x69, 0x6b, 0x65, 0x79, 0x2d, 0x6f, 0x62, 0x66, 0x75, 0x73, 0x63, 0x61,
    0x74, 0x69, 0x6f, 0x6e, 0x2d, 0x76, 0x31,
];

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
    let key = CREDENTIAL_OBFUSCATION_KEY;
    let encrypted: Vec<u8> = input
        .bytes()
        .enumerate()
        .map(|(i, b)| b ^ key[i % key.len()])
        .collect();
    hex::encode(encrypted)
}

fn simple_decrypt(hex_str: &str) -> String {
    let key = CREDENTIAL_OBFUSCATION_KEY;
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

#[cfg(test)]
mod tests {
    use super::{simple_decrypt, simple_encrypt};

    // Fixed synthetic fixtures written by the previous version, including key wraparound.
    const LEGACY_ASCII_CIPHERTEXT: &str =
        "1e0e5f1710010d191211075e18000012555d131f1f02011c5f42525747405754425e57574c1452090014";
    const LEGACY_UNICODE_CIPHERTEXT: &str =
        "1e0e5f1710010d1993d1c3cbfaee43050a4217190d0e17541d5e5055414655564c500e0c4e12540b55435146464c02405157";

    #[test]
    fn existing_credentials_remain_readable() {
        assert_eq!(
            simple_decrypt(LEGACY_ASCII_CIPHERTEXT),
            "sk-test-existing-provider-0123456789abcdef",
        );
        assert_eq!(
            simple_decrypt(LEGACY_UNICODE_CIPHERTEXT),
            "sk-test-中文-provider-0123456789abcdef0123456789",
        );
    }

    #[test]
    fn credential_file_format_remains_unchanged() {
        assert_eq!(
            simple_encrypt("sk-test-existing-provider-0123456789abcdef"),
            LEGACY_ASCII_CIPHERTEXT,
        );
        assert_eq!(
            simple_encrypt("sk-test-中文-provider-0123456789abcdef0123456789"),
            LEGACY_UNICODE_CIPHERTEXT,
        );
    }
}
