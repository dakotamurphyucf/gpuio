# Avatar preview: stable member keys, scoped sources and passive fallbacks

[avatar_preview.ml](avatar_preview.ml) and its [interface](avatar_preview.mli)
compose five team members with `Avatar_group`. The Presentation branch in
[pages.ml](pages.ml) supplies app/window/palette/graph. Members are passive avatars;
only the optional overflow button performs an action, incrementing a mock Team opens
counter. There is no team fetch or popup here.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Presentation**, change avatar source/limit/size/overlap, reverse order,
try custom fallback and actionable overflow. No external photos/files are required;
there is no page-specific diagnostic or self-test. [Development](../../docs/development.md)
covers toolchain setup, and this review adds no native image, assistive or platform acceptance.

Resource acquisition registers [gradient PNM](image_samples.md), invalid PNM bytes
and the example's own `person_svg` sequentially in one `Preview_scope`. Ready exposes
three borrowed handles; encoded publication does not imply native decode success.
Failure/departure retires the visit's registrations and late results are suppressed;
returning reacquires them. This is the component's Eio/native resource boundary,
not a per-member fetch. See [preview_scope.md](preview_scope.md).

`B.state_machine0` stores limit=3 (cycle 0–5), size index=2 (Medium), overlap index=1
(30%), source index=0 (initials) and actions=0. `B.toggle` owns reverse=false,
overflow=true, actionable=false, identity-colors=true, custom-fallback=false and
square=false. `B.state` owns the latest Ada image observation. `let%arr` derives items
from current values; native callbacks execute setter/injection effects. Size choices
are named 16/24/48/80 pixels or custom 56; overlap choices are 0/30/60%.

Member tuples use stable keys ada/grace/yuki/sam/morgan, explicit initials including
京都, and meaningful descriptions. Only Ada optionally receives the valid/invalid
image. `Avatar.Config` falls back to initials for absent/loading/failed data.
`Avatar.Palette.for_key` gives deterministic identity colors when no asset is supplied;
reordering does not recompute colors from positions. Otherwise this demo uses ordinary
palette styles. Square refinement sets radius 8 while keeping group sizing.

With custom fallback on, `A.Item.create_with_fallback` supplies a decorative native
person icon at half avatar size; its passive checked slot retains the avatar's one
meaningful description. The source image still takes precedence when ready.
`A.create` validates all keys before applying limit and mounts only the visible prefix.
Surviving reordered keys/size changes retain identity; omitted members' native image
leases/observations retire, while encoded registrations remain scope-owned.

For a concrete trace, select Image source with initial limit three. Ada binds the
valid handle; native Loading/Ready callbacks set image_state, deriving the status.
Select Invalid image and a native Failed observation shows fallback while retaining
Ada's semantics. Reverse order at limit three now omits Ada; the status may retain
its last observation because hidden members send no callbacks. It is not a current
paint certificate. Raise the limit to remount Ada under its same logical key.

Overflow receives the exact omitted count/size during OCaml view construction,
returning either `A.ellipsis` with meaningful description or an ordinary button.
Click the actionable overflow: open_team's effect increments the readout, without
making passive avatar leaves clickable. See
[avatar_group.mli](../../lib/core/avatar_group.mli) and
[avatar.mli](../../lib/core/avatar.mli) for key/lifetime/fallback/color contracts.
To add a person, provide a unique stable key, explicit fallback and description;
keep registered assets scope-owned instead of giving an omitted member ownership
of the whole source registration. State choices persist in the branch graph; there
is no file/network task or OCaml animation loop to cancel.
