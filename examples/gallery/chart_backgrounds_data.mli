(** Pure, bounded fixture state. Identities and values belong to the original
    batches, independently of their display order or background representation. *)
type t [@@deriving equal]

module Baselines : sig
  type t =
    | Zero
    | Shared
    | Individual
  [@@deriving equal]

  val all : t list
  val label : t -> string
end

val initial : t
val advance : t -> t
val reorder : t -> t
val toggle_patterns : t -> t
val phase : t -> int
val is_reordered : t -> bool
val has_patterns : t -> bool
val baselines : t -> Baselines.t
val with_baselines : t -> Baselines.t -> t

(** 24 categorical bars with descending, noncontiguous IDs. Resolves accent,
    foreground and muted tokens using the supplied theme. This small validated
    fixture raises only if its supplied theme lacks those known tokens. *)
val data_exn : t -> theme:Gpuio.Theme.t -> Gpuio.Chart_data.t

(** Stable ID of Batch 01, even after reordering. *)
val first_id : Gpuio.Chart_data.Datum_id.t

val series_id : Gpuio.Chart_data.Series_id.t
