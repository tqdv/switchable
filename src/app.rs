// Application related functions, not the entry point
//
// # Symbols
// - write_aliases, wa::
// - parse_xrandr, ParseXrandrError
prelude!();
use crate::{config, alias, util};
use std::error::Error;
use config::FullConfig;
use std::process::Command;
use regex::Regex;

// For preexec_subcommand
// Metadata is whether a regex failed to compile
pub fn matches_command(config :&FullConfig, s :&str) -> Metadata<bool, bool> {
	let mut matched = false;
	let mut failed = false;
	
	// Find it
	for m in config.match_.iter() {
		let re = match_ok! { Regex::new(m);
			Err(_) => {
				failed = true;
				continue;
			}
		};
		
		if re.is_match(s) {
			matched = true;
			break;
		}
	}
	
	Metadata(failed, matched)
}

// For reload_aliases_subcommand
// This function writes to the terminal through the text function
// Reads the configuration file, and write a new aliases file while
// informing the user of the changes
pub fn reload_aliases<F :Fn(String) -> String> (text :F) -> Result<String, ExitCode> {
	let pln = |s :String| { println!("{}", text(s)) };
	// Macro to handle literal strings as well
	macro_rules! pln {
		($e:expr) => {
			pln(String::from($e))
		}
	}
	
	// Closure because they all depend on pln
	let handle_config_failure = |e :config::Error| -> ExitCode {
		pln!(format!("{}", e));
		if let Some(ee) = e.source() {
			pln!(format!("{}", ee));
		}
		exitcode::BAD_IO
	};
	
	let alias_write_f = |e :alias::wa::Error| -> ExitCode {
		pln!(format!("{}", e));
		exitcode::BAD_IO
	};
	
	let read_alias_f  = |e :alias::ra::Error| {
		pln!(format!("{}", e))
	};
	
	// FIXME: there may be no config
	let config = terror! { config::load_config() => handle_config_failure };
	
	// Tell the user which commands to unalias
	let old_aliases = alias::read_old_aliases()
		.map_err(read_alias_f)
		.ok();
	let (to_remove, to_add) = old_aliases
		.map(|v| util::set_diff(v, config.alias.clone()))
		.split2();
	
	let aliases_file = terror! { alias::write_aliases(&config) => alias_write_f };
	
	if let Some(true) = to_add.map(|v| !v.is_empty()) {
		pln!(format!("New aliases written to '{}'", aliases_file));
	} else {
		pln!(format!("Aliases written to '{}'", aliases_file));
	}
	
	if let Some(to_rm) = to_remove {
		if !to_rm.is_empty() {
			// Single quote and join the aliases strings
			let aliases_str = to_rm.iter()
				.map(|v| format!("'{}'", v))
				.reduce(|a, b| format!("{} {}", a, b)).unwrap(); // We checked is_empty
			pln!(format!("The following aliases have been removed: {}", aliases_str));
			pln!("They are still loaded so unalias them by hand");
		}
	}
	
	Ok(aliases_file)
}

pub fn hook_ran () -> bool {
	std::env::var("SWITCHABLE_RAN").is_ok()
}

// For parse_xrandr
pub mod pax {
	use std::{error, result, io, fmt};
	use fmt::{Display, Debug};
	
	#[derive(Debug)]
	pub struct CommandF(pub io::Error);
	
	impl Display for CommandF {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			write!(f, "Executing the xrandr command failed")
		}
	}
	
	impl error::Error for CommandF {
		fn source (&self) -> Option<&(dyn error::Error + 'static)> {
			Some(&self.0)
		}
	}
	
	pub type Result<T> = result::Result<T, CommandF>;
}

// Assumes, English output of `xrandr --listproviders`.
// Returns, an (possibly empty) vec of (DRI_PRIME value, GPU description) pairs.
// Ignores, providers without descriptions
pub fn parse_xrandr () -> pax::Result<Vec<(String, String)>> {
	let data = terror! { Command::new("xrandr")
		.arg("--listproviders")
		.output() => pax::CommandF
	};
	
	let data = String::from_utf8_lossy(data.stdout.as_slice());
	let data :Vec<&str> = data.split('\n').collect();
	
	let mut ret :Vec<(String, String)> = Vec::new();
	let re = Regex::new(
		r#"(?x) Provider \  (\d+) : .*? name: \  (.*?) (?: ; | $)"#)
		.unwrap();
	for line in data.iter() {
		if let Some(cap) = re.captures(line) {
			let id = cap[1].to_string();
			let desc = cap[2].to_string();
			ret.push((id,desc));
		}
	}
	
	Ok(ret)
}