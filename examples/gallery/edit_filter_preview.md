# Prepared edit-filter walkthrough

Read [edit_filter_preview.ml](edit_filter_preview.ml) and its [interface](edit_filter_preview.mli). [application.ml](application.ml) calls `prepare ()` during Eio initialization before windows; [pages.ml](pages.ml) passes the resulting immutable rules into **Text editing → Options**.

`B` aliases `Bonsai.Cont`, `V` aliases `Gpuio_bonsai.View`, and `Editor` aliases `Gpuio_eio.Text_input`. Prepared `rules` is an ordinary immutable OCaml value; palette, mode, notice and controller outputs are reactive Bonsai values. `graph` hosts state/controller computations, and `let%arr` reads current reactive inputs to derive configuration or UI descriptions when they change. `V` constructs those descriptions rather than performing edits.

Abstract `t` contains handle/reference Input_validation rules. `prepare` validates bounded regex sources and calls Gpuio_eio.Input_validation.prepare_regex, translating compile errors into Or_error. It prepares `[a-z0-9_-]*` and `[0-9-]*` once; compiling during Bonsai evaluation would be the wrong boundary. [`Input_validation`](../../lib/core/input_validation.mli) checked values are immutable source data with no Rust handle/window lifetime, and native ingress still revalidates them. Whole-value matching is the default; these expressions permit incremental prefixes, including empty input.

`Mode.t` is Handle/Reference/Free. `B.state Handle` and notice are reactive values. Configuration `let%arr` chooses the prepared handle filter, reference filter plus `Input_format.Pattern "99-99"`, or neither. Editor.create seeds one native single-line placement with `workspace-17`. Native Reference activation executes set_mode Reference, updates the model, derives config/help/selected-button text and reconciles the existing editor. Constructing setters does not execute them. The old draft stays even if incompatible with the newly selected rule.

Filtering runs on formatted text, explaining why Reference allows the inserted hyphen. It guides native typing/paste without calling an OCaml predicate during editing. It is not business/submission validation; undo may restore incompatible history, and submissions still carry the exact draft. Prepared source permits at most 2,048 UTF-8 bytes and does not support backreferences/lookaround. Preparation alone attaches no policy.

Load matching sample uses the latest observed snapshot when constructing its effect. On native activation, effect `let%bind` awaits Editor.replace_if_unchanged with End selection/Record undo; native lease/revision/composition checks prevent it overwriting intervening edits. Its result sets notice, triggering reactive view derivation and native readout update. No snapshot gives “not ready”. Auxiliary buttons preserve existing editor focus on pointer activation and have no separate Tab/keyboard/semantic Focus target.

Try punctuation/uppercase, switch rules without replacing text, then load a sample and Undo. GPUIO owns filtering/editing/history, the adapter owns guarded commands, Bonsai owns mode/notice, and application initialization owns prepared immutable rules. No asset or background scope is needed. Adapt with prefix-compatible rules, preparation errors handled before mounting, and separate complete-value validation before accepting a form. Do not assume mounting a filter repairs historical text or invalidates every past undo state.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not executed checks for this documentation change. There is no standalone executable or self-test for this component. Compilation does not establish native keyboard/IME/focus or platform acceptance. See [gallery instructions](README.md) and [development](../../docs/development.md).
