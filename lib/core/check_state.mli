open Core

(** A checkbox's application value. Native activation requests are applied to
    the current model, so rapid activations do not reuse a stale rendered value. *)
type t =
  | Unchecked
  | Checked
  | Indeterminate
[@@deriving equal, sexp_of]

(** An indeterminate checkbox becomes checked on activation. *)
val activate : t -> t

val of_bool : bool -> t
