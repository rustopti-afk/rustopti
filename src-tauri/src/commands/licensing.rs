use serde_json::json;
use reqwest::Client;
use std::time::Duration;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tauri::State;

use crate::utils::license_guard::{
    LicenseState, activate_license, deactivate_license,
    save_license_cache, delete_license_cache, is_license_valid,
    touch_license_cache, get_cache_status,
    store_key_in_memory, get_stored_key,
    store_sub_info, get_sub_info,
};

type HmacSha256 = Hmac<Sha256>;

// ═══════════════════════════════════════════════════════════════
// Multi-layer encrypted constants
// Keys derived from scattered seeds via SHA256 — no single XOR key
// ═══════════════════════════════════════════════════════════════

// Same seeds as license_guard.rs (shared across modules)
const SEED_A: [u8; 8] = [0x7B, 0x3F, 0xA2, 0x19, 0xE6, 0x54, 0xCD, 0x88];
const SEED_B: [u8; 8] = [0x41, 0xD7, 0x0C, 0x95, 0x6A, 0xF3, 0x28, 0xBE];

fn derive_key(context: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(&SEED_A);
    hasher.update(&SEED_B);
    hasher.update(context);
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

fn decrypt_with_context(encrypted: &[u8], context: &[u8]) -> Vec<u8> {
    let key = derive_key(context);
    encrypted.iter().enumerate().map(|(i, &b)| {
        b ^ key[i % 32]
    }).collect()
}

// HMAC secret encrypted with derive_key(SEED_B || SEED_A || "hmac")
// Note: different seed ORDER + context than cache secret
const HMAC_SECRET_ENC: [u8; 32] = [
    0x1B, 0xCF, 0x9F, 0x23, 0x6F, 0x19, 0xA4, 0x4A,
    0x51, 0xFE, 0x53, 0x93, 0x3D, 0x1A, 0xB4, 0x64,
    0x73, 0x36, 0xAC, 0x15, 0x8F, 0xF8, 0xE3, 0xEB,
    0xBE, 0xA7, 0x2F, 0xA2, 0xF7, 0x65, 0xA3, 0x8E,
];

fn get_hmac_secret() -> Vec<u8> {
    // Key = SHA256(SEED_B || SEED_A || "hmac")
    // Note: reversed seed order + "hmac" context — different from cache key
    let key = {
        use sha2::Digest;
        let mut hasher = Sha256::new();
        hasher.update(&SEED_B); // reversed!
        hasher.update(&SEED_A);
        hasher.update(b"hmac");
        let result = hasher.finalize();
        let mut k = [0u8; 32];
        k.copy_from_slice(&result);
        k
    };
    HMAC_SECRET_ENC.iter().enumerate().map(|(i, &b)| {
        b ^ key[i % 32]
    }).collect()
}

// Server URL encrypted with derive_key(SEED_A || "url" || SEED_B)
const SERVER_URL_ENC: [u8; 38] = [
    0xF8, 0x10, 0x85, 0x50, 0x6E, 0xFA, 0x99, 0x6C,
    0xA8, 0x17, 0x82, 0x77, 0xEC, 0xAD, 0xA2, 0xA4,
    0xB7, 0xA2, 0x3E, 0x5B, 0x9B, 0x50, 0x0B, 0xC1,
    0xBF, 0x28, 0x3E, 0xF6, 0x7E, 0x32, 0x4C, 0x0C,
    0xFC, 0x0D, 0x95, 0x41, 0x69, 0xA5,
];

fn get_server_url() -> String {
    // Key = SHA256(SEED_A || "url" || SEED_B) — yet another derivation
    let key = {
        use sha2::Digest;
        let mut hasher = Sha256::new();
        hasher.update(&SEED_A);
        hasher.update(b"url");
        hasher.update(&SEED_B);
        let result = hasher.finalize();
        let mut k = [0u8; 32];
        k.copy_from_slice(&result);
        k
    };
    let decrypted: Vec<u8> = SERVER_URL_ENC.iter().enumerate().map(|(i, &b)| {
        b ^ key[i % 32]
    }).collect();
    String::from_utf8(decrypted).unwrap_or_default()
}

// ═══════════════════════════════════════════════════════════════
// Commands
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub async fn validate_license_remote(
    key: String,
    hwid: String,
    state: State<'_, LicenseState>,
) -> Result<serde_json::Value, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Network client error: {}", e))?;

    let payload = json!({ "key": key, "hwid": hwid });
    let server_url = get_server_url();

    let response = client.post(&server_url)
        .json(&payload)
        .header("User-Agent", "RustOpti-Secure-Client/2.2")
        .send()
        .await
        .map_err(|e| format!("Connection error: {}", e))?;

    let signature = response.headers()
        .get("X-Signature")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let body = response.text().await.map_err(|e| format!("Read error: {}", e))?;

    // Verify HMAC signature from server
    let secret = get_hmac_secret();
    if let Some(sig_hex) = signature {
        let mut mac = HmacSha256::new_from_slice(&secret)
            .map_err(|_| "HMAC init error")?;
        mac.update(body.as_bytes());

        let sig_bytes = hex::decode(&sig_hex).map_err(|_| "Invalid signature format")?;
        if mac.verify_slice(&sig_bytes).is_err() {
            return Err("CRITICAL: Server signature is invalid!".to_string());
        }
    } else {
        #[cfg(not(debug_assertions))]
        return Err("Error: Missing server signature.".to_string());
    }

    let v: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Parse error: {}", e))?;

    if v.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
        activate_license(&state);
        store_key_in_memory(&state, &key);
        save_license_cache(&hwid, Some(&key));
        // Store subscription info (plan + expiry) for the account page
        let plan = v.get("plan").and_then(|p| p.as_str()).unwrap_or("monthly");
        let expires_at = v.get("expires_at").and_then(|e| e.as_str());
        store_sub_info(&state, plan, expires_at);
    }

    Ok(v)
}

/// Check license — now uses canary-verified check, not just Mutex
#[tauri::command]
pub fn check_license_status(state: State<'_, LicenseState>) -> bool {
    is_license_valid(&state)
}

/// Get cache status: "valid" | "needs_recheck" | "expired"
#[tauri::command]
pub fn get_license_cache_status() -> String {
    get_cache_status().to_string()
}

/// Revalidate license against the server (called periodically by frontend).
/// Uses the key stored in Rust memory — frontend never sees the key.
/// If server confirms → refresh cache timestamp.
/// If server rejects → deactivate license.
/// If server unreachable → keep current state (grace period handles expiry).
#[tauri::command]
pub async fn revalidate_license(
    state: State<'_, LicenseState>,
) -> Result<serde_json::Value, String> {
    let key = match get_stored_key(&state) {
        Some(k) => k,
        None => return Ok(json!({ "status": "no_key", "message": "No key in memory, restart required" })),
    };

    let hwid = crate::utils::hwid::get_hwid();

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Network error: {}", e))?;

    let payload = json!({ "key": key, "hwid": hwid });
    let server_url = get_server_url();

    let response = match client.post(&server_url)
        .json(&payload)
        .header("User-Agent", "RustOpti-Secure-Client/2.2")
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => {
            return Ok(json!({ "status": "offline", "message": "Server unreachable, using cached license" }));
        }
    };

    // Verify HMAC signature (same as validate_license_remote)
    let signature = response.headers()
        .get("X-Signature")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let body = response.text().await.unwrap_or_default();

    let secret = get_hmac_secret();
    if let Some(sig_hex) = signature {
        let mut mac = HmacSha256::new_from_slice(&secret)
            .map_err(|_| "HMAC init error")?;
        mac.update(body.as_bytes());
        let sig_bytes = hex::decode(&sig_hex).unwrap_or_default();
        if mac.verify_slice(&sig_bytes).is_err() {
            return Ok(json!({ "status": "offline", "message": "Invalid server signature" }));
        }
    } else {
        #[cfg(not(debug_assertions))]
        return Ok(json!({ "status": "offline", "message": "Missing server signature" }));
    }

    let v: serde_json::Value = serde_json::from_str(&body)
        .unwrap_or(json!({ "success": false }));

    if v.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
        // Server confirmed — refresh cache timestamp, preserve key
        let stored_key = get_stored_key(&state);
        touch_license_cache(&hwid, stored_key.as_deref());
        // Update subscription info if server returned it
        if let Some(plan) = v.get("plan").and_then(|p| p.as_str()) {
            let expires_at = v.get("expires_at").and_then(|e| e.as_str());
            store_sub_info(&state, plan, expires_at);
        }
        Ok(json!({ "status": "valid", "message": "License confirmed" }))
    } else {
        // Server rejected — subscription expired or key revoked
        deactivate_license(&state);
        delete_license_cache();
        Ok(json!({ "status": "expired", "message": v.get("error").and_then(|e| e.as_str()).unwrap_or("License expired") }))
    }
}

#[tauri::command]
pub fn revoke_license(state: State<'_, LicenseState>) {
    deactivate_license(&state);
    delete_license_cache();
}

/// Return subscription info (plan + expiry) for the account page
#[tauri::command]
pub fn get_license_info(state: State<'_, LicenseState>) -> serde_json::Value {
    match get_sub_info(&state) {
        Some((plan, expires_at)) => json!({
            "plan": plan,
            "expires_at": expires_at,
            "active": true,
        }),
        None => {
            // sub_info is stored in RAM only and resets on restart.
            // Fall back to the license validity check — if the cached license
            // was restored successfully the user IS active, just without
            // fresh plan details (those are refreshed by revalidate_license).
            let active = is_license_valid(&state);
            json!({
                "active": active,
                "plan": if active { "monthly" } else { "" },
                "expires_at": null
            })
        }
    }
}
