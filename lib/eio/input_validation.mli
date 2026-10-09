open Core

(** Syntax/resource-check a Rust-regex source before using it as a native input
    rule. The returned value is immutable source data, not a Rust handle.
    Requires an Eio context, but no running GPUI application or window.

    Preparation is serialized and runs in a system thread with the OCaml runtime
    released during native compilation. Cancellation waits for that bounded job
    to finish; it does not release the preparation slot while native work runs.
    No editor, callback registry or UI tree is changed. *)
val prepare_regex
  :  Gpuio.Input_validation.Regex.Source.t
  -> (Gpuio.Input_validation.Regex.t, Gpuio.Input_validation.Error.t) Result.t
