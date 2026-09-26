# vhs demo/demo.tape   (needs target/release/gutcheck and the model cached: run gutcheck once first)
Output demo/demo.gif
Set Shell "bash"
Set FontSize 17
Set Width 1400
Set Height 700
Set Padding 24
Set Margin 24
Set MarginFill "#282a36"
Set BorderRadius 12
Set WindowBar Colorful
Set Theme "Dracula"
Set TypingSpeed 28ms

Hide
Type "export CLICOLOR_FORCE=1 PATH=$PWD/target/release:$PATH && cd examples && source ../demo/prompt.sh" Enter
Type "clear; banner" Enter
Show
Sleep 2s

Hide
Type "clear; cap 'filter: grep, but the pattern is a question'" Enter
Show
Type `gutcheck -n "is this a bug report?" issues.txt` Enter
Sleep 7.5s

Hide
Type "clear; cap 'logs: 72 lines, 8 model calls, ranked'" Enter
Show
Type `gutcheck --estimate "does this describe an error?" access.log` Enter
Sleep 2.5s
Type `gutcheck -f --top 6 "does this log line describe an error?" access.log` Enter
Sleep 9s

Hide
Type "clear; cap 'diffs: judge each hunk'" Enter
Show
Type `gutcheck --diff "removes a security or permission check" change.diff` Enter
Sleep 8s

Hide
Type "clear; cap 'classify: your own labels, any language'" Enter
Show
Type `gutcheck -c bug,billing,question,praise "what is this message about?" support.txt` Enter
Sleep 8s

Hide
Type "clear; cap 'tally: the shape of a whole file'" Enter
Show
Type `gutcheck -c bug,billing,question,praise --tally "what is this message about?" support.txt` Enter
Sleep 8s
