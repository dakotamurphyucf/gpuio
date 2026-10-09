# Installed spinner, progress and checkable controls — OCH-41

On 2026-10-05 the fresh installed gallery at application source `4792413` passes
three further real macOS walkthroughs. The independently staged public libraries
and static-extension backend are described in [the consumer evidence](avatar-native-consumer-och41.md).
No application code changed during these runs. Test-harness corrections, original
failures, successful logs and representative screenshots are in [the archive](installed-indicators-och41/local-validation.tar.gz).
[The manifest](installed-indicators-och41/manifest.json) hashes every capture and
identifies those retained in the archive; others remain local scratch artifacts.

## Results

- **Custom spinner:** source failure/recovery and built-in/custom SVG switching;
  Light/Dark, 40px geometry and owner retention; static and animated screenshot
  pixels; actual Space activation; cycle/easing updates; page retirement and
  fresh native owner with retained Bonsai configuration. Pass in components-001.
- **Progress:** semantic values, empty/full/tiny/quarter/nearly-complete ring
  pixels in both themes, native indeterminate movement, value-transition policy,
  center-editor keyboard input/geometry/identity, inert removal/restoration and
  page remount. Added actual Cmd-Z/Shift-Cmd-Z verifies native editor history
  survives inert hiding, restoring `Keep typing` and then the typed `a`. Pass in
  components-002. This is not a latency/resource benchmark.
- **Checkable appearance/rich labels:** 24 GPU part-bound cases across themes,
  sizes and label order; reset/owner retention; checked/unchecked/mixed values;
  actual Space/Return and retained focus; radio AX requests; rich-label pointer
  activation with one semantic owner; disabled/stale/inert action fences;
  retirement and state restoration. Pass in controls-003.

These public OCaml/installed-consumer results supplement the independently
passing [native image suite](avatar-native-consumer-och41.md), which exercises
spinner masks/tint, progress clipping/alpha and 21 checkable native size/value
cases. Driver and application processes exit successfully, one window sequence
at a time. Environment: macOS 14.5 arm64, M1 Max, built-in display.

## Harness corrections and retained failures

The shell button names the current appearance; spinner/checkable drivers now
select the intended appearance explicitly. Spinner page departure retires native
sources/objects but preserves the Bonsai timing/easing model. Assertions now
check those actual lifecycle contracts rather than demanding a model reset.

The first progress run fails CFEqual after inert hiding. Inert removes the nodes
from the AX tree, so re-exposure can create new AppKit wrappers. The corrected
check reacquires semantic references, verifies retained text **and native undo
history**, and resumes identity checks for ordinary value/style changes. Page
unmount still requires fresh native objects.

The first checkable run assumes AXValue is always CFNumber; the pinned adapter
exposes ordinary toggles as CFBoolean and mixed state as numeric 2. The reader
now handles both representations without conflating mixed with true. A second
run then reaches the same inert-wrapper identity assumption as progress. The
final check reacquires wrappers after exposure, verifies retained values/mode,
and retains action rejection while disabled/hidden plus subsequent identity and
page-retirement assertions. No production workaround was added for these tests.

## Commands and remaining scope

After the fresh consumer build, each command uses that installed executable:

```sh
python3 scripts/test_gallery.py --section spinners --executable <installed-main.exe> --images <output>
python3 scripts/test_gallery.py --section progress --executable <installed-main.exe> --images <output>
python3 scripts/test_gallery.py --section control-appearance --executable <installed-main.exe> --images <output>
```

Exact arguments and driver patches are archived. Python syntax and structural
catalog checks pass. These are scoped macOS functional results, not clean-machine
packaging, final-source hosted gates, whole-family selection/navigation coverage,
measured frame/resource acceptance or physical monitor migration. Linux desktop
qualification remains OCH-47. VoiceOver was untouched and remains on hold.
