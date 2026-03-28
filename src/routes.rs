use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::models::*;
use crate::AppState;

// ==================== Public: Key Validation ====================

pub async fn validate_key(
    State(state): State<AppState>,
    Json(req): Json<ValidateKeyRequest>,
) -> Json<ValidateKeyResponse> {
    let mut store = state.write().await;
    store.total_validations += 1;

    let response = match store.find_key_mut(&req.key) {
        Some(key) => {
            if !key.is_valid() {
                let reason = if key.revoked {
                    "Key has been revoked"
                } else {
                    "Key has expired"
                };
                ValidateKeyResponse {
                    valid: false,
                    premium: false,
                    message: reason.to_string(),
                    expires_at: key.expires_at,
                    owner: key.owner.clone(),
                }
            } else {
                // HWID lock: first use binds to hardware
                if let Some(ref hwid) = req.hwid {
                    if let Some(ref stored_hwid) = key.hwid {
                        if stored_hwid != hwid {
                            return Json(ValidateKeyResponse {
                                valid: false,
                                premium: false,
                                message: "Key is locked to another device".to_string(),
                                expires_at: key.expires_at,
                                owner: key.owner.clone(),
                            });
                        }
                    } else {
                        key.hwid = Some(hwid.clone());
                    }
                }

                // Update usage
                key.last_used = Some(Utc::now());
                key.uses += 1;
                if let Some(ref username) = req.username {
                    if key.owner.is_none() {
                        key.owner = Some(username.clone());
                    }
                }

                ValidateKeyResponse {
                    valid: true,
                    premium: true,
                    message: "Premium activated!".to_string(),
                    expires_at: key.expires_at,
                    owner: key.owner.clone(),
                }
            }
        }
        None => ValidateKeyResponse {
            valid: false,
            premium: false,
            message: "Invalid key".to_string(),
            expires_at: None,
            owner: None,
        },
    };

    store.save();
    Json(response)
}

// ==================== Public: Update Check ====================

pub async fn check_update(State(state): State<AppState>) -> Json<UpdateInfo> {
    let store = state.read().await;
    Json(UpdateInfo {
        latest_version: store.latest_version.clone(),
        download_url: store.download_url.clone(),
        changelog: store.changelog.clone(),
        required: false,
    })
}

// ==================== Public: Config Sync ====================

pub async fn sync_config(
    State(state): State<AppState>,
    Json(req): Json<SyncConfigRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut store = state.write().await;

    // Verify key
    match store.find_key(&req.key) {
        Some(key) if key.is_valid() => {}
        _ => return Err(StatusCode::UNAUTHORIZED),
    }

    // Find or create config
    let now = Utc::now();
    if let Some(config) = store.configs.iter_mut().find(|c| c.username == req.username) {
        if let Some(modules) = req.modules {
            config.modules = modules;
        }
        if let Some(keybinds) = req.keybinds {
            config.keybinds = keybinds;
        }
        if let Some(gui_positions) = req.gui_positions {
            config.gui_positions = gui_positions;
        }
        config.updated_at = now;
    } else {
        store.configs.push(UserConfig {
            username: req.username.clone(),
            updated_at: now,
            modules: req.modules.unwrap_or(serde_json::json!({})),
            keybinds: req.keybinds.unwrap_or(serde_json::json!({})),
            gui_positions: req.gui_positions.unwrap_or(serde_json::json!({})),
        });
    }

    store.save();
    Ok(Json(serde_json::json!({ "status": "synced", "username": req.username })))
}

pub async fn get_config(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let store = state.read().await;
    match store.find_config(&username) {
        Some(config) => Ok(Json(serde_json::json!({
            "username": config.username,
            "updated_at": config.updated_at,
            "modules": config.modules,
            "keybinds": config.keybinds,
            "gui_positions": config.gui_positions,
        }))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

// ==================== Public: Cape ====================

pub async fn get_cape(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let store = state.read().await;
    match store.find_cape(&username) {
        Some(cape) => Ok(Json(serde_json::json!({
            "username": cape.username,
            "cape_id": cape.cape_id,
            "custom_url": cape.custom_url,
        }))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

// ==================== Admin: Key Management ====================

pub async fn admin_list_keys(State(state): State<AppState>) -> Json<Vec<KeyListEntry>> {
    let store = state.read().await;
    let entries: Vec<KeyListEntry> = store
        .keys
        .iter()
        .map(|k| {
            let status = if k.revoked {
                "revoked".to_string()
            } else if k.expires_at.is_some_and(|exp| Utc::now() > exp) {
                "expired".to_string()
            } else {
                "active".to_string()
            };

            // Show preview: first 4 + "..." + last 2
            let key_preview = if k.key.len() > 6 {
                format!("{}...{}", &k.key[..4], &k.key[k.key.len() - 2..])
            } else {
                k.key.clone()
            };

            KeyListEntry {
                id: k.id.clone(),
                key_preview,
                owner: k.owner.clone(),
                created_at: k.created_at,
                expires_at: k.expires_at,
                revoked: k.revoked,
                hwid: k.hwid.clone(),
                last_used: k.last_used,
                uses: k.uses,
                status,
            }
        })
        .collect();

    Json(entries)
}

pub async fn admin_create_key(
    State(state): State<AppState>,
    Json(req): Json<CreateKeyRequest>,
) -> Json<CreateKeyResponse> {
    let mut store = state.write().await;

    let key_string = req.key.unwrap_or_else(|| generate_key());
    let expires_at = req.expires_days.map(|days| Utc::now() + Duration::days(days));

    let new_key = PremiumKey::new(key_string.clone(), req.owner.clone(), expires_at);
    let id = new_key.id.clone();
    store.keys.push(new_key);
    store.save();

    tracing::info!("Created new key: {} for {:?}", &key_string[..4], req.owner);

    Json(CreateKeyResponse {
        id,
        key: key_string,
        owner: req.owner,
        expires_at,
    })
}

pub async fn admin_revoke_key(
    State(state): State<AppState>,
    Path(key_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut store = state.write().await;

    let key = store.keys.iter_mut().find(|k| k.id == key_id);
    match key {
        Some(k) => {
            k.revoked = true;
            store.save();
            tracing::info!("Revoked key: {}", key_id);
            Ok(Json(serde_json::json!({
                "status": "revoked",
                "id": key_id,
            })))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn admin_stats(State(state): State<AppState>) -> Json<AdminStats> {
    let store = state.read().await;
    let now = Utc::now();

    let active = store.keys.iter().filter(|k| k.is_valid()).count();
    let revoked = store.keys.iter().filter(|k| k.revoked).count();
    let expired = store
        .keys
        .iter()
        .filter(|k| !k.revoked && k.expires_at.is_some_and(|exp| now > exp))
        .count();

    Json(AdminStats {
        total_keys: store.keys.len(),
        active_keys: active,
        revoked_keys: revoked,
        expired_keys: expired,
        total_validations: store.total_validations,
        total_configs: store.configs.len(),
        total_capes: store.capes.len(),
    })
}

// ==================== Helpers ====================

fn generate_key() -> String {
    let segments: Vec<String> = (0..4)
        .map(|_| {
            let uuid = Uuid::new_v4();
            let hex = uuid.to_string().replace('-', "");
            hex[..5].to_uppercase()
        })
        .collect();
    format!("AEGIS-{}", segments.join("-"))
}
