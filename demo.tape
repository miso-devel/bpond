# VHS tape file for recording bpond demo GIF
# Usage: vhs demo.tape

Output assets/demo.gif

Set Shell zsh
Set FontSize 14
Set Width 1200
Set Height 600
Set Padding 20
Set Theme "Catppuccin Mocha"
Set Framerate 60

# Build silently
Hide
Type "cargo build --release 2>/dev/null"
Enter
Sleep 3s
Show

# Launch
Type "cargo run --release"
Sleep 500ms
Enter
Sleep 3s

# Feed a few times so koi chase food
Type "f"
Sleep 2500ms
Type "f"
Sleep 2500ms
Type "f"
Sleep 2500ms
Type "f"
Sleep 3500ms

# Quit
Type "q"
Sleep 500ms
