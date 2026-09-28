# Platform release policy

Owner decision, 2026-09-28: continue development and ship milestone 07 as a
macOS-first v1 release. Linux remains an intended platform, with full desktop
qualification deferred until a suitable remote environment is available. This
supersedes earlier simultaneous macOS/Linux release requirements and references
assigning full Linux acceptance to OCH-17. Historical test results remain valid
only for the environments and behavior they actually exercised.

| Work | Release gate / owner |
| --- | --- |
| Complete public component gallery and pinned coverage ledger | Milestone 07, OCH-41 |
| macOS native behavior, accessibility, performance/resource budgets, documentation and clean-machine distribution | Milestone 07, OCH-17 |
| Linux compilation, OCaml/Rust unit tests, private-D-Bus checks and independent consumer builds | Required engineering checks throughout development |
| X11/Wayland software-rendered graphical CI smoke | Informational, with actual results and failures preserved |
| Real Linux desktop input/IME/clipboard/accessibility, portals, notifications, compositor/GPU behavior and clean-machine packaged-app qualification | Deferred milestone 07b, OCH-47 |

[OCH-47](https://linear.app/ochat/issue/OCH-47/qualify-linux-x11wayland-desktop-behavior-and-distribution-after-macos)
does not block OCH-17, OCH-41, milestone 08 or ongoing feature development. No
local VM, cloud machine, spending or deadline is required now. Existing advanced
component and optional-platform milestones retain their scope. This deferral
does not reduce catalog completeness or move macOS performance work out of M7.
In particular, the unverified local chart exact-series 63,050.99ms outlier in M6
evidence still needs the M7 performance audit.

Until qualification passes, describe Linux as experimental / build-tested and
list specific graphical evidence and limitations. Compilation, screenshots and
private-bus fixtures do not establish desktop integration. Record X11 and
Wayland separately; a remote X11 session does not validate Wayland. Hardware
performance claims require named hardware and driver evidence; software Vulkan
and render-callback measurements do not establish physical presentation timing.

The final [M6 CI run](https://github.com/dakotamurphyucf/gpuio/actions/runs/36432631460)
passes required macOS/Linux gates. Informational X11 smoke passes; Wayland still
fails the clipboard assertion in `rust/native/src/control_test.rs` (empty text
versus `De`). Both backends pass desktop-link invocation checks, but real portal,
notification daemon, IME, screen-reader and GPU qualification is still pending.
OCH-47 preserves the reproduction context and acceptance checklist. Old evidence
artifacts may name OCH-17 as the Linux follow-up; OCH-47 now owns that work.
