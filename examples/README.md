# Examples

`foundation/` is the bootstrap Bonsai/Eio/GPUI window and input example. Run it with
`./scripts/gpuio smoke`; `--self-test` exercises the bridge/lifecycle/input handlers
and cleanup, while `--two-windows` exercises native window identity and independent
editor lifetime. These tests open actual windows, but input probes invoke native
handlers; they do not prove real OS IME or accessibility behavior.

The bootstrap protocol and single-window application host are private to this
example. OCH-7–9 implement the production protocol/API/scheduler. The two-window
scenario is native scaffolding, not the future per-window Bonsai application API.
Agent chat and graphics examples grow with subsequent milestones.
