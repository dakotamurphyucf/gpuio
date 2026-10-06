# A constrained-content presentation fixture

[content_cases.ml](content_cases.ml) is a diagnostic component, exposed by
[content_cases.mli](content_cases.mli). It cycles 19 presentation/form families
through Long, Empty and Localized content, making wrapping and action geometry
inspectable at narrow width. It is not the first application template. Read
`Content`, `Case`, `view`, then `component`; [main.md](main.md) owns startup.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/presentation/main.exe
_build/default/examples/presentation/main.exe --content-check
python3 scripts/test_presentation_content.py --images scratch/content-images-001
```

The launch opens a 440×860 window; the foreground macOS driver requires
Accessibility access and checks actual AX bounds/actions at narrow/wide widths
and both themes. Images are optional via `--images PATH`. There is no separate
executable or self-test in this module. Do not combine --content-check with the
main --self-test, which requires the other component's observations. No external
assets/service are needed, though startup still loads the main app's embedded
avatar sources. [dune](dune), [development](../../docs/development.md) and
[platform policy](../../docs/platform-release-policy.md) define setup/limits.

`Content.t` is Long/Empty/Localized with derived typed equality. `Content.text`
returns a long wrapping paragraph, empty string or Japanese/French/German/emoji
fixture. `Case.t` exhaustively names all 19 widget families; `Case.all` fixes
reading/cycling order and `Case.name` supplies diagnostic labels. `cases` is their
cartesian product: a bounded array of 57 combinations. No model loading or
unbounded dynamic list is involved.

`view case p content ~on_action` is presentation-only. It receives ordinary
values and a deferred effect, returning a GPUIO view. `P` is
`Gpuio.Presentation`; each match branch composes labels/tags/badges/markers,
links, groups/settings/descriptions, empty/alert/banner, shortcut/status,
attachments/messages/bubbles/tool results, or Form fields. Description uses
stable key 0 for its one entry. Empty Link uses actionable Open item rather
than an invalid blank label; empty field uses Optional field and omits help.
Form.field validates metadata/layout and attaches semantics to supplied control.
The checkbox is deliberately always Unchecked: activating it counts a fixture
action, not a durable preference toggle. Shortcut tokens are display-only.

`component` allocates persistent Bonsai index 0, dark true and action count 0.
`B.state` returns reactive state plus setters; `B.toggle` supplies theme state
and toggle effect. `let%arr` reads current ordinary values to derive a reactive
view; `and` declares dependencies, not threads. The index invariant is maintained
by Next case's modulo Array.length. Calling the view helper or constructing a
setter effect does not execute an action. Case/theme are application state;
there is no editor, native resource handle, lifecycle task or I/O here.

Click Open item in an attachment case: native button event arrives asynchronously,
the callback returns the action-count setter effect, Bonsai increments count,
and the footer's derived text returns through GPUIO native submission. Next case
similarly changes index; theme chooses `P.Appearance` and explicit root colors.
The full-height scroll column uses logical pixel padding/gaps so constrained
content can be inspected. A successful action or admitted transaction does not
prove physical display, VoiceOver speech or Linux desktop acceptance.

To add a new presentation family, extend Case's variant, all/name lists and view
match. Keep any required labels nonblank even in Empty fixtures, and preserve
visible actions when content wraps. Update the macOS driver's expected sequence
intentionally if acceptance automation depends on count/order. This component
can be extracted into another window using its interface; carry no native editor
lease across windows. See [Form](../../lib/core/form.mli) and
[Presentation](../../lib/core/presentation.mli) for validation/composition contracts.
