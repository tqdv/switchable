/*! File paths

Everything in this module returns `Option<T>` because getting the home dir can fail.
*/

use std::path::PathBuf;
use dirs::*;

/// Configuration files locations
#[derive(Clone, Copy)]
#[derive(Debug)]
pub enum Location {
	Dot,
	Xdg,
}

/// The type of data file
pub enum FileType {
	Config,
	Aliases,
}

/// Project name, for the folder
const NAME :&str = "switchable";
/// Hidden home directory folder for our files
const DOT_DIR :&str = ".switchable";
/// Configuration file name
const CONFIG_NAME :&str = "config.toml";
/// Aliases file name
const ALIAS_NAME :&str = "aliases.bash";

/// Get file path for a file in the specified location
pub fn get_path (l :Location, name :FileType) -> Option<PathBuf> {	
	match l {
		Location::Dot => get_dot_path(name),
		Location::Xdg => get_xdg_path(name),
	}
}

/// Get filepath for file in Xdg location
pub fn get_xdg_path (name :FileType) -> Option<PathBuf> {
	match name {
		FileType::Config =>
			config_dir().map(|v| v.join(NAME).join(CONFIG_NAME)),
		FileType::Aliases =>
			data_dir().map(|v| v.join(NAME).join(ALIAS_NAME)),
	}
}

/// Get filepath for file in Dot location
pub fn get_dot_path (name :FileType) -> Option<PathBuf> {
	match name {
		FileType::Config =>
			home_dir().map(|v| v.join(DOT_DIR).join(CONFIG_NAME)),
		FileType::Aliases =>
			home_dir().map(|v| v.join(DOT_DIR).join(ALIAS_NAME)),
	}
}

/** Returns the preferred location four our configuration file based on existing ones

It chooses where the configuration file is. If it exists in both Dot and Xdg, it will prefer Xdg.

Returns None if the home directory could not be found.
*/
pub fn preferred_location () -> Option<Location> {
	fn exists (l :Location) -> Option<bool> {
		get_path(l, FileType::Config).map(|v| v.exists())
	}
	
	let xdg = exists(Location::Xdg);
	let dot = exists(Location::Dot);
	
	match (xdg, dot) {
		(Some(x), Some(d)) => {
			if d && !x {
				Some(Location::Dot)
			} else {
				Some(Location::Xdg)
			}
		},
		(Some(_), None) => Some(Location::Xdg),
		(None, Some(_)) => Some(Location::Dot),
		_ => None,
	}
}

/** Find file path specified

Can fail because it couldn't determine home dir
*/
fn find_file (t :FileType) -> Option<PathBuf> {
	preferred_location().and_then(|v| get_path(v, t))
}

/// Get configuration file path
pub fn find_config_file () -> Option<PathBuf> {
	find_file(FileType::Config)
}

/// Get aliases file path
pub fn find_aliases_file () -> Option<PathBuf> {
	find_file(FileType::Aliases)
}

/** Get configuration file path with metadata about the location

Returns a single Option because both depend on the home dir existing
*/
pub fn find_config_file_meta () -> Option<(PathBuf, Location)> {
	match (find_config_file(), preferred_location()) {
		(Some(p), Some(l)) => Some((p, l)),
		_ => None,
	}
}
