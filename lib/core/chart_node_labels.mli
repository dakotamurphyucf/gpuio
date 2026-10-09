open Core

(** ID-keyed Sankey label presentation. This collection owns immutable text and
    styles, never a callback into layout or paint. Original dataset labels and
    selection values are independent of these presentation overrides. *)
module Line : sig
  type t [@@deriving equal, sexp_of]

  (** At most 256 UTF-8 bytes without ASCII control characters. Optional font size
      is 8..32 logical pixels; omitted font/color inherit native chart defaults.
      Colors, including tokens, are resolved against the chart style's theme. *)
  val create : ?color:Color.t -> ?font_size:float -> string -> t Or_error.t
end

module Node : sig
  type t [@@deriving equal, sexp_of]

  (** Zero to four lines. An empty list explicitly suppresses the node label. *)
  val create : node:Chart_data.Node_id.t -> Line.t list -> t Or_error.t
end

type t [@@deriving equal, sexp_of]

(** At most 128 unique node IDs and 32 KiB of text across all lines. *)
val create : Node.t list -> t Or_error.t

val empty : t

module Expert : sig
  val to_wire : t -> theme:Theme.t -> Gpuio_protocol.Chart_node_labels_wire.t Or_error.t
end
