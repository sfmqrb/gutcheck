# sourced by demo.tape: colorful prompt + caption helper
PS1=$'\[\e[1;35m\]❯\[\e[0m\] '
cap() { printf '\e[1;38;5;213m▍\e[0m \e[1;38;5;117m%s\e[0m\n\n' "$*"; }
banner() { printf '\n\e[1;38;5;213m  gutcheck\e[0m  \e[1;38;5;117mgrep for meaning\e[0m\n\e[2m  a plain-English question, answered locally on CPU\e[0m\n\n'; }
