//! Application related functions, not the entry point
prelude!();
use crate::exitcode::{self, ExitCode};
use crate::{config, alias, util};
use std::error::Error;
use config::FullConfig;
use regex::Regex;

/// Test if the command matches any regex. The metadata is whether a regex failed to compile
pub fn matches_command(config :&FullConfig, s :&str) -> Metadata<bool, bool> {
	let mut failed = false;
	
	// Find it
	let matched = config.match_.iter()
		.any(|e| -> bool {
			let re = tear! { Regex::new(e) => |_| { failed = true; false } }; // TODO improve syntax
			re.is_match(s)
		});
	
	Metadata(failed, matched)
}

/** Writes aliases and reload them by printing shell commands if possible

This function writes to the terminal through the text function.
It reads the configuration file, and write a new aliases file while informing the user
of the changes
*/
pub fn reload_aliases<F :Fn(String) -> String> (text :F) -> Result<String, ExitCode> {
	/// Our printer
	macro_rules! pln {
		($e:expr) => {
			println!("{}", text($e.to_string()))
		}
	}

	// All closures because they depend on the pln which depends on the text function

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

/// Whether the preexec hook has been run
pub fn hook_ran () -> bool {
	std::env::var("SWITCHABLE_RAN").is_ok()
}

/// Module for `parse_xrandr`
pub mod pax {
	prelude!();
	use std::{result, io};
	
	#[derive(ThisError, Debug)]
	#[error("Command `xrandr --listproviders` failed to execute")]
	pub struct CommandF(#[source] pub io::Error);
	
	pub type Result<T> = result::Result<T, CommandF>;
}

/** Returns a vec of (DRI_PRIME value, GPU description) pairs.

Assumes, English output of `xrandr --listproviders`.
Ignores, providers without descriptions
*/
pub fn parse_xrandr () -> pax::Result<Vec<(String, String)>> {
	use std::process::Command;

	let data = terror! {
		Command::new("xrandr")
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
