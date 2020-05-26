# switchable

A command-line tool to enable switchable graphics for certain commands.

You won't need to type `DRI_PRIME=1 steam` again.

## Usage

Write the following to the configuration file `~/.config/switchable/config.toml`.

```json
"match": [ "steam" ]
```
And then just run a command that matches.
```bash
steam
```
And it will automatically use your discrete GPU. *(Read below for requirements)*

## Requirements

* bash
* [bash-preexec][bash-preexec]

[bash-preexec]: https://github.com/rcaloras/bash-preexec

## Installation

* `cargo install switchable`
* Add `eval "$( switchable init )"` to your `.bashrc`

## Configuration

We first look at `~/.config/switchable/config.toml`, and if that doesn't exist,
we try `~/.switchable/config.toml`.

The configuration is a TOML file with the following keys:

```toml
# Default value for DRI_PRIME
driver = 1
# Path to bash-preexec if it's not in its default location
preexec = "/home/user/.bash-preexec.sh"

# Regexes to match commands against
match = [
    "steam",
    "echo",
]

# Commands to alias
alias = [     
    "glxgears",
]
```

## Caveats

`switchable run` doesn't work with aliases such as `ll`.

Doesn't work with pipes or &&-chained commands unless you use preexec,
in which case there may be false positives.

## See also

* [Arch Wiki page about PRIME](https://wiki.archlinux.org/index.php/PRIME), used for switchable graphics
* [Wikipedia page about GPU switching](https://en.wikipedia.org/wiki/GPU_switching)

## License

This software is copyright (c) 2019 by Tilwa Qendov.

This is free software, licensed under the [Artistic License 2.0](LICENSE).
