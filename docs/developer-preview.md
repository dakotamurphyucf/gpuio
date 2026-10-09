# GPUIO developer preview

GPUIO's source-library developer preview is for people building native OCaml
applications and reporting what works, what breaks and what is missing.
The first preview's release identity is **`v0.1.0-preview.1`**. Check
[GitHub Releases](https://github.com/dakotamurphyucf/gpuio/releases) for publication
status and the exact tagged revision; an unpublished draft is not an available
release. The [closeout checklist](milestone-07-closeout.md) records its
qualification scope and remaining work.

## Try an application

Start with the [prerequisites and isolated toolchain](development.md), then
[build the starter](getting-started.md). The setup keeps its opam environment in
the checkout and does not change your default switch. Rust is needed to compile
the renderer, but ordinary application code uses the OCaml APIs.

Once the release appears on GitHub Releases, clone its tag to reproduce that
source revision:

```sh
git clone --branch v0.1.0-preview.1 --depth 1 https://github.com/dakotamurphyucf/gpuio.git
cd gpuio
./scripts/gpuio bootstrap
GPUIO_JOBS=2 ./scripts/gpuio build examples/getting_started/main.exe
_build/default/examples/getting_started/main.exe
```

The final command opens the interactive starter. A source tag identifies this
preview as a whole; it does not establish a stable binary ABI or independently
published opam package version.

The [starter walkthrough](../examples/getting_started/README.md) explains a small
Bonsai application, including how state, callbacks and native views fit together.
Explore [Component Studio](../examples/gallery/README.md) for individual controls,
[Agent Workspace](../examples/agent_chat/README.md) for a streaming chat interface,
and [Signal Studio](../examples/signal_studio/README.md) for graphics and charts.
Each example's README links its code walkthroughs. These examples demonstrate
application composition; the chat example is not a bundled production LLM service.

The getting-started guide also covers an independent project linked against
installed public libraries. Use its isolated prefix workflow and a single matching
GPUIO revision for OCaml packages, native archives and extension packages. There
is no published upstream opam package or standalone binary SDK implied by this
preview. Once a prerelease is published, pin its tag or commit instead of depending
on a moving development branch.

## What to expect

The public OCaml APIs cover application/window lifetimes, Bonsai state and effects,
styles/themes, native text input, virtual lists and paging, read-only tables and
trees, Markdown/code display, charts, canvas, animation, and desktop services.
The [coverage inventory](catalog/preview-coverage.md) maps the implemented component
families to examples and scoped evidence. The [API layer map](api-layers.md)
explains when to use `Gpuio`, `Gpuio_bonsai` and `Gpuio_eio`.

This is an experimental API. Pin your dependencies and expect migration work
between preview revisions. Persist your application's own versioned data, not
native handles or internal protocol records. Read [compatibility and limits](api-compatibility.md)
for ownership, asynchronous completion, admission limits and deferred components.

macOS is the first functional target, with a 14.4 deployment target and local
qualification on Apple Silicon. Linux has required build, unit/private-bus and
independent-consumer checks; full X11/Wayland desktop qualification is deferred.
Windows is outside the current scope. See [platform policy](platform-release-policy.md)
for the distinction between builds and desktop acceptance.

The preview does not certify complete screen-reader support, every OS/gesture
combination, or signed/notarized reference applications. Timing evidence covers
named workloads and hardware; it is not a universal 120 FPS guarantee.
[Current status](status.md) records open issues and the exact coverage obtained.
[Distribution](distribution.md) explains what remains necessary to ship your own
application. These limits should inform adoption, without preventing experiments
with the implemented APIs.

## Give feedback

[Open a GitHub issue](https://github.com/dakotamurphyucf/gpuio/issues/new/choose).
A small example using the public APIs is particularly helpful. Include:

- The behavior you expected, what happened, and exact steps to reproduce it.
- Your GPUIO tag or commit (`git rev-parse HEAD` in the framework checkout),
  local modifications, OS version, architecture and build command/profile.
- The smallest relevant OCaml code and Dune dependencies, or the existing example
  and interaction that reproduces the problem. Include custom native extensions
  or document profiles when they affect the case.
- Relevant error output and whether the issue occurs consistently.

For scrolling, flicker or input problems, include a short recording when possible,
window/display scaling and refresh rate, and whether the window was active,
covered or minimized. For Linux, include X11/Wayland and the GPU/driver. For memory
or latency reports, include data size, update frequency and how you measured it.
Do not run the whole qualification suite just to file an issue.

For missing capabilities or awkward APIs, describe the application you are trying
to build, the user interaction you need, and any workaround you tried. A concrete
use case helps us distinguish an API gap from missing documentation or a component
that can already be composed from existing primitives.
