# vhs demo/demo.tape   (needs target/release/gutcheck and the model cached: run gutcheck once first)
Output demo/demo.gif
Set Shell "bash"
Set FontSize 19
Set Width 1200
Set Height 700
Set Padding 24
Set Margin 24
Set MarginFill "#282a36"
Set BorderRadius 12
Set WindowBar Colorful
Set Theme "Dracula"
Set TypingSpeed 28ms

Hide
Type "export PATH=$PWD/target/release:$PATH && cd examples && source ../demo/prompt.sh" Enter
Type "clear; banner" Enter
Show
Sleep 2s

Hide
Type "clear; cap 'filter: keep the lines that match your question'" Enter
Show
Type `gutcheck "is this a bug report?" < issues.txt` Enter
Sleep 7.5s

Hide
Type "clear; cap 'score: P(yes) for every line, red = very likely'" Enter
Show
Type `gutcheck -s "does this log line describe an error?" < app.log` Enter
Sleep 8s

Hide
Type "clear; cap 'classify: your own labels, any language'" Enter
Show
Type `gutcheck -c bug,billing,question,praise "what is this message about?" < support.txt` Enter
Sleep 8s
