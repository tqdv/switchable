// This module contains the functions for each individual subcommand
prelude!();
use crate::{config, file, alias, app, util};
use std::env;
use std::process::Command;
use std::os::unix::process::CommandExt;
use regex::Regex;
use getopts;

// Glossary:
// * p_name: Program name

const DRI_PRIME :&str = "DRI_PRIME"; // The env variable to set
const INIT_NAME :&str = "switchable"; // Name used in init and preexec hooks

// Entry point, dispatches to the right subcommand
pub fn execute (p_name :&str, args :Vec<String>) -> ExitCode {
	let n_args = &args[1..];
	
	match args[0].as_str() {
		"_test" => test_func(),
		"run" => run_subcommand(p_name, n_args),
		"init" => init_subcommand(),
		"preexec" => preexec_subcommand(n_args),
		"precmd" => precmd_subcommand(),
		"xrandr" => xrandr_subcommand(p_name, n_args),
		"show-config" => show_config_subcommand(),
		"reload-aliases" => reload_aliases_subcommand(),
		v => {
			eprintln!(r#"Unknown subcommand given: "{}", see --help"#, v);
			exitcode::BAD_ARG
		}
	}
}

fn test_func () -> i32 {
	println!("The test function works!");
	0
}

// COMMANDS

fn run_subcommand (p_name :&str, args :&[String]) -> ExitCode {
	fn print_help (p_name :&str) {
		print!(
r#"Usage:
  {p_name} run [options] <command>

Options:
  --help, -h             Display this help text
  --driver, -d <string>  The value of DRI_PRIME
  --expand               Pass the command as a string to eval
"#,
		p_name = p_name);
	}

	fn create_parser () -> getopts::Options {
		let mut parser = getopts::Options::new();
		// Avoid consuming the switches belonging to the command to run
		parser.parsing_style(getopts::ParsingStyle::StopAtFirstFree);
		parser.optflag("h", "help", "");
		parser.optflag("", "expand", "");
		parser.optopt("d", "driver", "", "");
		parser
	}
	
	fn get_config_driver () -> Option<String> {
		match config::load_config() {
			Ok(config) => Some(config.driver),
			Err(e) => {
				eprintln!("{}", e);
				None
			},
		}
	}
	
	fn parser_f (e :getopts::Fail) -> ExitCode {
		eprint!("{}", e.to_string());
		exitcode::BAD_ARG
	}
	
	// Parse switches
	let parser = create_parser();
	let opts = fear! { parser.parse(args) => parser_f };
	let args = &opts.free;
	
	// Handle arguments
	tear_if! { opts.opt_present("help") || args.is_empty(),
		// No command to run
		print_help(p_name);
		exitcode::OK
	}
	
	// Modify env
	let driver = opts.opt_str("driver").or_else(get_config_driver);
	let driver :&str = driver.as_deref().unwrap_or("1");
	env::set_var(DRI_PRIME, driver);
	
	// Execute the command
	// See docs/bash_splitting for details on how the arguments are handled
	let mut command;
	if opts.opt_present("expand") {
		// Pass the string to sh to perform shell expansion
		command = Command::new("sh");
		command.arg("-c").arg(args.join(" "));
	} else {
		// Otherwise, keep the arguments as is
		command = Command::new(&args[0]);
		command.args(args[1..].iter());
	};
	command.exec();
	
	exitcode::OK // This will never be reached because we exec
}

// Prints out shell code to load aliases
fn init_subcommand () -> ExitCode {
	use config::FullConfig;
	use std::path::PathBuf;
	
	fn setup_preexec (path :&Option<PathBuf>) {
		fn path_not_utf8 () {
			eprintln!("bash_preexec path is not utf8")
		}
		
		// Get path
		tear_if! { path.is_none(), }
		let path = path.as_ref().unwrap();
		tear_if! { !path.exists(),
			eprintln!("Could not find bash_preexec")
		}
		
		// Load preexec
		let path = fear! { path.to_str() => |_| path_not_utf8() };
		println!("source \"{}\"", path);
		
		// Define hooks. '{{' and '}}' are for escaping
		print!(r#"
sw_preexec() {{ eval "$( {pn} preexec "$1" )"; }}
sw_precmd() {{ eval "$( {pn} precmd "$1" )"; }}
preexec_functions+=(sw_preexec)
precmd_functions+=(sw_precmd)
"#, pn=INIT_NAME
		);
	}
	
	fn setup_aliases (config :&FullConfig){
		match alias::write_aliases(config) {
			Ok(file) => {
				let file = file.replace("'", r#"'\''"#);
				println!("source '{}'", file);
			},
			Err(e) => {
				eprintln!("{}", e);
			}
		}
	}
	
	// Load config or die
	let config = fear! { 
		config::load_config() => |e| { eprintln!("{}", e); exitcode::FAIL }
	};
	
	// Load bash-preexec and write aliases
	setup_preexec(&config.preexec);
	println!();
	setup_aliases(&config);
	
	println!("\nexport SWITCHABLE_EXISTS=1");
	exitcode::OK
}

// NB the output of this command is executed in the shell only when there's a command
// Make sure eveything is quoted properly !
fn preexec_subcommand (args :&[String]) -> ExitCode {
	#![allow(clippy::print_literal)]
	use util::shell_escape;
	use config::FullConfig;
	
	fn load_config_f (x :config::Result<FullConfig>)
		-> ValRet<Option<FullConfig>, ExitCode>
	{
		use config::Error::*;
		match x {
			Ok(v) => Val(Some(v)),
			Err(NoFileF(..)) => Val(None),
			Err(_) => {
				println!(r#"echo 'Failed to load {} config' >&2"#, INIT_NAME);
				Ret(exitcode::FAIL)
			}
		}
	}
	
	// No warnings as this executed at every command entered in the shell
	let command = fear! { args.get(0) => |_| exitcode::MISSING_ARG };
	
	// Process `switchable reload-aliases`
	let reload_re = Regex::new(r"(?x)
		^ \s* (?: (?: \w | [/.] )* )? switchable      # switchable
		\s+ reload-aliases      # followed by the `reload-aliases` subcommand")
		.unwrap();
	if reload_re.is_match(&command) {
		let sayf = |v :String| format!("echo {}", shell_escape(&v));
		
		// We don't handle Err as it is already done by reload_aliases
		if let Ok(aliases_file) = app::reload_aliases(sayf) {
			println!("source {}", shell_escape(&aliases_file));
			println!(r#"echo 'Loaded new aliases in this shell'"#);
		}
	}
	
	// Process configured matches
	let config = tear! { load_config_f( config::load_config() ) };
	if let Some(conf) = config {
		// Set DRI_PRIME if needed
		let Metadata(some_failed, matched) = app::matches_command(&conf, command);
		if matched {
			print!("{}",
r#"if [ -n "${DRI_PRIME+x}" ]
then
	export SWITCHABLE_DP_BAK="$DRI_PRIME"
fi
"#
			);
			println!("export DRI_PRIME={}", util::shell_escape(&conf.driver));
		}
		
		// If regex are invalid, warn but keep it short
		if some_failed {
			println!(r#"echo '{pn}: Invalid regex found, see `{pn} show-config`' >&2"#, pn=INIT_NAME);
		}
	}
	
	println!("export SWITCHABLE_RAN=1");
	exitcode::OK
}

// NB the output of this command is executed in the shell
// NB precmd is executed even if there was nothing entered in the shell
#[allow(clippy::print_literal)]
fn precmd_subcommand () -> ExitCode {
	print!("{}",
r#"unset SWITCHABLE_RAN
unset DRI_PRIME

if [ -n ${SWITCHABLE_DP_BAK+x} ]
then
	DRI_PRIME="$SWITCHABLE_DP_BAK"
	unset SWITCHABLE_DP_BAK
fi
"#
	);
	exitcode::OK
}


// Displays DRI_PRIME values for each GPU by parsing the output of `xrandr --listproviders`
fn xrandr_subcommand (p_name :&str, args :&[String]) -> ExitCode {
	fn parser_handler (e :getopts::Fail) -> ExitCode {
		eprint!("{}", e.to_string());
		exitcode::BAD_ARG
	}
	
	fn command_failed_handler (e: app::pax::CommandF) -> ExitCode {
		eprintln!("Command `xrandr --listproviders` failed to execute:\n{}", e.0);
		exitcode::BAD_IO
	}
	
	// Parser options
	let mut parser = getopts::Options::new();
	parser.optflag("h", "help", "Display help");
	let opts = fear! { parser.parse(args) => parser_handler };
	
	// Print help if needed
	tear_if! { opts.opt_present("help"),
		println!("Usage: {} xrandr", p_name);
		println!();
		println!("Prints the DRI_PRIME values for each GPU based on the output of `xrandr --listproviders`");
		exitcode::OK
	}
	
	// Collect data
	let data = fear! { app::parse_xrandr() => command_failed_handler };
	
	// Output
	println!("DRI_PRIME: description");
	for (id, desc) in data.iter() {
		println!("{}: {}", id, desc);
	}

	exitcode::OK
}

// TODO warn when Regex is invalid
// Display the loaded configuration.
fn show_config_subcommand () -> ExitCode {
	fn handle_config_error(e :config::Error) -> ExitCode {
		use config::Error::*;
		fn print_source (e :config::Error) {
			use std::error::Error;
			if let Some(ee) = e.source() {
				eprintln!("{}", ee)
			}
		}
		
		match e {
			FindFileF | ParseF(..) => {
				eprintln!("{}", e);
				print_source(e);
				exitcode::FAIL
			},
			NoFileF(p) => {
				eprintln!("Configuration file '{}' doesn't exist. Try creating it.",
					p.to_string_lossy());
				exitcode::BAD_IO
			},
			ReadFileF(..) => {
				eprintln!("{}", e);
				print_source(e);
				exitcode::BAD_IO
			},
		}
	}
	
	fn print_matches (r#match :Option<Vec<String>>) {
		if let Some(matches) = r#match {
			println!("Commands matches:");
			if matches.is_empty() {
				println!("  (None defined)")
			} else {
				// Print the list of matches
				for m in matches {
					println!("- {}", m);
				}
			}
		} else {
			println!("No commands matches defined in the 'match' key")
		}
	}
	
	fn print_aliases (alias :Option<Vec<String>>) {
		if let Some(aliases) = alias {
			println!("Aliases:");
			if aliases.is_empty() {
				println!("  (None defined)");
			} else {
				// Print them
				for a in aliases {
					println!("- {}", a)
				}
			}
		} else {
			println!("No aliases defined by the 'alias' key");
		}
	}
	
	// Load config
	let Metadata(meta, config) = fear! {
		config::load_config_meta() => handle_config_error
	};
	println!("Configuration file: {}", meta.path.to_string_lossy());
	
	// Warn if another configuration file was ignored
	if let file::Location::Xdg = meta.location {
		if let Some(path) = file::get_dot_path(file::FileType::Config) {
			if path.exists() {
				println!("  (File \"{}\" was ignored)", path.to_string_lossy());
			}
		}
	}
	
	println!();
	
	// Handle 'preexec'
	if let Some(preexec) = config.preexec {
		println!("Preexec path: {}", preexec);
	}
	
	// Handle 'driver' key
	let driver = config.driver.unwrap_or_else(|| "1 ('driver' not set)".to_string());
	println!("Default GPU id: {}", driver);

	// Handle 'match' and 'alias' keys
	print_matches(config.match_);
	print_aliases(config.alias);

	exitcode::OK
}

// Reloads the aliases by using the preexec hooks if available.
fn reload_aliases_subcommand () -> ExitCode {
	use std::convert::identity;
	
	tear_if! { app::hook_ran(),
		// Do nothing as it has already been done in the preexec hook
		exitcode::OK
	}

	tear! { app::reload_aliases(identity) };
	exitcode::OK
}