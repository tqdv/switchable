// Loading the configuration file
//
// # Synopsis
// ```rust
// use crate::config;
// let config = match config::load_config() {
// 	Ok(v) => v,
// 	Err(e) => handle_config_error(e),
// }
// ```
//
// # Symbols
// - RawConfig, FullConfig
// - load_config, load_config_meta
//
// # Implementation details
// We use match_ instead of match, because match is a keyword,
// and would need to be quoted as r#match, which is annoying

prelude!();
use crate::file;
use std::{error, io};
use std::fmt::{self, Debug, Display};
use std::{path::PathBuf, fs::File, io::Read};
use serde::Deserialize;
use self::Error::*;

// Configuration  metadata
pub struct Meta {
	pub path :PathBuf,
	pub location :file::Location,
}

// The Config before the defaults are applied, mirroring the configuration file
type O<T> = Option<T>;
#[derive(Deserialize, Debug)]
pub struct RawConfig {
	pub driver :O<String>,
	#[serde(rename = "match")] // Use 'match' in the config
	pub match_ :O<Vec<String>>,
	pub alias :O<Vec<String>>,
	pub preexec :O<String>,
}

// The consumable configuration where we limit the amount of optional values.
// IDEA: Implement Default trait, but we wouldn't really use it except in init
#[derive(Debug)]
pub struct FullConfig {
	pub driver :String,
	pub match_ :Vec<String>,
	pub alias :Vec<String>,
	pub preexec :Option<PathBuf>,
}

impl RawConfig {
	// Creates a valid Config object from a RawConfig object by setting defaults
	pub fn set_defaults (self) -> FullConfig {
		use dirs::home_dir;
		
		let preexec = self.preexec
			.map(PathBuf::from)
			.or_else(|| home_dir().map(|v| v.join(".bash-preexec.sh")));
		
		FullConfig {
			driver: self.driver.unwrap_or_else(|| "1".to_string()),
			match_: self.match_.unwrap_or_default(),
			alias: self.alias.unwrap_or_default(),
			preexec,
		}
	}
}

// config::Error
#[derive(Debug)]
pub enum Error {
	FindFileF, // from file::* functions
	NoFileF(PathBuf), // We don't keep the io::Error because it is unhelpful
	ReadFileF(PathBuf, io::Error),
	ParseF(PathBuf, toml::de::Error),
}

impl Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			FindFileF => write!(f, "Failed to find configuration file because the home directory could not be determined"),
			NoFileF(p) =>
				write!(f, "Configuration file '{}' doesn't exist",
					p.to_string_lossy()),
			ReadFileF(p, _) =>
				write!(f, "Failed to read from configuration file '{}'",
					p.to_string_lossy()),
			ParseF(p, _) => 
				write!(f, "Failed to parse configuration file '{}'",
					p.to_string_lossy()),
		}
	}
}

impl error::Error for Error {
	fn source (&self) -> Option<&(dyn error::Error + 'static)> {
		match self {
			ReadFileF(_, e) => Some(e),
			ParseF(_, e) => Some(e),
			_ => None,
		}
	}
}

#[allow(non_snake_case)]
fn IoF (p :PathBuf, e :io::Error) -> self::Error {
	use io::ErrorKind::*;
	match e.kind() {
		NotFound => NoFileF(p),
		_ => ReadFileF(p, e),
	}
}

pub type Result<T> = std::result::Result<T, self::Error>;

// Used by load_config_file and load_config_meta, but mostly for meta to work
fn load_config_file (path :PathBuf) -> Result<RawConfig> {
	// Open file and read contents to string
	let mut str = String::new();
	let mut file = terror! { File::open(&path) => |e| self::IoF(path, e) };
	terror! { file.read_to_string(&mut str) => |e| ReadFileF(path, e) };
	
	// Parse string as a config
	let config :RawConfig = terror! { toml::from_str(&str) => |e| ParseF(path, e) };
	Ok(config)
}

// Returns the RawConfig loaded from disk
pub fn load_config () -> Result<FullConfig> {
	let path = terror! { file::find_config_file() => |_| FindFileF };
	load_config_file(path).map(|v| v.set_defaults())
}

// Loads the config while preserving the metadata of it
// This is useful for checking where the config comes from (eg. in show_config_subcommand)
pub fn load_config_meta () -> Result<Metadata<Meta, RawConfig>> {
	let (path, loc) = terror! { file::find_config_file_meta() => |_| FindFileF };	
	let config = load_config_file(path.clone());
	
	let meta = Meta {
		path,
		location: loc,
	};
	
	config.map(|v| Metadata(meta, v))
}