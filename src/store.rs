use crate::models::{PremiumKey, UserConfig, CapeEntry};
use serde::{Deserialize, Serialize};
use std::path::Path;

const DATA_FILE: &str = "aegis-data.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct Store {
    pub keys: Vec<PremiumKey>,
    pub configs: Vec<UserConfig>,
    pub capes: Vec<CapeEntry>,
    pub total_validations: u64,
    pub admin_password: String,
    pub latest_version: String,
    pub download_url: String,
    pub changelog: String,
}

impl Store {
    pub fn load_or_create() -> Self {
        if Path::new(DATA_FILE).exists() {
            match std::fs::read_to_string(DATA_FILE) {
                Ok(data) => match serde_json::from_str(&data) {
                    Ok(store) => {
                        tracing::info!("Loaded {} keys from {}", store_key_count(&store), DATA_FILE);
                        return store;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse {}: {}, creating fresh store", DATA_FILE, e);
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read {}: {}, creating fresh store", DATA_FILE, e);
                }
            }
        }

        let mut store = Self {
            keys: Vec::new(),
            configs: Vec::new(),
            capes: Vec::new(),
            total_validations: 0,
            admin_password: "aegis-admin-2026".to_string(),
            latest_version: "2.0.0".to_string(),
            download_url: "https://aegisclient.github.io/site/aegis-launcher-1.0.0.jar".to_string(),
            changelog: "v2.0.0: Premium system, 142 modules, launcher upgrade".to_string(),
        };

        // Seed the original premium key
        store.keys.push(PremiumKey::new(
            "UltronJARVIS7232!".to_string(),
            Some("arhan".to_string()),
            None, // permanent
        ));

        store.save();
        tracing::info!("Created fresh store with default key");
        store
    }

    pub fn save(&self) {
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = std::fs::write(DATA_FILE, json) {
                    tracing::error!("Failed to save store: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to serialize store: {}", e);
            }
        }
    }

    pub fn find_key(&self, key: &str) -> Option<&PremiumKey> {
        self.keys.iter().find(|k| k.key == key)
    }

    pub fn find_key_mut(&mut self, key: &str) -> Option<&mut PremiumKey> {
        self.keys.iter_mut().find(|k| k.key == key)
    }

    pub fn find_key_by_id(&self, id: &str) -> Option<&PremiumKey> {
        self.keys.iter().find(|k| k.id == id)
    }

    pub fn find_config(&self, username: &str) -> Option<&UserConfig> {
        self.configs.iter().find(|c| c.username == username)
    }

    pub fn find_cape(&self, username: &str) -> Option<&CapeEntry> {
        self.capes.iter().find(|c| c.username == username)
    }
}

fn store_key_count(store: &Store) -> usize {
    store.keys.len()
}
