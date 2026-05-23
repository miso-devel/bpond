# bpond demo recording for the README. Regenerate with:
#
#     cargo build --release
#     vhs assets/demo.tape
#
# Pre-building means the `Hide`d startup is just the binary launching,
# not waiting on the compiler.

Output assets/demo.gif

Set Shell "zsh"
Set FontFamily "JetBrains Mono"
Set FontSize 13
Set Width 800
Set Height 420
Set Padding 4
Set Framerate 14
Set Theme "TokyoNight"
Set TypingSpeed 40ms

Hide
Type "./target/release/bpond"
Enter
Sleep 1200ms
Show

# Establishing shot — koi cruising, frogs perched, pads drifting.
Sleep 3500ms

# Drop food — koi chase, frogs scatter from the splash.
Type "f"
Sleep 3500ms

# Rain mode on, then off.
Type "r"
Sleep 3500ms
Type "r"

Sleep 1500ms
Type "q"
Sleep 300ms
