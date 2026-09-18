# Deathloop opponent overlay

**Ctrl+Alt+F12 closes the overlay. Ctrl+Alt+F7 shows keyboard controls.**

Right-click the notification-area icon to toggle name, day and network stats.
Settings persist beside the executable. See [controls and network readings](NETWORK_AND_CONTROLS.md).

Native Win32 layered window, GDI text and per-pixel alpha; no universal GUI library.
The window remains topmost, click-through and non-activating.

Run `deathloop-invader-tool.exe`. It can start before the game and
reattaches after the game exits/restarts. If process access is denied, run at the
same privilege level as the game (Administrator when required).

- Playing as host: **Invader: name**.
- Playing as invading Julianna: **Host: name | Day 219** (host campaign day).
- No second loaded player: **waiting for opponent**.
- During loading: **Waiting for game session**.



`deathloop-invader-tool.exe --inspect` prints a one-shot read-only role/name/player
count report without opening the overlay. This prints the other player's display
name when present.
