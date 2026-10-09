# How `Outline_test` checks pure approval policy

[README](README.md) · [Source](outline_test.ml) · [Policy](outline_data.md)

[Dune](dune) places this module in the `tree_example_tests` library with
`inline_tests` and `ppx_jane`. It has no standalone entry point, Bonsai graph,
Eio scope, GUI, or filesystem capability. `let%expect_test` declares Jane Street
inline tests whose captured output is compared with the following `[%expect]`
block. This syntax is test PPX, not reactive `let%arr`.

`snapshot ()` creates a pure `Gpuio.Tree_loading` controller from fresh outline
data and gets its immutable snapshot. `state` seeds both folders expanded.
`propose` captures typed endpoints and passes `Request.move` to
`Tree_interaction.apply`; it requires a `Move` outcome. This uses the same pure
request validation as the widget, without simulating native drag. `children`
reads ordered IDs from a branch for concise expected output.

The first test loops over Before, After, and Inside. It moves plan relative to
release or into Archive, calls `Outline_data.approve`, and checks the plan's
incarnation and payload are unchanged. Expected sibling lists show removal from
Inbox and insertion at the selected position. These identity assertions are
separate from the printed order; matching labels alone would not prove lifetime
preservation.

The second test moves plan after notes in its own parent, then separately
promotes it before Inbox in the root list. It checks atomic old/new sibling-list
updates. The third test tries an old proposal against a fresh snapshot generation
and against state with Inbox collapsed. Both approvals must return errors, and
the original source must still contain plan/notes in order. Its empty expected
output means successful assertions print nothing, not that the test does nothing.

Run the group from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j2 examples/tree
```

Use [development setup](../../docs/development.md). This command addresses the
local Dune inline-test alias; it does not launch `main.exe` or exercise native
focus, menus, modal approval tokens, filesystem paging, or GUI acceptance.
[Outline_demo](outline_demo.md) has separate bridge and real-input harness paths.

Add policy tests for meaningful new rules using fresh snapshots and typed
identity checks. Keep expected output focused on observable order/results, and
test rejection leaves the immutable input unchanged. Avoid replacing current
snapshot revalidation with fixture IDs that happen to share labels.
