Output marketing/demo.gif
Set Shell "bash"
Set FontSize 18
Set Width 1400
Set Height 900
Set Padding 20
Set TypingSpeed 35ms

Type "# LEFT: npm-run-all  |  RIGHT: run-all-now"
Enter
Sleep 500ms
Type "tmux new-session -d -s run-all-now-demo 'bash bench/startup.sh'"
Enter
Type "tmux split-window -t run-all-now-demo -h 'bash bench/memory.sh'"
Enter
Type "tmux attach-session -t run-all-now-demo"
Enter
Sleep 8s
