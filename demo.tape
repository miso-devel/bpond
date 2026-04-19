# VHS tape file for recording bpond demo GIF
# Usage: vhs demo.tape

Output assets/demo.gif

Set Shell zsh
Set FontSize 14
Set Width 1200
Set Height 600
Set Padding 20
Set Theme "Catppuccin Mocha"

# Build and run
Hide
Type "cargo build --release 2>/dev/null"
Enter
Sleep 3s
Show

Type "cargo run --release"
Sleep 500ms
Enter
Sleep 12s

# Quit
Type "q"
Sleep 500ms
