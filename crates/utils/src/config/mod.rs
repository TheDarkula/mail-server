/*
 * Copyright (c) 2023 Stalwart Labs Ltd.
 *
 * This file is part of Stalwart Mail Server.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of
 * the License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 * in the LICENSE file at the top-level directory of this distribution.
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 * You can be released from the requirements of the AGPLv3 license by
 * purchasing a commercial license. Please contact licensing@stalw.art
 * for more details.
*/

pub mod cron;
pub mod ipmask;
pub mod parser;
pub mod utils;

use std::{collections::BTreeMap, time::Duration};

use ahash::AHashMap;
use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct Config {
    #[serde(skip)]
    pub keys: BTreeMap<String, String>,
    pub warnings: AHashMap<String, ConfigWarning>,
    pub errors: AHashMap<String, ConfigError>,
    #[cfg(debug_assertions)]
    #[serde(skip)]
    pub keys_read: parking_lot::Mutex<ahash::AHashSet<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type")]
pub enum ConfigWarning {
    Missing,
    AppliedDefault { default: String },
    Unread { value: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type")]
pub enum ConfigError {
    Parse { error: String },
    Build { error: String },
    Macro { error: String },
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ConfigKey {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Rate {
    pub requests: u64,
    pub period: Duration,
}

pub type Result<T> = std::result::Result<T, String>;

impl Config {
    pub async fn resolve_macros(&mut self) {
        for macro_class in ["env", "file", "cfg"] {
            self.resolve_macro_type(macro_class).await;
        }
    }

    async fn resolve_macro_type(&mut self, class: &str) {
        let macro_start = format!("%{{{class}:");
        let mut replacements = AHashMap::new();
        'outer: for (key, value) in &self.keys {
            if value.contains(&macro_start) && value.contains("}%") {
                let mut result = String::with_capacity(value.len());
                let mut snippet: &str = value.as_str();

                loop {
                    if let Some((suffix, macro_name)) = snippet.split_once(&macro_start) {
                        if !suffix.is_empty() {
                            result.push_str(suffix);
                        }
                        if let Some((location, rest)) = macro_name.split_once("}%") {
                            match class {
                                "cfg" => {
                                    if let Some(value) = replacements
                                        .get(location)
                                        .or_else(|| self.keys.get(location))
                                    {
                                        result.push_str(value);
                                    } else {
                                        self.errors.insert(
                                            key.clone(),
                                            ConfigError::Macro {
                                                error: format!("Unknown key {location:?}"),
                                            },
                                        );
                                    }
                                }
                                "env" => match std::env::var(location) {
                                    Ok(value) => {
                                        result.push_str(&value);
                                    }
                                    Err(_) => {
                                        self.errors.insert(
                                                key.clone(),
                                                ConfigError::Macro { error : format!(
                                                    "Failed to obtain environment variable {location:?}"
                                                )},
                                            );
                                    }
                                },
                                "file" => {
                                    let file_name = location.strip_prefix("//").unwrap_or(location);
                                    match tokio::fs::read(file_name).await {
                                        Ok(value) => match String::from_utf8(value) {
                                            Ok(value) => {
                                                result.push_str(&value);
                                            }
                                            Err(err) => {
                                                self.errors.insert(
                                                    key.clone(),
                                                    ConfigError::Macro {
                                                        error: format!(
                                                        "Failed to read file {file_name:?}: {err}"
                                                    ),
                                                    },
                                                );
                                                continue 'outer;
                                            }
                                        },
                                        Err(err) => {
                                            self.errors.insert(
                                                key.clone(),
                                                ConfigError::Macro {
                                                    error: format!(
                                                        "Failed to read file {file_name:?}: {err}"
                                                    ),
                                                },
                                            );
                                            continue 'outer;
                                        }
                                    }
                                }
                                _ => {
                                    unreachable!()
                                }
                            };

                            snippet = rest;
                        }
                    } else {
                        result.push_str(snippet);
                        break;
                    }
                }

                replacements.insert(key.clone(), result);
            }
        }

        if !replacements.is_empty() {
            for (key, value) in replacements {
                self.keys.insert(key, value);
            }
        }
    }

    pub fn update(&mut self, settings: Vec<(String, String)>) {
        self.keys.extend(settings);
    }

    pub fn log_errors(&self, use_stderr: bool) {
        for (key, err) in &self.errors {
            let message = match err {
                ConfigError::Parse { error } => {
                    format!("Failed to parse setting {key:?}: {error}")
                }
                ConfigError::Build { error } => {
                    format!("Build error for key {key:?}: {error}")
                }
                ConfigError::Macro { error } => {
                    format!("Macro expansion error for setting {key:?}: {error}")
                }
            };
            if !use_stderr {
                tracing::error!("{}", message);
            } else {
                eprintln!("ERROR: {message}");
            }
        }
    }

    pub fn log_warnings(&mut self, use_stderr: bool) {
        #[cfg(debug_assertions)]
        self.warn_unread_keys();

        for (key, warn) in &self.warnings {
            let message = match warn {
                ConfigWarning::AppliedDefault { default } => {
                    format!("WARNING: Missing setting {key:?}, applied default {default:?}")
                }
                ConfigWarning::Missing => {
                    format!("WARNING: Missing setting {key:?}")
                }
                ConfigWarning::Unread { value } => {
                    format!("WARNING: Unused setting {key:?} with value {value:?}")
                }
            };
            if !use_stderr {
                tracing::debug!("{}", message);
            } else {
                eprintln!("{}", message);
            }
        }
    }
}

impl Clone for Config {
    fn clone(&self) -> Self {
        Self {
            keys: self.keys.clone(),
            warnings: self.warnings.clone(),
            errors: self.errors.clone(),
            #[cfg(debug_assertions)]
            keys_read: Default::default(),
        }
    }
}

impl PartialEq for Config {
    fn eq(&self, other: &Self) -> bool {
        self.keys == other.keys && self.warnings == other.warnings && self.errors == other.errors
    }
}

impl Eq for Config {}

impl From<(String, String)> for ConfigKey {
    fn from((key, value): (String, String)) -> Self {
        Self { key, value }
    }
}

impl From<(&str, &str)> for ConfigKey {
    fn from((key, value): (&str, &str)) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
        }
    }
}

impl From<(&str, String)> for ConfigKey {
    fn from((key, value): (&str, String)) -> Self {
        Self {
            key: key.to_string(),
            value,
        }
    }
}

impl From<(String, &str)> for ConfigKey {
    fn from((key, value): (String, &str)) -> Self {
        Self {
            key,
            value: value.to_string(),
        }
    }
}
