# Date-picker preset evidence — OCH-41

2026-10-02, local macOS dirty worktree based on `83eb87e`. This checkpoint adds
public date presets and documents remaining calendar/color catalog gaps. It does
not complete OCH-41 or the milestone release requirements.

Two new Core expect tests cover empty/single/range choices, rejection of partial
ranges, UTF-8/label and unique-ID bounds, the 32-item collection limit, ordering,
current mode/read-only/disabled policy and both range-constraint policies.

Seven real Bonsai/Window_driver tests in the private date-picker component cover:

- Revision-guarded replacement changes only the draft; Apply reads again before
  calling the application callback.
- Concurrent preset/Apply requests return Busy without additional native work.
- A confirmation started before a preset cannot commit while replacement is pending.
- Old actions/replies cannot affect a reopened picker or release its busy gate.
- Captured actions recheck current read-only, disabled, constraints, controlled
  value and Bonsai activation before dispatch.
- Later native revisions reject stale results; native errors release the busy
  gate. Read-only set after dispatch retains the native draft but prevents Apply.
- A rendered preset button with a maximum-length stable ID remains disabled until
  the first calendar observation, then routes its Press into the guarded flow.

Native commands are injected, deliberately delayed test completions. This tests
the real controller/reconciler, not the actual Eio/native transport or OS input.
The public wrapper still supplies the existing App.Window calendar command.
There is no new Rust code, wire format, native owner or synchronous FFI callback.

The initial implementation used Calendar.Selection.is_complete for construction;
tests caught that it excludes Empty. The final preset policy explicitly allows
Empty while rejecting Range_start. A button harness initially expected a handler
on its disabled Create; it now follows the actual Bind after the mount observation.
No failing expectation or backtrace was accepted as successful behavior.

Passing commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest @fmt examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @lib/eio/runtest @fmt
python3 scripts/audit_component_catalog.py
git diff --check
```

The full suite/gallery command passed with the two Core and first six controller
tests. The final focused suite/format command passed after adding the seventh
button test; production code was unchanged after the full-suite pass. Standard
linker warnings about duplicate system libraries remain. The catalog audit
verifies snapshot hashes and structural ownership, not implementation parity.

The public gallery offers Demo day, One week later and Clear date presets and
explains draft/Apply behavior. No OS window was opened in this slice. Physical
macOS input/accessibility/VoiceOver/rendering, measured resources, Linux automated
checks at the final release revision and consumer/distribution acceptance remain
separate gates. The [source review](../catalog/calendar-color-review.md) records
multi-month, internal appearance and color presentation gaps explicitly; the
[preset contract](../design/date-picker-presets.md) states the shipped semantics.
