# Installed selection and action controls — OCH-41

On 2026-10-05, further public-gallery checks run against the independently
installed application at source `4792413`, on macOS 14.5 arm64 / M1 Max / built-in
display. The executable SHA-256 is
`b978ed554931821f88572e1fe283d2a03123ef636667d669208b369be187a115`.
See [the original consumer build](avatar-native-consumer-och41.md) for its isolated
installation. These runs change only Python test drivers, not the application.

## Passing scopes

- **Selection:** toolbar roles, orientation and identity; checked/mixed values;
  exclusive alignment and independent formatting selections; actual Space,
  Return and Tab; disabled and per-child busy policy; bulk changes preserve
  unavailable selections; connected/separated geometry in both orientations;
  singleton owner retention and restoration; page retirement. The complete
  `selection` run exits zero, with both success markers.
- **Rich buttons:** plain/rich content changes retain owners and focus; per-owner
  loading blocks actions without disabling the shared command on a sibling;
  external `AXElementBusy` values, real Return/Space/pointer input, Tab opt-out
  and preserve-sibling-focus behavior pass. Both button runs reach the success
  marker and proceed to the next scenario.
- **Appearance and composed Link:** the second button run completes all eleven
  variants, hover notification/keyboard clearing, tooltip focus/Escape, native
  Return/Space, compact/large geometry, selected/outline/rounded presentation,
  disabled and loading policies, retained owners and Light/Dark captures. Busy
  Link content expands and restores its width, rejects actions, retains its
  semantic name and recovers. The loading-text screenshot was visually inspected.
  This function completed before the driver entered menu observation; the combined
  run later failed in the separate split-button scenario.
- **Menu observation:** root open/close notifications, submenu navigation/action,
  Escape, placement configurations, observer detach/reattach, disabled action
  rejection, owner retention and page retirement pass in the second run. Its
  success marker precedes the split-button failure. Placement pixel accuracy is
  not established by changing settings and taking screenshots alone.
- **Command hints:** the third isolated run exits zero. Live shortcut
  replacement/removal, actual Command-Shift-H/J invocation and stale-binding
  rejection, disabled action rejection, owner retention, both display-platform
  labels, focus/Escape and page retirement pass. The final Linux-style keycap
  screenshot was visually inspected; this was still a macOS application run.

## Harness corrections

Ordinary toggles expose CFBoolean; mixed checkboxes expose numeric 2. The selection
reader preserves that distinction, and ordinary button-setting readers require
Boolean values. The appearance selector now follows the shell's current-theme
label, as established in [earlier indicator checks](installed-indicators-och41.md).

The first appearance run fails while looking for `Loading guide…` as a separate
AX node. `Link.Config.label` explicitly supplies the semantic name independently
of passive visible content. The original screenshot shows the loading text and
the tree shows `Link appearance action`. The corrected check retains busy/action/
identity assertions and checks changed/restored geometry plus a visible capture;
it does not change the production Link contract.

The command-tooltip driver's original chord query likewise used painted glyphs
instead of `Presentation.Kbd`'s explicit accessible label. Source and the captured
tree agree on `Shift + Command + H`; the correction checks that label independently
of rendered keycaps. This is an AX query, not a screen-reader test.
The second run then fails reopening the tooltip: AXPress on a setting leaves
focus on the action, while Escape suppresses its tooltip until focus leaves.
The final driver focuses the setting and presses Space, then re-enters the action.
That actual keyboard sequence passes without changing production tooltip policy.

## Unresolved split-button identity failure

The second combined run fails `CFEqual` for the primary split action after
switching to action-only mode. A separate instrumented run narrows the *first*
reference change to enabling **Disable split pair**, before any mode change:

- identity remains equal through native menu actions, primary-only disabled
  policy and primary loading/recovery;
- it changes on ancestor pair disabling and remains different after recovery;
- the subsequent mode assertion compares against that earlier reference.

This does not yet distinguish AX-wrapper replacement from replacement of the
native focus/action owner. The assertion remains intact; split acceptance is
**open**, with the original failures and trace preserved. Inspect ancestor
interaction shielding and retained native identity before changing the test or
implementation. No production workaround or acceptance waiver was made.

## Reproduction and scope

```sh
python3 scripts/test_gallery.py --section selection --executable <installed-main.exe> --images <output>
python3 scripts/test_gallery.py --section buttons --executable <installed-main.exe> --images <output>
python3 scripts/test_gallery.py --section split-buttons --executable <installed-main.exe> --images <output>
python3 scripts/test_gallery.py --section command-tooltip --executable <installed-main.exe> --images <output>
```

Individual `button-appearance`, `menu-observation`, `split-buttons` and
`command-tooltip` sections now allow focused iteration without rerunning unrelated
desktop scenarios. `buttons` and `all` still include every constituent scenario.

Exact commands, patches, exit results, diagnostic source, logs and screenshots
are retained in [the archive](installed-actions-och41/local-validation.tar.gz),
with hashes in [the manifest](installed-actions-och41/manifest.json). These results
do not close whole-family resource/performance, standalone-radio/Tab-order,
final-source CI or distribution requirements. Linux desktop remains OCH-47.
VoiceOver settings, automation and tests were untouched and remain on owner hold.
