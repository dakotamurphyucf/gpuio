# Reviewing example walkthroughs

[The checklist](coverage.md) maps every tracked `.ml`, `.mli` and `.rs` under
`examples/` to one owner. An implementation and its adjacent interface normally
share a row. Rust adapter sources have separate rows because their audience and
ownership differ from ordinary OCaml application code. The inventory
exposes omissions as examples evolve; it does not certify READMEs by their presence.

`coverage.json` is the maintained source of the table. `walkthroughs` lists
existing companions worth reviewing, or is empty when one is missing. `review`
stays `pending` until a person or agent has read the source and checked the
contents below. New or substantially changed guides require renewed content review. `reviewed`
means documentation review, not native input, accessibility, performance or
platform qualification. Keep the evidence for those claims separate.

The role describes the reader's context, not an exemption from documentation:

- `application-entry`: startup and runtime composition.
- `component`: an independently meaningful example or reusable helper.
- `support`: models, shared presentation, routing or re-exports.
- `test-support` / `benchmark` / `diagnostic`: optional verification or measurement
  code. Explain launch flags and private instrumentation; do not recommend it as
  the first application template.
- `historical-bootstrap`: the original `foundation` host/protocol demonstration.
  Identify private infrastructure and link the current application API.
- `extension-author`: OCaml/Rust static component or document-profile packages.
  Identify which parts require Rust knowledge.
- `generated-registration`: checked-in backend registration glue. The owning
  example should explain its generator and build/link role; a redundant tutorial
  for each generated function is unnecessary. Review the classification against
  the actual file before marking it complete.

Dune stanzas, Cargo manifests/locks, consumer manifests, theme fixtures and other
assets are build inputs rather than independent components. Their owning
walkthrough must explain relevant entry points, generation and prerequisites.
Build-generated `_build` output and ignored local artifacts are excluded.
New source languages require extending the audit's explicit suffix set; do not
silently omit a new maintained component because it uses a different extension.

## Content review

Assume the reader is new to Bonsai as well as GPUIO. Explain the actual example's
implementation: name and link its functions, types and API calls, and describe
how they cooperate. Introduce graph construction, reactive values, `let%arr`,
`match%sub`, effects and lifecycle hooks when that example uses them. Trace an
event through the specific handler/model update/view derivation, including which
parts stay native. A list of library names or a visual feature tour is not enough.
Use small source excerpts where they clarify unfamiliar syntax; avoid copying
the entire file or paraphrasing every line without explaining why it exists.

For each independently reusable component, add an adjacent `component.md`. A
single-component application may use a complete adjacent `README.md`. A group of
support modules may share a guide if that guide links and explains each part.
Split rows as necessary when one module becomes independently reusable.

Read the final source and check that its guide explains:

1. Purpose, visible interactions and exact isolated-toolchain build/run commands.
2. Prerequisites, ordinary launch versus self-test, assets and actual platform limits.
3. Linked code map, reading order, important types, IDs, invariants and initial state.
4. Bonsai graph/state/effects/lifecycle as used, separately from GPUIO view APIs.
5. Native state and explicit commands/observations; asynchronous event delivery.
6. Eio capabilities, task scopes, cancellation and stale results where present.
   State when there is no I/O or reactive state instead of inventing layers.
7. An end-to-end interaction, including completion/cancellation for async examples.
8. Identity, resource lifetime, bounded work and relevant performance choices.
9. A realistic small adaptation and the invariants/pitfalls it must preserve.
10. Public interface/contract links and clear identification of mock data, private
    diagnostics and extension-author code.

Keep the owning README a discoverable entry point linking each companion. Include
the component's supporting interface and modules, without duplicating the entire
source. Validate links and commands, reusing relevant current build evidence for
documentation-only edits. Compilation alone never proves GUI or IME behavior.
Record review and verification in OCH-48, then set only the checked rows reviewed.

```sh
python3 scripts/audit_example_docs.py --write
python3 scripts/audit_example_docs.py
```

The first command renders the table from an intentionally edited inventory; it
does not discover omissions automatically or approve prose. The second rejects
missing/duplicate source ownership, invalid paths and a stale generated table.
Run it when adding, moving or removing an example source. It deliberately allows
explicit pending rows while OCH-48 is in progress. Completion requires **zero
pending maintained components** plus the content/link/command reviews above.

The audit also follows inline Markdown links from `examples/README.md` and each
application's own README to every reviewed companion. It excludes the generated
coverage table and review guide from these paths: an inventory entry alone is
not a newcomer reading path. Keep links in ordinary prose, not just code blocks,
and link a new family from the examples index. This checks discovery, not prose
quality, heading anchors, external URLs or runtime acceptance.
