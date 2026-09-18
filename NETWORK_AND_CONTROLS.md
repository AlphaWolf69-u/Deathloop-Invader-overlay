# Display controls and network readings

## Keyboard controls (work without clicking the overlay)

Hold **Ctrl+Alt**, then press:

| Key | Action |
| --- | --- |
| F1 | Toggle name |
| F2 | Toggle day |
| F3 | Toggle network stats |
| F6 | Hide/show overlay, keeping your chosen fields |
| F7 | Toggle on-screen shortcut guide |
| F12 | Close overlay |

These shortcuts work globally while the overlay runs.
Keys are observed, not blocked from the foreground app.
Name/day/network choices persist; hide/help state is temporary.
A brief startup hint shows the help and close shortcuts. Run at the same
privilege level as the game when required. The display stays click-through.

The tray menu below remains an optional alternative.

Right-click this overlay's icon in the Windows notification area (use the ^ arrow
if Windows hides it). Check/uncheck **Opponent name**, **Host day**, or **Network
statistics** independently.
**Hide all** leaves the control icon available; **Show all** restores every group.
**Exit overlay** closes only that overlay.

Settings are saved to a same-named `.ini` beside the executable. The display is still
transparent and click-through; control it through the tray, not the text itself.

## Networking

- Ping: game's peer RTT in milliseconds. Missing/stale measurements show N/A.
- RTT variation: mean absolute difference between newly observed RTT measurements,
  up to 20 differences. This is sampled RTT variation, not one-way packet jitter.
- RX loss: game's recent incoming sequence-gap loss estimate. Reordering can
  affect this estimate. N/A until the game has a nonempty loss window.
- RX/TX KiB/s: differences in this peer's game-packet byte counters, sampled over
  approximately one second. Not total adapter/VPN traffic or Internet capacity.
- TX packets/s: outgoing game-packet counter rate, not rendered FPS.
- RX idle: time since the overlay last observed incoming byte-counter activity.
  Sampled every 250 ms; not individual packet inter-arrival timing or snapshot age.

No active game peer -> a waiting message. Multiple eligible peers -> unavailable,
rather than arbitrarily mixing connections. Histories reset when the peer/process
changes or reads fail. No packet capture driver, debug hooks or game writes.

Remote FPS, receive packets/s, exact snapshot gaps, geographic location and
reconciliation magnitude are not implemented. There is no verified counter for
these in this build yet; do not infer them from the readings above.

## Code evidence (this executable build)

Game lobby at exe+3333088, peer array +25C8/count +25D4, stride 3E0.
When invading, use the indexed host peer (index at exe+3333168), requiring
connection state +4==2. Its gameplay state +8 can remain 3 throughout a live
match. When hosting, retain +4==2 and +8>=6 for selecting the invader.
This follows the separate indexed and enumeration paths in +C67F90.
Packet processor pointer is peer+278. No addresses or user identifiers are sent
to external services.

- +C50E9D..+C50F8E handles OOB_PING_REPLY. Peer +2E0/+2E8 are send/receive
  timestamps; RTT is stored at +2F4, then converted to milliseconds at peer+328.
  +C556CD/+C68071 use +328 as the fallback ping for bad-connection checks.
- +C37092 calls +C36710 with outgoing packet byte length. It adds bytes to
  processor+1017F8. Outgoing packet count increments at +C37068 (+10181C).
- +C3716C calls +C36780 with incoming byte length, accumulating +1017FC.
- +C371DA..+C37214 detects sequence gaps and feeds +C37800 with missed count.
  That function calculates lost/window-count at processor+101870. Window count
  is +101868. The same loss field feeds the bad-connection check +C6809D.

Compilation, unit tests and strict Clippy passed. The connected-host selection
was checked against an active invasion, and the corrected overlay was confirmed
working in multiplayer. Counters remain reverse-engineered and build-specific.
