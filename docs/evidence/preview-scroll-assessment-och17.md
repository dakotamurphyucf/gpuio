# Preview scrolling assessment

The reported jarring movement and possible overlap occurred during the automated
loaded-history workload. That fixture deliberately issues immediate
`Controller.scroll_to` row/range jumps; it does not model wheel or trackpad motion.
The preview decision is to assess ordinary scrolling and retain the report for
follow-up, without claiming an unobserved rendering fix or requiring exhaustive
per-frame desktop qualification before users can try the framework.

Evidence relevant to that decision:

- The [ordinary-wheel diagnostic](list-scroll-diagnostics-och17.md#ordinary-scrolling-versus-benchmark-traversal)
  exercised the same 10k-row fixture with automatic traversal held. Eighty actual
  OS pixel-wheel events produced ordered, nonoverlapping sampled text bounds and
  524 retained-row comparisons with the expected 24-pixel displacement. The
  retained screenshot inspection shows separated rows and edge clipping. Its
  null `frontmost` field is not used as activity evidence; pointer ownership was
  checked separately.
- The [paint diagnostic](list-paint-geometry-och17.md) observes 24,338 list paints
  without row/content overlap or a list-state geometry mismatch. Its interrupted
  full run is diagnostic evidence only, not a passing full timing trial.
- The independently reproduced [inherited-metrics defect](list-inherited-metrics-och17.md)
  is repaired: changing inherited line height no longer turns a 24-pixel wheel
  delta into a 44-pixel row movement. Actual-window checks pass three line heights,
  retained anchors, exact wheel displacement and fresh painted bounds. Foundation
  [37805065605](https://github.com/dakotamurphyucf/gpuio/actions/runs/37805065605)
  passes that same regression. Native list/renderer/vendor sources have no changes
  from its `734fca1c` source through the preview checkpoint `87b43c73`.
- The [new optimized comparison](preview-list-comparison-och17.md) completes all
  three current-source instrumented trials, full traversal/growth/anchor checks,
  frame budgets, sixty-second zero-redraw idle and cleanup. The ordinary backend
  also completes all three full traversals. This replaces incomplete performance
  evidence, not the historical failure records.

These checks identify no reproducible ordinary-scroll blocker for the developer
preview. The abrupt stress traversal is expected application behavior. The known
metric-change defect is fixed; the original possible transient overlap is **not
claimed fixed**, nor is its cause retroactively assigned to that defect or to a
covered window. There is no per-frame video or hardware-momentum certification.

P1 is complete for the focused preview scope. Keep transient-jitter observation
and broader gesture qualification in OCH-164: if a real application reproduces
overlap or jumps during ordinary scrolling, obtain its revision, minimal example,
recording, scroll source and visibility/display conditions and investigate that
specific case. A reproduced normal-use defect still requires a fix. This is the
owner's usable-preview/feedback boundary, not a claim that every visual edge case
has been eliminated.
