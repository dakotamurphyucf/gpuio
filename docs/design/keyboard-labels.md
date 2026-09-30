# Typed keyboard labels

`Shortcut.format`, `Shortcut.accessible_label` and `Presentation.Kbd` supply the
pure display portion of the pinned [Kbd source](../catalog/sources/component-kbd.rs.txt)
(gpui-kit `84f57fdfcb4910623fb0bb7f795b077e249f9271`). `Command.shortcuts` exposes
ordered declarations for display. The legacy string-list `shortcut_label` remains
unchanged. This is **partial source coverage**: effective native action/context/
focus binding lookup remains required OCH-41 work.

```ocaml
let shortcut =
  Gpuio.Shortcut.create ~key:"k" ~modifiers:[ Primary ] ()
  |> Core.Or_error.ok_exn
in
Gpuio.Presentation.Kbd.create appearance ~platform:Macos shortcut
```

## Formatting and platform semantics

A shortcut is still a validated single chord; display adds neither multi-stroke
input nor a binding registration. Its routing priority, text-input policy and
composition flag do not affect display. Platform is explicit (`Macos | Linux`),
so pure Core code does not inspect the environment or introduce I/O. Applications
choose the label platform; selecting Linux labels on macOS does not change the
native command router's operating system.

`Primary` resolves to Command on macOS and Control on Linux. `Super` resolves to
Command on macOS and Super on Linux. Duplicate physical modifiers collapse after
resolution: `[Primary; Super]` displays one Command on macOS, while
`[Primary; Control]` displays one Control on Linux. The existing constructor still
rejects duplicate *logical* modifiers. Display order is Control, Alt, Shift,
Command/Super, matching native modifier resolution.

| Input | macOS display | Linux display |
| --- | --- | --- |
| Primary + K | ⌘K | Ctrl+K |
| Control + Alt + Shift + Super + A | ⌃⌥⇧⌘A | Ctrl+Alt+Shift+Super+A |
| Shift + Backspace | ⇧⌫ | Shift+Backspace |
| Shift + Delete | ⇧⌦ | Shift+Delete |
| Primary + Enter | ⌘⏎ | Ctrl+Enter |
| Primary + Left | ⌘← | Ctrl+Left |

Named keys have readable labels; macOS uses symbols for Enter, Escape, deletion
and arrows. Function keys retain their full F1..F24 number. Scalar keys use the
existing pinned Uucp full uppercase mapping, including expansions (`ß` → `SS`,
`ﬃ` → `FFI`); the shortcut's actual key is unchanged. A plus key displays as `+`,
so `Ctrl++` is an intentional display string, not a parsing format.

Two intentional source differences are explicit: Linux uses **Super**, not the
source's Windows-oriented **Win**, and macOS forward Delete uses **⌦**, distinct
from Backspace **⌫**. Bare modifier names are not shortcut keys in GPUIO's existing
validated chord domain. No parser or protocol change is introduced.

`accessible_label` supplies an English spoken-name form, e.g.
`Control + Option + Shift + Command + Left arrow`, independent of visual symbols.
Applications may override the keycap's accessible name with localized nonempty
UTF-8 without NUL, up to 4096 bytes; invalid names return `Or_error`.

## Keycap appearance and ownership

`Presentation.Kbd.create` returns a normal text view with Label semantics and its
spoken accessible name. It adds no focus stop, callback, command scope, editor,
resource, clock or task. Stable caller keys preserve the same native text object
through platform, variant, theme and style changes. Slot removal/page departure
uses ordinary native retirement. Application state and I/O remain caller-owned.

- `Filled` (default): Appearance muted text and raised fill, 4px radius, 4px/2px
  horizontal/vertical padding, 20px minimum width, 12px type, 100% line height,
  centered normal whitespace and no shrink.
- `Outline`: the same metrics plus a one-pixel Appearance border and surface fill.
- `Plain`: inherited typography and no keycap defaults.

Caller styles refine defaults in every variant, including Plain. This deliberate
improvement permits styled plain text; the pinned source returns raw text before
applying its refinement. No claim is made that arbitrary placement overrides
preserve the default keycap geometry.

`Command.shortcuts` preserves declaration order, including on disabled commands.
It does **not** report effective focus, command shadowing, competing shortcuts,
native editor availability or IME composition eligibility. Formatting a declaration
must not be presented as proof that pressing the chord currently invokes it.

## Local evidence

Four Core expect tests pass:

- macOS/Linux modifier aliases and order; punctuation, Unicode expansions and
  named/function key goldens;
- spoken names, routing-policy independence and ordered disabled-command
  declarations;
- three variant defaults, style refinements and invalid/localized names;
- six platform/variant updates preserving one inert text node, idle equal
  snapshots and native-node retirement.

Full Dune `@all @runtest @fmt` passes locally on macOS 14.5 arm64 (2026-09-30).
The public **Keys with meaning** card changes platform, chord, styles, order and
slot visibility around a retained native draft. Its focused native run passes
13 layout/name/identity cases and four GPU theme/refinement cases, including
filled/outline surfaces, borders and plain inherited typography. Actual Command-K
produces no invocation without registration, invokes while enabled, stays silent
while disabled and resumes when enabled. Selecting Linux display leaves native
macOS routing unchanged; removing registration stops further invocation.
Slot/page retirement and application-owned counter retention pass.

The early harness failures were an incorrect `node_values` tuple unpack and an
incorrect dark-surface reference (using `on_solid` instead of `surface`). Both were
corrected; the native implementation did not need a workaround. Native tests use
480-second process-group watchdogs and reap their children. A fresh installed-library consumer also passes the same 13 geometry/name/identity
and four GPU cases, explicit registration/routing checks and slot/page teardown.
The consumer stages libraries in its own prefix and builds an independent locked
native backend without changing opam switches. The focused section is included
in `core` and `all`; the expanded combined run and CI/release acceptance remain
separate.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
python3 scripts/test_gallery.py --section keyboard-labels
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-m7-keyboard-consumer-20260930
python3 scripts/test_gallery.py --section keyboard-labels --executable /private/tmp/gpuio-m7-keyboard-consumer-20260930/consumer/_build/default/main.exe
```

No VoiceOver, real IME, Linux GUI, application performance or clean-machine
qualification is established by these focused checks. Required Linux non-GUI
checks remain; desktop qualification is deferred OCH-47.

## Remaining binding-query contract

The source also queries the highest-precedence action binding for a context or
focus handle and displays its first stroke. GPUIO has no equivalent effective
binding observation yet. Do not promote the entire Kbd source row to a functional
equivalent until this gap is resolved and validated.

The implementation should share native resolution policy with actual command
routing. A pure scan of OCaml registry declarations cannot account for current
focus, nearest-scope shadowing, native-first handling, priority, composition,
modal focus boundaries or native editor availability. A query/result contract
must define exactly which of these it answers rather than promising that any
reported binding will consume every OS key event.

Keep native resolution on the GPUI thread. Any result crossing into OCaml must be
asynchronous, associated with the originating window and mounted query owner,
and fenced by current focus/registry revision. Specify missing, disabled and stale
results; retire observations on unmount/window close. Bound query counts, updates
and retained results. Use the existing single-chord domain rather than silently
adding sequences. Test nested/disabled shadowing, conflicts, focus transitions,
IME gates, stale revisions, window isolation and actual invocation. Until that
work lands, callers can display explicit declarations only.

The [native observation design and shared resolver evidence](command-binding-observations.md)
now specifies these requirements in more detail, including different same-phase
and cross-phase conflict rules and the separate native widget keymap source.
Actual routing uses the shared iterator/input policy and its native regressions
pass. The public observation API and its lifecycle integration remain unfinished.
