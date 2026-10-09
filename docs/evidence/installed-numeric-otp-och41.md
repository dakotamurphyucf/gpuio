# Installed numeric frames and OTP — OCH-41

On 2026-10-05, physical gallery walkthroughs on macOS 14.5 arm64 / M1 Max /
built-in display exercise the public numeric-frame, application-step and segmented
OTP APIs. The initial consumer is the installed `e210d3e` application from
[the lifetime checkpoint](gallery-lifetime-repairs-och41.md), executable SHA-256
`5e96404a5df658b7dfa9af05b8d4ff8240737e8df0f751f05c58968a0e24d495`.

## Numeric rendering defect and repair

The first numeric run passes its input, geometry and history assertions, but
visual inspection finds **duplicate content**: Qty, units and the custom arrows
appear both in their intended slots and again after the numeric control. That
run is not numeric presentation acceptance despite its exit code and success
marker. The original log and capture are retained.

`node_content` already builds the numeric presentation using its four structural
slots. The generic child loop in `host.rs` then incorrectly appends the same
children again. Commit `b891868ad1bf1da20801ade6e4748bcf0392002c` excludes
`Kind::NumberInput` from that second traversal, as other component-owned slot
layouts already are excluded. No public API, wire format or dependency changes.

The existing mounted native numeric-frame test now assigns four distinct paint
colors to the slots and counts their actual quads. Before the fix it fails:
slot 0 paints twice, expected once. After the fix it checks one leading/trailing
paint in Side, Stacked and Hidden modes, one of each custom step slot in Side and
Stacked, and zero custom step slots when Hidden. Existing editing, composition,
history, focus, repeat and policy assertions remain intact. An initial test-draft
compile error needed an explicit u32-to-wire-i64 color conversion; it was separate
from the subsequently reproduced behavior failure.

The full native library passes **930 tests with two existing ignores** using
`--features native-image-tests,native-canvas-tests --lib`. Strict native lib/tests
Clippy with the same features and `-D warnings`, Rust formatting and Dune `@fmt`
pass. Commands run through `GPUIO_JOBS=2 ./scripts/gpuio exec`.

The public driver is strengthened to count exactly one Qty action when framed,
zero when plain, and no separately exposed passive custom-arrow text. A successful
geometry/history check alone must not hide duplicated paint again.

A fresh staged installed consumer at `b891868` builds and passes both catalog
checks. Its executable SHA-256 is
`0d8c9451c48088ab72653cb6b64d6155217cf9c5c911009a81b0b406dbfb925c`.
The strengthened numeric walkthrough exits 0 with all 24 cases and both success
markers; the OTP walkthrough on the same new executable also exits 0 with all 16
cases. The corrected Light framed/custom/stacked capture was visually inspected:
there is one Qty prefix, one units suffix and one pair of step symbols. Both
applications close and are reaped normally. This is a fresh local installed
consumer, not clean-machine distribution acceptance.

## Desktop walkthrough scope

Numeric checks cover 24 Light/Dark × framed/plain × default/custom symbols ×
Side/Stacked/Hidden cases with retained native editor identity, draft and committed
value. A real typed replacement retains undo/redo across all updates. An unfinished
`1e-` draft is rejected without destruction, Restore recovers the committed value,
and `19.13` commits to the quarter-step value `19.25`. The public command buttons
perform commit, restore, undo and redo. The independent Qty prefix does not alter
the draft; it remains available when read-only and becomes disabled with its
numeric owner. Required/optional-empty policies and page retirement/remount are
checked separately.

Application-selected steps run through the public OCaml/Eio request path: keyboard
Up checks 9.75→10, 10→11, 49→50 and 50→55; AX decrement and a native step-button
action also resolve through that path. Restoring native mode produces the normal
quarter step. These are end-to-end request checks, not simulated callback results.

OTP checks cover 16 Light/Dark × grouped/ungrouped × small/large × masked/unmasked
retention cases. Physical digit typing, Left/Backspace, completion/incompletion
and native undo/redo pass across presentation changes. AX replacement normalizes
full-width digits and atomically rejects invalid characters. Masked values expose
secure-field semantics with no code disclosure; unmasking retains the value.
Read-only preserves focus and rejects deletion. Remount creates a fresh editor
with the mount seed, and real typing works on that new owner. The Dark grouped,
large-cell capture was visually inspected.

## Reproduction and limits

```sh
python3 scripts/test_gallery.py --section numbers --executable <installed-main.exe> --images <output>
python3 scripts/test_gallery.py --section otp --executable <installed-main.exe> --images <output>
```

Both selectors are included in `all`; focused runs do not establish the full
gallery. Appearance updates use ordinary AX actions, while editing checks use
actual native keys. Captures and geometry are not exact per-cell pixel measurements.
The [archive](installed-numeric-otp-och41/local-validation.tar.gz) and
[manifest](installed-numeric-otp-och41/manifest.json) retain exact commands and
exit results, before/after native results, original visual-failure evidence,
fresh-consumer logs, final driver sources, captures and sample data. Python
compilation, catalog audit and whitespace checks pass.
Physical OTP blink timing, reduced-motion transitions, hold-repeat timing,
IME/clipboard/privacy beyond these checked paths, measured resources/performance,
final-source hosted CI and distribution remain separately qualified work.
VoiceOver remains on owner hold and was untouched. Linux desktop remains OCH-47.
