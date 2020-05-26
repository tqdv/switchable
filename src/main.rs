#[macro_use] mod slang;
mod util;
mod exitcode;
mod commands;
mod config;
mod alias;
mod file;
mod app;

prelude!();
use std::env;
use std::process::exit;

/// Program version
static VERSION :&str = "0.1.0";

/// Print command-line usage
fn print_usage (p_name: &str) {
	print!(
r#"Usage:
  {p_name} <subcommand> <arguments>
  {p_name} <subcommand> --help
  {p_name} --help | --version

Subcommands:
  run             Enable the GPU for the supplied command
  reload-aliases  Reloads the aliases if possible
  show-config     Displays the loaded configuration
  xrandr          List DRI_PRIME values for each GPU
"#,
	p_name = p_name);
}

/** Everything starts here

Assumes UTF-8 command-line arguments
*/
fn main () {
	fn create_parser () -> getopts::Options {
		let mut parser = getopts::Options::new();
		parser.optflag("h", "help", "");
		parser.optflag("", "version", "");
		parser.parsing_style(getopts::ParsingStyle::StopAtFirstFree);
		parser
	}
	
	let args: Vec<String> = env::args().collect();
	let program_name = args[0].clone();
	
	// Handle command line arguments
	let parser = create_parser();
	let opts = tear! { parser.parse(&args[1..]) => |f :getopts::Fail| {
		eprint!("{}", f.to_string());
		exit(exitcode::BAD_ARG);
	}};
	
	// Handle help and version flags
	let code = {
		if opts.opt_present("help") {
			print_usage(&program_name);
			exitcode::OK
		
		} else if opts.opt_present("version") {
			println!("{} v{}", program_name, VERSION);
			exitcode::OK
		
		} else if opts.free.is_empty() {
			eprintln!("No subcommand given, see --help");
			exitcode::MISSING_ARG
		
		} else {
			commands::execute(&program_name, opts.free)
		}
	};
	
	exit(code);
}
