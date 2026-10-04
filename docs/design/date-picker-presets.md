# Date-picker presets

OCH-41 extends the existing controlled popup with complete civil-date shortcuts.
`Gpuio.Date_picker.Preset.create` takes a stable `Choice.Id`, accessible label and
validated selection. Single, complete range and empty selections are supported;
a partial range is rejected. Dates come from the application. The library never
reads the clock to decide what “today” or “last week” means.

`Preset.Collection` preserves order, requires unique IDs and bounds the menu to
32 items. Labels follow Choice's UTF-8/no-NUL/4096-byte contract. Duplicate labels
and values are valid. A wrong-mode or currently disallowed preset stays visible
but disabled. The same is true in disabled/read-only configurations.

`Gpuio_eio.Date_picker.view ~presets` adds native buttons above the calendar.
Each button has the stable preset ID and the supplied accessible name. No wire
operation or Rust ownership boundary is added: selecting a preset uses the
existing native calendar Replace command, guarded by the current draft revision.
`select_preset` is also available for custom application composition.

The controller rechecks its latest Bonsai input and popup session before sending
that command. Until a reply arrives, `is_selecting_preset` is true and Apply and
other presets are unavailable. Concurrent requests return `Native Busy` without
sending another native command. A confirmation already awaiting its native read
also cannot commit while a preset is pending. Confirmation after the replacement
must use an acceptable current revision.

A preset only changes the native draft. It never calls application `on_change`,
closes the popup, or moves its month cursor/focus. Applications can navigate
explicitly with calendar commands. Native constraints remain authoritative when
a command is processed. An admitted replacement may finish after read-only is
set; it is a programmatic draft command, and subsequent Apply still revalidates
current policy. Cancel and configuration changes that invalidate the opening
make delayed replies stale. A stale reply cannot clear a newer opening's pending
state. Native remounts and later observations also fence old leases/revisions.

Apply retains the original fresh-read/session/constraint validation. Cancel,
Escape and outside dismissal discard the opening. This intentionally differs
from the pinned styled DatePicker, whose preset immediately commits and closes.
The public contract remains explicit confirmation.

The private `Date_picker_component` accepts the native-command effect as a
capability so deterministic Bonsai-driver tests can delay/reorder completions.
The public `create` supplies the existing window command adapter. It does not
expose fake snapshots or synchronous native callbacks to applications.
