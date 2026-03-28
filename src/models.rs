use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ==================== Premium Keys ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumKey {
    pub id: String,
    pub key: String,
    pub owner: Option<String>,       // username or email of buyer
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked: bool,
    pub hwid: Option<String>,        // hardware ID lock (set on first use)
    pub last_used: Option<DateTime<Utc>>,
    pub uses: u64,
}

impl PremiumKey {
    pub fn new(key: String, owner: Option<String>, expires_at: Option<DateTime<Utc>>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            key,
            owner,
            created_at: Utc::now(),
            expires_at,
            revoked: false,
            hwid: None,
            last_used: None,
            uses: 0,
        }
    }

    pub fn is_valid(&self) -> bool {
        if self.revoked {
            return false;
        }
        if let Some(expires) = self.expires_at {
            if Utc::now() > expires {
                return false;
            }
        }
        true
    }
}

// ==================== Config Sync ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub username: String,
    pub updated_at: DateTime<Utc>,
    pub modules: serde_json::Value,   // module settings JSON blob
    pub keybinds: serde_json::Value,  // keybind mappings
    pub gui_positions: serde_json::Value, // ClickGUI panel positions
}

// ==================== Cape ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapeEntry {
    pub username: String,
    pub cape_id: String,             // which cape design
    pub custom_url: Option<String>,  // custom cape texture URL
}

// ==================== API Request/Response types ====================

#[derive(Debug, Deserialize)]
pub struct ValidateKeyRequest {
    pub key: String,
    pub username: Option<String>,
    pub hwid: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ValidateKeyResponse {
    pub valid: bool,
    pub premium: bool,
    pub message: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub owner: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateKeyRequest {
    pub key: Option<String>,          // custom key, or auto-generate
    pub owner: Option<String>,
    pub expires_days: Option<i64>,    // days until expiry, None = permanent
}

#[derive(Debug, Serialize)]
pub struct CreateKeyResponse {
    pub id: String,
    pub key: String,
    pub owner: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct SyncConfigRequest {
    pub username: String,
    pub key: String,
    pub modules: Option<serde_json::Value>,
    pub keybinds: Option<serde_json::Value>,
    pub gui_positions: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub download_url: String,
    pub changelog: String,
    pub required: bool,
}

#[derive(Debug, Serialize)]
pub struct AdminStats {
    pub total_keys: usize,
    pub active_keys: usize,
    pub revoked_keys: usize,
    pub expired_keys: usize,
    pub total_validations: u64,
    pub total_configs: usize,
    pub total_capes: usize,
}

#[derive(Debug, Serialize)]
pub struct KeyListEntry {
    pub id: String,
    pub key_preview: String,   // first 4 + last 2 chars
    pub owner: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked: bool,
    pub hwid: Option<String>,
    pub last_used: Option<DateTime<Utc>>,
    pub uses: u64,
    pub status: String,
}
