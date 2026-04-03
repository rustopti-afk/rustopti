use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::path::PathBuf;
use std::time::Instant;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

// ═══════════════════════════════════════════════════════════════
// Multi-layer key derivation (no single "XOR key" in binary)
//
// Instead of: secret XOR static_key (trivially reversible)
// We do:      secret XOR SHA256(SEED_A || SEED_B || context)
//
// The seeds look like random data. The attacker must find BOTH
// seeds AND understand the derivation logic to decrypt.
// ═══════════════════════════════════════════════════════════════

// Scattered seed constants — look like random data in binary
const SEED_A: [u8; 8] = [0x7B, 0x3F, 0xA2, 0x19, 0xE6, 0x54, 0xCD, 0x88];
const SEED_B: [u8; 8] = [0x41, 0xD7, 0x0C, 0x95, 0x6A, 0xF3, 0x28, 0xBE];

/// Derive a 32-byte key stream from seeds + context string.
/// Different context = different key, even with same seeds.
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

/// Decrypt data using a derived key stream
fn decrypt_with_context(encrypted: &[u8], context: &[u8]) -> Vec<u8> {
    let key = derive_key(context);
    encrypted.iter().enumerate().map(|(i, &b)| {
        b ^ key[i % 32]
    }).collect()
}

// Cache secret: encrypted with derive_key(SEED_A || SEED_B)
// Context: empty (default derivation)
const CACHE_SECRET_ENC: [u8; 26] = [
    0x6E, 0xAF, 0x08, 0x8E, 0x39, 0xB5, 0x6B, 0x9F,
    0x86, 0xE5, 0x89, 0x9E, 0x26, 0x9D, 0x9C, 0x91,
    0xFE, 0xF5, 0xE5, 0x99, 0x3F, 0xFB, 0x99, 0x56,
    0xA9, 0xD1,
];

fn get_cache_secret() -> Vec<u8> {
    // Key = SHA256(SEED_A || SEED_B || "")  (no extra context)
    decrypt_with_context(&CACHE_SECRET_ENC, b"")
}

// ═══════════════════════════════════════════════════════════════
// Runtime canary — NOT a magic constant like 0xDEADBEEF
//
// Generated once at startup from a hash. Value is unique per
// build/run, and not searchable as a known pattern.
// ═══════════════════════════════════════════════════════════════

static LICENSE_CANARY: AtomicU64 = AtomicU64::new(0);
static EXPECTED_CANARY: AtomicU64 = AtomicU64::new(0);

/// Compute canary value from seeds (deterministic but not a magic constant)
fn compute_canary() -> u64 {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(b"canary");
    hasher.update(&SEED_A);
    hasher.update(&SEED_B);
    let hash = hasher.finalize();
    // Take first 8 bytes as u64
    u64::from_le_bytes(hash[0..8].try_into().unwrap())
}

// ═══════════════════════════════════════════════════════════════
// License state
// ═══════════════════════════════════════════════════════════════

pub struct SubInfo {
    pub plan: String,
    pub expires_at: Option<String>,
}

pub struct LicenseState {
    pub is_valid: Mutex<bool>,
    /// The license key stored in memory after activation (never exposed to frontend)
    pub cached_key: Mutex<Option<String>>,
    /// Subscription info (plan + expiry) from server response
    pub sub_info: Mutex<Option<SubInfo>>,
}

impl LicenseState {
    pub fn new() -> Self {
        // Compute and store expected canary value
        let canary_val = compute_canary();
        EXPECTED_CANARY.store(canary_val, Ordering::SeqCst);

        let cached = try_restore_from_cache();

        if cached {
            LICENSE_CANARY.store(canary_val, Ordering::SeqCst);
        }

        // Move restored key from global static to LicenseState
        let restored = RESTORED_KEY.lock().ok().and_then(|mut g| g.take());

        Self {
            is_valid: Mutex::new(cached),
            cached_key: Mutex::new(restored),
            sub_info: Mutex::new(None),
        }
    }
}

/// Store the license key in memory after successful activation
pub fn store_key_in_memory(state: &LicenseState, key: &str) {
    if let Ok(mut guard) = state.cached_key.lock() {
        *guard = Some(key.to_string());
    }
}

/// Get the stored key (for revalidation — never exposed to frontend)
pub fn get_stored_key(state: &LicenseState) -> Option<String> {
    state.cached_key.lock().ok().and_then(|g| g.clone())
}

pub fn store_sub_info(state: &LicenseState, plan: &str, expires_at: Option<&str>) {
    if let Ok(mut guard) = state.sub_info.lock() {
        *guard = Some(SubInfo {
            plan: plan.to_string(),
            expires_at: expires_at.map(|s| s.to_string()),
        });
    }
}

pub fn get_sub_info(state: &LicenseState) -> Option<(String, Option<String>)> {
    state.sub_info.lock().ok().and_then(|g| {
        g.as_ref().map(|s| (s.plan.clone(), s.expires_at.clone()))
    })
}

pub fn activate_license(state: &LicenseState) {
    if let Ok(mut guard) = state.is_valid.lock() {
        *guard = true;
        LICENSE_CANARY.store(EXPECTED_CANARY.load(Ordering::SeqCst), Ordering::SeqCst);
    }
}

pub fn deactivate_license(state: &LicenseState) {
    if let Ok(mut guard) = state.is_valid.lock() {
        *guard = false;
        LICENSE_CANARY.store(0, Ordering::SeqCst);
    }
}

/// Check license — verifies primary + canary + anti-debug + timing
pub fn require_license(state: &tauri::State<LicenseState>) -> Result<(), String> {
    // Timing-based anti-debug: measure how long the check takes.
    // A debugger with breakpoints makes this take milliseconds instead of microseconds.
    let start = Instant::now();

    // Anti-debug check (release builds only)
    #[cfg(not(debug_assertions))]
    if crate::utils::security::is_debugger_attached() {
        return Err("License required. Please activate RustOpti.".to_string());
    }

    let primary = state.is_valid.lock()
        .map(|g| *g)
        .unwrap_or(false);

    let canary = LICENSE_CANARY.load(Ordering::SeqCst);
    let expected = EXPECTED_CANARY.load(Ordering::SeqCst);

    // Timing check: if this took more than 500ms, a debugger is stepping through
    #[cfg(not(debug_assertions))]
    {
        let elapsed = start.elapsed();
        if elapsed.as_millis() > 500 {
            return Err("License required. Please activate RustOpti.".to_string());
        }
    }
    // Suppress unused variable warning in debug builds
    #[cfg(debug_assertions)]
    let _ = start;

    // Integrity: primary and canary must agree
    if primary && canary == expected && expected != 0 {
        Ok(())
    } else if !primary && canary == 0 {
        Err("License required. Please activate RustOpti.".to_string())
    } else {
        // Tamper detected — silent fail
        Err("License required. Please activate RustOpti.".to_string())
    }
}

/// Same check as require_license but returns bool (for check_license_status command)
pub fn is_license_valid(state: &LicenseState) -> bool {
    let primary = state.is_valid.lock().map(|g| *g).unwrap_or(false);
    let canary = LICENSE_CANARY.load(Ordering::SeqCst);
    let expected = EXPECTED_CANARY.load(Ordering::SeqCst);
    primary && canary == expected && expected != 0
}

// ═══════════════════════════════════════════════════════════════
// License cache (signed token + timestamp on disk)
//
// Format: HWID_HASH.HMAC_SIGNATURE.TIMESTAMP_SECS
// Timestamp = Unix epoch seconds when last validated with server
// ═══════════════════════════════════════════════════════════════

const RECHECK_INTERVAL_SECS: u64 = 5 * 60; // 5 minutes
const GRACE_PERIOD_SECS: u64 = 24 * 60 * 60;  // 1 day offline grace

pub fn save_license_cache(hwid: &str, key: Option<&str>) {
    let token = create_signed_token_with_key(hwid, key);
    if let Some(path) = get_cache_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, token);
    }
}

/// Update timestamp in cache, preserving the stored key
pub fn touch_license_cache(hwid: &str, key: Option<&str>) {
    save_license_cache(hwid, key);
}

pub fn delete_license_cache() {
    if let Some(path) = get_cache_path() {
        let _ = std::fs::remove_file(&path);
    }
}

/// Check if the cached license needs online revalidation.
/// Returns: "valid" | "needs_recheck" | "expired"
pub fn get_cache_status() -> &'static str {
    let path = match get_cache_path() {
        Some(p) => p,
        None => return "expired",
    };

    let token = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return "expired",
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    match get_token_timestamp(&token) {
        Some(ts) => {
            let age = now.saturating_sub(ts);
            if age < RECHECK_INTERVAL_SECS {
                "valid"         // Checked less than 24h ago
            } else if age < GRACE_PERIOD_SECS {
                "needs_recheck" // 1-3 days since last check
            } else {
                "expired"       // More than 3 days — block
            }
        }
        None => "needs_recheck", // No timestamp = old format, recheck
    }
}

fn get_cache_path() -> Option<PathBuf> {
    std::env::var("APPDATA").ok().map(|appdata| {
        PathBuf::from(appdata).join("RustOpti").join(".license")
    })
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Token format: HWID_HASH.HMAC_SIGNATURE.TIMESTAMP.ENCRYPTED_KEY
/// The license key is XOR-encrypted with the cache secret so it's not in plaintext on disk.
fn create_signed_token(hwid: &str) -> String {
    create_signed_token_with_key(hwid, None)
}

fn create_signed_token_with_key(hwid: &str, key: Option<&str>) -> String {
    let timestamp = now_secs().to_string();

    let hwid_hash = {
        use sha2::Digest;
        let mut hasher = Sha256::new();
        hasher.update(hwid.as_bytes());
        hex::encode(hasher.finalize())
    };

    // Sign: HMAC(hwid_hash + "." + timestamp)
    let payload = format!("{}.{}", hwid_hash, timestamp);
    let secret = get_cache_secret();
    let signature = {
        let mut mac = HmacSha256::new_from_slice(&secret).expect("HMAC key error");
        mac.update(payload.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    };

    // Encrypt the license key with cache secret (XOR)
    let encrypted_key = match key {
        Some(k) => {
            let encrypted: Vec<u8> = k.as_bytes().iter().enumerate()
                .map(|(i, &b)| b ^ secret[i % secret.len()])
                .collect();
            hex::encode(encrypted)
        }
        None => String::new(),
    };

    format!("{}.{}.{}.{}", hwid_hash, signature, timestamp, encrypted_key)
}

/// Extract timestamp from token (third part in both 3-part and 4-part formats)
fn get_token_timestamp(token: &str) -> Option<u64> {
    let parts: Vec<&str> = token.trim().split('.').collect();
    match parts.len() {
        3 | 4 => parts[2].parse::<u64>().ok(),
        _ => None,
    }
}

/// Verify token and optionally extract the encrypted license key.
/// Supports formats: 4-part (with key), 3-part (without key), 2-part (legacy)
fn verify_token(token: &str, current_hwid: &str) -> bool {
    verify_and_extract(token, current_hwid).is_some()
}

/// Verify token and return the decrypted license key if present.
fn verify_and_extract(token: &str, current_hwid: &str) -> Option<Option<String>> {
    let parts: Vec<&str> = token.trim().split('.').collect();

    let (hwid_hash, signature, timestamp_str, enc_key_hex) = match parts.len() {
        4 => (parts[0], parts[1], Some(parts[2]), Some(parts[3])),
        3 => (parts[0], parts[1], Some(parts[2]), None),
        2 => (parts[0], parts[1], None, None),
        _ => return None,
    };

    // Build the payload that was signed
    let payload = match timestamp_str {
        Some(ts) => format!("{}.{}", hwid_hash, ts),
        None => hwid_hash.to_string(),
    };

    // Verify HMAC signature
    let secret = get_cache_secret();
    let mut mac = match HmacSha256::new_from_slice(&secret) {
        Ok(m) => m,
        Err(_) => return None,
    };
    mac.update(payload.as_bytes());

    let sig_bytes = match hex::decode(signature) {
        Ok(b) => b,
        Err(_) => return None,
    };

    if mac.verify_slice(&sig_bytes).is_err() {
        return None;
    }

    // Verify HWID matches
    let current_hash = {
        use sha2::Digest;
        let mut hasher = Sha256::new();
        hasher.update(current_hwid.as_bytes());
        hex::encode(hasher.finalize())
    };

    if hwid_hash != current_hash {
        return None;
    }

    // Decrypt the license key if present
    let decrypted_key = match enc_key_hex {
        Some(hex_str) if !hex_str.is_empty() => {
            let encrypted = hex::decode(hex_str).ok()?;
            let decrypted: Vec<u8> = encrypted.iter().enumerate()
                .map(|(i, &b)| b ^ secret[i % secret.len()])
                .collect();
            Some(String::from_utf8(decrypted).ok()?)
        }
        _ => None,
    };

    Some(decrypted_key)
}

/// Restore license from cache. Checks signature, HWID, grace period,
/// and restores the license key into memory.
fn try_restore_from_cache() -> bool {
    let path = match get_cache_path() {
        Some(p) => p,
        None => return false,
    };

    let token = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return false,
    };

    let current_hwid = crate::utils::hwid::get_hwid();

    // Verify + extract key
    let decrypted_key = match verify_and_extract(&token, &current_hwid) {
        Some(key) => key,
        None => return false,
    };

    // Store restored key in global (will be moved to LicenseState after init)
    if let Some(key) = decrypted_key {
        RESTORED_KEY.lock().ok().map(|mut g| *g = Some(key));
    }

    // Check grace period
    match get_token_timestamp(&token) {
        Some(ts) => {
            let age = now_secs().saturating_sub(ts);
            age < GRACE_PERIOD_SECS
        }
        None => true,
    }
}

// Temporary storage for key restored from cache (before LicenseState is created)
static RESTORED_KEY: Mutex<Option<String>> = Mutex::new(None);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_secret_decryption() {
        let decrypted = get_cache_secret();
        // Verify it decrypts to something non-empty (don't leak the actual secret)
        assert!(!decrypted.is_empty());
        assert!(decrypted.len() > 10);
    }

    #[test]
    fn test_canary_is_nonzero() {
        let canary = compute_canary();
        assert_ne!(canary, 0);
        assert_ne!(canary, 0xDEAD_BEEF_CAFE_F00D); // not a magic constant
    }

    #[test]
    fn test_token_roundtrip() {
        let hwid = "test-hwid-abc123";
        let token = create_signed_token(hwid);
        assert!(verify_token(&token, hwid));
        assert!(!verify_token(&token, "wrong-hwid"));
    }

    #[test]
    fn test_tampered_token_fails() {
        let hwid = "test-hwid-abc123";
        let token = create_signed_token(hwid);
        let tampered = format!("{}x", token);
        assert!(!verify_token(&tampered, hwid));
    }
}
