Output demo/demo.gif
Set Shell "bash"
Set FontSize 18
Set Width 1200
Set Height 640
Set Padding 16
Set Theme "Dracula"
Set TypingSpeed 40ms
Hide
Type "export PATH=$PWD/target/release:$PATH PS1='$ ' && cd examples && clear" Enter
Show
Type "# ask a question about every line, locally, on CPU" Enter
Sleep 800ms
Type `gutcheck "is this a bug report?" < issues.txt` Enter
Sleep 15s
Type "clear" Enter
Type `gutcheck -s "could this commit break prod or weaken security?" < commits.txt | sort -rn | head -3` Enter
Sleep 15s
Type "clear" Enter
Type `gutcheck -c bug,billing,question,praise "what is this message about?" < support.txt` Enter
Sleep 10s
