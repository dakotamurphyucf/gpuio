# SVG icon transformations

OCH-41, 2026-10-07, after `90e7156`, macOS 14.5 arm64 on Apple M1 Max.
The [contract](../design/icon-transforms.md) now has public OCaml and native
implementations. This is scoped feature evidence; OCH-41 and OCH-17 remain open.

`Icon.Transform.create` and `rotate_degrees` provide validated scale, clockwise
rotation and logical-pixel translation. Configurations and decorative slots accept
the same value; replacement/reset leaves source, node and handler identity intact.
The additive tag-128 operation requires matching OCaml/native packages. No GPUI,
Bonsai or compiler pin changes are needed.

Ordinary icons use decoded alpha with native foreground and the existing GPU
affine mask path. Changing color or transform reuses the decoded image/request.
Fit, measured size, density and rounded clipping belong to bounded raster work;
previous completed pixels can remain visible while a replacement is prepared.
Transforming artwork preserves layout, focus, input and accessibility ownership.
AppKit menus take bounded transformed template snapshots in their existing
16-logical-pixel slot; they do not continuously repaint while tracking.

## Local validation

- OCaml expect tests compare independently composed transform/reset bytes,
  validate numeric bounds, and check transform-only reconciliation, silent
  refreshes, source preservation and decorative forwarding. Rust protocol tests
  cover paired bytes, malformed/truncated values and bounds; native admission
  rejects wrong-kind/invalid updates atomically. The full protocol suite passes
  **468 tests** at the foundation checkpoint.
- The final native unit suite passes **1,069 tests / 2 ignored**. It includes
  worker corner coverage/alpha/accounting and AppKit bitmap snapshot checks.
  Strict native Clippy passes with all three native test features and all targets.
- `native_image_views` passes actual Metal readback for asymmetric icons:
  identity, rotation, translation, reflection, nonuniform combined transforms,
  zero scale, reset, source/foreground/element alpha, rounded and parent clipping,
  unchanged layout and decoded-image/request identity, plus retired-source
  resampling and cleanup. All five fit policies pass in a nonsquare viewport.
  Tests use synthetic densities **1, 1.5 and 2**; these are not physical monitor
  migration or presentation-timestamp measurements. The existing image, button,
  avatar, spinner, progress and control-appearance scenarios also pass.
- `native_menu_popup_queue` passes actual AppKit row/submenu image construction,
  independent bitmap pixels at **1×/2×**, and Objective-C action routing. The
  existing queued tracking, source retirement and open-popup cancellation suite
  also passes. Separate mounted-host tests verify transform metadata reaches the
  snapshot while preserving the retired source lease. These checks do not claim
  VoiceOver or transformed-menu keyboard-navigation acceptance.
- The public gallery's Assets walkthrough passes seven presets/reset, native
  icon/button identity and size retention, actual keyboard activation for every
  preset, and physical pointer activation with collapsed artwork. Three page
  departures/reentries retain approval count 10 and the explicitly restored
  default icon, followed
  by zero registered resources/source bytes and normal application shutdown.

The new [preset helper walkthrough](../../examples/gallery/icon_transform_sample.md)
and expanded [Assets walkthrough](../../examples/gallery/assets_page.md) explain
the actual pure helper, Bonsai graph, effects, public API calls and scope ownership.
An owner-authorized GPT-6.1 Sol documentation agent contributed both guides;
the parent reviewed them against the implementation. Documentation inventory:
**429 sources / 266 reviewed groups / 0 pending**. Structural coverage does not
establish explanatory quality or native acceptance on its own.

Initial transform-fixture failures were compilation mistakes and an atlas query
that only TestPlatform implements; Metal's default membership query returns false.
The corrected fixture uses actual GPU readback and decoded-image identity.
Production rendering did not change to satisfy that false atlas assertion.
The initial AppKit test used the wrong generated Objective-C method signature;
the corrected test invokes the declared selector through Objective-C dispatch.

Full Dune `@all @runtest @fmt` passes. A fresh package prefix, copied gallery
sources and independent composed backend build and pass both catalog handshakes.
The installed executable passes the same native walkthrough, including all seven
presets, keyboard/pointer activation and cleanup. Source hashes match; this uses
the existing machine/toolchain and is not clean-machine or signed-distribution
acceptance.

The first installed walkthrough lost its window after the Offset preset; its
original run did not enable close tracing, so the reason is unknown. The same
installed binary passed a traced repeat, with only the expected final close
request. A final repository run separately detected Wispr Flow covering the
pointer target and stopped before clicking. Moving only the test window to the
upper screen resolved that obstruction. The first positioning helper ran before
window readiness and trapped; the corrected helper selects the page and requires
a non-null window before moving it. The final repository binary then passed the
entire traced walkthrough. No production change was made for these interruptions.

[Archived logs and qualified patch](icon-transforms-och41-logs.tar.gz) and the
[verified manifest](icon-transforms-och41-manifest.json) preserve exact commands,
exit codes, platform, source/binary hashes, initial failures and reviewed captures.
Current-source hosted/Linux validation, real Linux GUI qualification under OCH-47,
and the broader macOS accessibility/performance/distribution release requirements
remain separate. No VoiceOver configuration was changed by these checks.
