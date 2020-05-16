// Dealing with the aliases file
//
// This module is separated in two parts: the reading and writing functions.
// Reading functions use the `ra` module (read-aliases)
// and writing ones use `wa` (write-aliases)
// 
// # Symbols
// - read_old_aliases
// - write_aliases
prelude!();
use crate::file;
use crate::config::FullConfig;
use std::io::{self, BufReader, BufRead, BufWriter, Write};
use std::{fs, path::PathBuf, fs::File};
use serde_json;

const JSON_PREFIX :&str = "# Generated from: ";
const NOHOME_FMSG :&str = "Failed to find aliases file because the home directory could not be determined";

pub mod ra {
	use std::{io, result, error, fmt};
	use std::path::PathBuf;
	use fmt::{Display, Debug};
	use super::NOHOME_FMSG;
	use Error::*;
	
	#[derive(Debug)]
	pub enum Error {
		FindFileF,
		NoFileF(PathBuf), // We don't keep the io::Error bc it is unhelpful
		ReadFileF(PathBuf, io::Error), // Remember to mention if the file exists
		NoJsonLine(PathBuf),
		FromJsonF(PathBuf, serde_json::Error),
	}
	
	#[allow(non_snake_case)] // Because we pretend it's a error category
	pub fn IoF(p :PathBuf, e :io::Error) -> Error {
		use io::ErrorKind::*;
		match e.kind() {
			NotFound => NoFileF(p),
			_ => ReadFileF(p, e)
		}
	}
	
	impl Display for Error {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			match self {
				FindFileF => write!(f, "{}", NOHOME_FMSG),
				NoFileF(p) =>
					write!(f, "Aliases file '{}' doesn't exist",
						p.to_string_lossy()),
				ReadFileF(p, _) => 
					write!(f, "Failed to read from aliases file '{}'",
						p.to_string_lossy()),
				NoJsonLine(p) =>
					write!(f, "\"Generated from\" line not found in aliases file '{}'",
						p.to_string_lossy()),
				FromJsonF(p, _) =>
					write!(f, "Failed to parse the list of old aliases as JSON in '{}'",
						p.to_string_lossy()),
			}
		}
	}
	
	impl error::Error for Error {
		fn source (&self) -> Option<&(dyn error::Error + 'static)> {
			match self {
				ReadFileF(_, e) => Some(e),
				FromJsonF(_, e) => Some(e),
				_ => None,
			}
		}
	}

	pub type Result<T> = result::Result<T, self::Error>;
}

fn find_prefix_line(buf :BufReader<File>, path :PathBuf) -> ra::Result<(String, PathBuf)> {
	use ra::Error::*;
	let mut line :Option<String> = None;
	
	// Find it
	for l in buf.lines() {
		let s = terror! { l => |e| ReadFileF(path, e) };
		if s.starts_with(JSON_PREFIX) {
			line = Some(s);
			break;
		}
	}
	let line = terror! { line => |_| NoJsonLine(path) };
	
	Ok((line, path))
}

pub fn read_old_aliases () -> ra::Result<Vec<String>> {
	use ra::Error::*;
	
	let path = terror! { file::find_aliases_file() => |_| FindFileF };
	let alias_file = terror! { File::open(&path) => |e| ra::IoF(path, e) };
	let alias_buf = io::BufReader::new(alias_file);
	
	let (line, path) = rip! { find_prefix_line(alias_buf, path) };
	
	let json_s = &line[JSON_PREFIX.len() .. ]; // May be empty
	let json = terror! { serde_json::from_str(json_s) => |e| FromJsonF(path, e) };
	Ok(json)
}

// ---

pub mod wa {
	use std::{io, fs, result, error, path};
	use std::fmt::{self, Display, Debug};
	use path::PathBuf;
	use super::NOHOME_FMSG;
	use Error::*;
	
	#[derive(Debug)]
	pub enum Error {
		FindFileF,
		WriteFileF(PathBuf, io::Error),
		ToJsonF(PathBuf, serde_json::Error),
		PathNotUtf8(PathBuf),
	}
	
	impl Display for Error {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			match self {
				FindFileF => write!(f, "{}", NOHOME_FMSG),
				WriteFileF(p, _) =>
					write!(f, "Failed to write to aliases file '{}'",
						p.to_string_lossy()),
				ToJsonF(p, _) =>
					write!(f, "Failed to write the list of aliases as JSON in '{}'",
						p.to_string_lossy()),
				PathNotUtf8(p) => write!(f, "Aliases path '{}' is not valid utf8",
					p.to_string_lossy()),
			}
		}
	}
	
	impl error::Error for Error {
		fn source (&self) -> Option<&(dyn error::Error + 'static)> {
			match self {
				WriteFileF(_, e) => Some(e),
				ToJsonF(_, e) => Some(e),
				_ => None,
			}
		}
	}
	
	pub type Result<T> = result::Result<T, self::Error>;
	pub type Buf = io::BufWriter<fs::File>;
}

fn write_aliases_text (file :&mut wa::Buf, config :&FullConfig, mut path :PathBuf)
	-> wa::Result<PathBuf>
{
	use wa::Error::*;
	use serde_json as json;
	
	fn write (b :&mut wa::Buf, s :impl AsRef<str>, path :PathBuf) -> wa::Result<PathBuf> {
		terror! { b.write_all(s.as_ref().as_bytes()) => |e| WriteFileF(path, e) };
		Ok(path)
	}
	
	macro_rules! w {
		($e:expr) => {
			path = rip! { write(file, $e, path) };
		}
	}
	
	let alias_json = terror! { json::to_string(&config.alias) => |e| ToJsonF(path, e) };
	
	// Start writing
	w!("# Generated by switchable, modifications will be overwritten\n");
	
	// Used for keeping track of previous aliases to tell the user
	// what commands to unalias
	w!(format!("{}{}\n", JSON_PREFIX, alias_json));
	w!("\n");
	
	for cmd in &config.alias {
		let cmd = cmd.replace("'", r#"'\''"#);
		w!(format!("alias '{cmd}'='DRI_PRIME=1 {cmd}'\n", cmd=cmd));
	}
	
	w!("# End of file");
	Ok(path)
}

// For init_subcommand
// Writes the aliases file if it can. Aborts as soon as it encounters an error
pub fn write_aliases (config :&FullConfig) -> wa::Result<String> {
	use wa::Error::*;
	
	// Find and open file
	let path = terror! { file::find_aliases_file() => |_| FindFileF };
	let file = terror! {
		fs::OpenOptions::new()
		.write(true).create(true)
		.open(&path) => |e| WriteFileF(path, e)
	};
	let mut file = BufWriter::new(file);
	
	// Write text
	let path = rip! { write_aliases_text(&mut file, &config, path) };
	terror! { file.flush() => |e| WriteFileF(path, e) };

	// Return path as valid String
	let path = terror! {
		path.into_os_string().into_string() => |e| PathNotUtf8(PathBuf::from(e))
	};
	Ok(path)
}