open Core

(** Stable-ID pie caption and leader-line presentation. Source names, weights,
    original-data rows and legend entries remain independent. *)
module Entry : sig
  type t [@@deriving equal, sexp_of]

  (** Omitted text inherits the source label; empty text hides the caption and
      leader. At most 256 UTF-8 bytes without ASCII controls. Line colors affect
      Outside labels and resolve against the chart style's theme. *)
  val create
    :  slice:Chart_data.Datum_id.t
    -> ?text:string
    -> ?line_color:Color.t
    -> unit
    -> t Or_error.t
end

type t [@@deriving equal, sexp_of]

(** At most 256 unique slice IDs and 32 KiB of override text. Unknown IDs are
    retained but ignored until present in the source. *)
val create : Entry.t list -> t Or_error.t

val empty : t

module Expert : sig
  val to_wire : t -> theme:Theme.t -> Gpuio_protocol.Chart_pie_labels_wire.t Or_error.t
end
