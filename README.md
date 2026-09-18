# Deathloop opponent overlay

**Ctrl+Alt+F12 closes the overlay. Ctrl+Alt+F7 shows keyboard controls.**

Right-click the notification-area icon to toggle name, day and network stats.
Settings persist beside the executable. See [controls and network readings](NETWORK_AND_CONTROLS.md).

Native Win32 layered window, GDI text and per-pixel alpha; no universal GUI library.
The window remains topmost, click-through and non-activating.

Run `target/release/deathloop-invader-tool.exe`. It can start before the game and
reattaches after the game exits/restarts. If process access is denied, run at the
same privilege level as the game (Administrator when required).

- Playing as host: displays **Invader: name**.
- Playing as invading Julianna: displays **Host: name | Day 219** (host campaign day).
- An unreadable campaign day is omitted without hiding the opponent name.
- No second loaded player: displays **waiting for opponent**, not a stale name.
- During loading or an incompatible layout: displays **Waiting for game session**.

The name is intentionally withheld until two player IDs are present. This is a
loaded-player check, not a fully decoded online connection-state check.

`deathloop-invader-tool.exe --inspect` prints a one-shot read-only role/name/player
count report without opening the overlay. This prints the other player's display
name when present; no background name logging is enabled.

## Build / tests

`cargo test --offline`

`cargo clippy --offline --all-targets -- -D warnings`

`cargo build --offline --release`

Offsets are specific to the inspected PC build, not universal signatures:

| Field | Location (hex) |
|---|---|
| idGameLocal pointer | Deathloop.exe + 5BD1010 |
| Expected game vtable | Deathloop.exe + 2613250 |
| Host flag | game + 6A730 (1 host, 0 client) |
| Player-ID list pointer / count | game + 210 / 21C |
| Host name buffer | Deathloop.exe + 3335638 |
| Invader name buffer | Deathloop.exe + 3334F68 |

## Review and changes

- Select opponent name from the host/client role; bound strings to the 32-byte
  buffers recorded in the CE table, instead of reading 256 bytes.
- Suppress stale names while no second player is loaded. Re-read session headers
  to catch some loading races; external memory reads are not atomic snapshots.
- Reconnect to restarted game processes and support launching before the game.
- Own process/snapshot handles so failures cannot leak them. Check the actual
  INVALID_HANDLE_VALUE returned by failed Toolhelp snapshots.
- Restrict generic memory reads to sealed primitive types with valid bit patterns;
  the old `T: Copy` API could construct invalid Rust bool/reference/enum values.
- Keep application ownership in the message loop rather than dropping it from a
  callback. Handle GetMessage errors and window/timer creation failures.
- Read names at 4 Hz and repaint only on text changes. Keep the native transparency
  mechanism; use grayscale coverage alpha for smooth text edges. Draw literal `&`
  characters in names instead of interpreting them as menu accelerators.

## Additional information found

The campaign manager is `[game + 34D8]`; its `m_day` is at +80. The inspected
host save reported **219** on 2026-09-18. This is an internal campaign-day counter,
not proven total lifetime loops or opponent skill. The campaign replica builder
(+1116190), serializer (+FD17C0) and receiving application (+11162A0) confirm
transmission and application of this counter. The overlay reads it only while
invading with an opponent present, checks the campaign vtable (+2693B50), and
omits unavailable values. No adjusted (+1) loop number is assumed. Display in a
live invasion was confirmed working by the user.

The game's UI schemas include Julianna current rank and current points. Their
presence is not proof the host receives the remote invader's progression. No
verified remote total-playtime, rank or XP reader is implemented, and none of
these candidate values are presented as opponent statistics.

Tests passed: compilation, strict Clippy, role-selection/label unit tests, and
live read-only attachment/host-role/no-opponent inspection. The network peer
selection fix was verified against an active invasion and confirmed working
by the user.
