# Regenera docs/tui.gif (requiere vhs, ttyd y ffmpeg):
#   vhs docs/tui.tape
Output docs/tui.gif

Require tinytop

Set Shell "bash"
Set FontSize 14
Set Width 1100
Set Height 620
Set Padding 10
Set Theme "Catppuccin Mocha"
Set TypingSpeed 40ms

Type "tinytop --tui --config examples/config.basic.toml"
Enter
Sleep 4s
Type "2"
Sleep 2s
Type "3"
Sleep 2s
Type "4"
Sleep 2s
Type "1"
Sleep 2s
Type "q"
Sleep 1s
