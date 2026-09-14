use serde::{Deserialize, Serialize, de::Error as DeError, Deserializer, Serializer};
use std::{fs, io, path::PathBuf};
use sdl2::{keyboard::Keycode};
use toml::Value;

mod keycode_serde {
    use std::format;

use super::*;
    pub fn serialize<S>(key: &Keycode, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&key.name())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Keycode, D::Error> where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        Keycode::from_name(&s).ok_or_else(|| DeError::custom(format!("Invalid key: {s}")))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub loaded_rom: Option<PathBuf>,
    pub controls: Controls,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Controls {
    #[serde(with = "keycode_serde")]
    pub a: Keycode,
    #[serde(with = "keycode_serde")]
    pub b: Keycode,
    #[serde(with = "keycode_serde")]
    pub start: Keycode,
    #[serde(with = "keycode_serde")]
    pub select: Keycode,
    #[serde(with = "keycode_serde")]
    pub up: Keycode,
    #[serde(with = "keycode_serde")]
    pub down: Keycode,
    #[serde(with = "keycode_serde")]
    pub left: Keycode,
    #[serde(with = "keycode_serde")]
    pub right: Keycode,
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .expect("Couldn't find config folder")
        .join("project-g")
        .join("config.toml")
}

impl Default for Controls {
    fn default() -> Self {
        Controls {
            a: Keycode::Z,
            b: Keycode::A,
            start: Keycode::RETURN,
            select: Keycode::SPACE,
            up: Keycode::UP,
            down: Keycode::DOWN,
            left: Keycode::LEFT,
            right: Keycode::RIGHT
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            controls: Controls::default(),
            loaded_rom: std::fs::read_dir("./roms/")
                .ok()
                .and_then(|mut entries| entries.next())
                .and_then(|entry| entry.ok())
                .map(|entry| entry.path())
        }
    }

}

impl Controls {
    fn resolve_field(table: &toml::map::Map<String, Value>, field: &str, fallback: Keycode) -> Keycode {
        table
        .get(field)
        .and_then(Value::as_str)
        .and_then(Keycode::from_name)
        .unwrap_or(fallback)
    }

    fn from_toml_value(value: Option<&Value>) -> Self {
        let default = Controls::default();
        let empty = toml::map::Map::new();
        let table = value.and_then(Value::as_table).unwrap_or(&empty);
        Controls {
            a: Self::resolve_field(table, "a", default.a),
            b: Self::resolve_field(table, "b", default.b),
            start: Self::resolve_field(table, "start", default.start),
            select: Self::resolve_field(table, "select", default.select),
            up: Self::resolve_field(table, "up", default.up),
            down: Self::resolve_field(table, "down", default.down),
            left: Self::resolve_field(table, "left", default.left),
            right: Self::resolve_field(table, "right", default.right),
        }
    }
}

impl Config {
    pub fn load_config() -> Self {
        let path = config_path();

        let contents = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(_) => return Config::default(),
        };

        let value: Value = match toml::from_str(&contents) {
            Ok(value) => value,
            Err(_) => return Config::default(),
        };

        let default = Config::default();

        Config { loaded_rom: value
            .get("loaded_rom")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .or(default.loaded_rom),
            controls: Controls::from_toml_value(value.get("controls")),
        }


    }

    pub fn write_rom_config(&mut self, rom_path: PathBuf) -> io::Result<()> {
        self.loaded_rom = Some(rom_path);

        let path = config_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let toml = toml::to_string(self)
            .map_err(io::Error::other)?;

        fs::write(config_path(), toml)?;
        Ok(())
    }
}
