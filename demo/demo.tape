# vhs demo/demo.tape   (needs target/release/gutcheck and the models cached: run gutcheck once with each of -M multilingual and @secret first)
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
Set TypingSpeed 24ms

Hide
Type "export CLICOLOR_FORCE=1 PATH=$PWD/target/release:$PATH && cd examples && source ../demo/prompt.sh" Enter
Type "clear; banner" Enter
Show
Sleep 2s

Hide
Type "clear; cap 'filter: grep, but the pattern is a question'" Enter
Show
Type `gutcheck -n "is this a bug report?" issues.txt` Enter
Sleep 7s

Hide
Type "clear; cap 'named questions: hunt credentials in a repo'" Enter
Show
Type `gutcheck @secret -r repo` Enter
Sleep 14s

Hide
Type "clear; cap 'logs: 72 lines, 8 model calls, ranked'" Enter
Show
Type `gutcheck -f --top 6 "does this log line describe an error?" access.log` Enter
Sleep 8s

Hide
Type "clear; cap 'explore: move the threshold, ask something new'" Enter
Show
Type `gutcheck -i -f "does this log line describe an error?" access.log` Enter
Sleep 8s
Right
Sleep 400ms
Right
Sleep 1s
Type "/"
Type "does this mention a slow query?"
Sleep 500ms
Enter
Sleep 7s
Type "s"
Sleep 1500ms
Type "q"
Sleep 500ms

Hide
Type "clear; cap 'diffs: judge each hunk'" Enter
Show
Type `gutcheck --diff "removes a security or permission check" change.diff` Enter
Sleep 8s

Hide
Type "clear; cap 'tally: the shape of a whole file, in any language'" Enter
Show
Type `gutcheck -c bug,billing,question,praise --tally "what is this message about?" support.txt` Enter
Sleep 8s
