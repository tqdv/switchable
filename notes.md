# Notes

Aliasing a command doesn't change the command name, so we don't have to worry about mistakenly calling the alias instead of the command

## Useful commands

    cargo expand --color=always | less
    cargo clippy --color=always 2>&1 | less
    cargo run --color=always -- init 2> >(perl -pE '($v) = (/warning.*unused/.../^$/) =~ /^(\d+)/; undef $_ if $v >= 3; if ($v == 3) {say}') | less
