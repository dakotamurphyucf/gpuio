# Attachment preview: mock lifecycle, independent actions and scoped media

[attachment_preview.ml](attachment_preview.ml) is called by the Presentation branch
in [pages.ml](pages.ml); it has no separate interface. It demonstrates
`Presentation.Attachment` slots, status styling, whole-card activation and image
failure. Upload/Open/Save labels are mock behavior: statuses are button-selected
values and actions increment counters, with no upload, file open or save operation.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Presentation**, change attachment status/size/layout, enable image and
simulate decode failure, activate the card or independent Save action, then add
and constrain attachments. No external file or service is needed and no dedicated
self-test/diagnostic flag exists. [Development](../../docs/development.md) covers
prerequisites; no new native action, decode, keyboard or platform acceptance is claimed.

Read `named`, resource acquisition, reactive choices, slot construction, `card`,
`additional` and `A.group`. `Preview_scope.acquire` registers the shared
[PNM gradient](image_samples.md) and malformed PNM bytes sequentially in one visit
scope. The Ready pair contains borrowed handles, not decoded images. Registration
can succeed for malformed bytes; native image decode reports failure later. Partial
acquisition failure or page departure cancels the scope and retires both assets;
returning reacquires them. Read the [scope guide](preview_scope.md) and
[asset contract](../../lib/eio/asset.mli).

`B.state_machine0` cycles status (initial Complete) through Pending/Uploading/
Processing/Failed/Complete and size (initial Medium) through XS/S/M/L/custom 56.
It also stores opened/saved counters. `B.toggle` owns axis, image/broken, slots,
trigger-disabled, refinement and group constraints. Media/content/actions start on;
image and other options start off. `B.state` stores the latest native image observation
(initial Loading). `let%arr` reads these current values to derive slots; setters
and injections are effects scheduled by native callbacks.

`A.Content` combines keyed title and description; the displayed PNG/2.4 MB metadata
is fictional, while the actual gradient is small PNM bytes. `A.Media` optionally
binds an image handle and keeps a PNG text overlay. `A.Trigger` is the named whole-card
native button, separate from `A.Actions` Save and permanently unavailable buttons.
Disabling the attachment disables its trigger; Save remains independently enabled.
Action-cluster shielding prevents clicks/gaps/disabled actions from arming the card
trigger. Status drives presentation (including title shimmer in work states), not
a real producer. Native motion policy owns shimmer; Bonsai starts no frame timer.

Enable Attachment image, then simulate failure: effects change config to the invalid
handle, native decode sends Image.State.Failed to `set_image_state`, and the readout
updates through Bonsai. Status Failed and image decode Failed are independent states.
The image-state value is the last observation; removing Media does not turn it into
proof of a currently mounted image. Click Save: only saved increments. Click the
card: only opened increments. The stable root/slot keys preserve surviving native
controls across ordinary status/layout changes; removing a slot retires its views.

`A.group` is an ordinary horizontally scrolling container, not a virtualized list.
More adds mock Research/Notes cards under distinct keys; constrained gives a
280-pixel width. The main card is 440 pixels horizontal or 180 vertical. Read
[Presentation.Attachment](../../lib/core/presentation.mli) for validated title,
trigger, slot status and propagation contracts. To add a real download, replace the
Save counter effect with a scoped application operation, keep its result/status
explicit, and preserve independent action/trigger ownership and borrowed asset lifetime.
