// Exitcodes for use with std::process::exit
//
// # Synopsis
// ```rust
// mod exitcode;
// use std::process::exit;
// 
// exit( exitcode::OK );
// ```

pub type ExitCode = i32;

pub const FAIL :ExitCode = -1;
pub const OK :ExitCode = 0;
pub const BAD_ARG :ExitCode = 1;
pub const MISSING_ARG :ExitCode = 2;
pub const BAD_IO :ExitCode = 3;