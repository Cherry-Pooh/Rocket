use super::Environment::*;
use super::Environment;

use logger::LoggingLevel;
use toml::Value;

use std::collections::HashMap;

#[derive(Debug)]
pub struct Config {
    pub address: String,
    pub port: usize,
    pub log_level: LoggingLevel,
    pub session_key: Option<String>,
    pub extra: HashMap<String, Value>,
}

macro_rules! parse {
    ($val:expr, as_str) => (
        match $val.as_str() {
            Some(v) => v,
            None => return Err("a string")
        }
    );

    ($val:expr, as_integer) => (
        match $val.as_integer() {
            Some(v) => v,
            None => return Err("an integer")
        }
    );
}

impl Config {
    pub fn default_for(env: Environment) -> Config {
        match env {
            Development => {
                Config {
                    address: "localhost".to_string(),
                    port: 8000,
                    log_level: LoggingLevel::Normal,
                    session_key: None,
                    extra: HashMap::new(),
                }
            }
            Staging => {
                Config {
                    address: "0.0.0.0".to_string(),
                    port: 80,
                    log_level: LoggingLevel::Normal,
                    session_key: None,
                    extra: HashMap::new(),
                }
            }
            Production => {
                Config {
                    address: "0.0.0.0".to_string(),
                    port: 80,
                    log_level: LoggingLevel::Critical,
                    session_key: None,
                    extra: HashMap::new(),
                }
            }
        }
    }

    pub fn set(&mut self, name: &str, value: &Value) -> Result<(), &'static str> {
        if name == "address" {
            self.address = parse!(value, as_str).to_string();
        } else if name == "port" {
            self.port = parse!(value, as_integer) as usize;
        } else if name == "session_key" {
            let key = parse!(value, as_str);
            if key.len() != 32 {
                return Err("a 192-bit base64 encoded string")
            }

            self.session_key = Some(key.to_string());
        } else if name == "log" {
            self.log_level = match parse!(value, as_str).parse() {
                Ok(level) => level,
                Err(_) => return Err("log level ('normal', 'critical', 'debug')"),
            };
        } else {
            self.extra.insert(name.into(), value.clone());
        }

        Ok(())
    }
}
