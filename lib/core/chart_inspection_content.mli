open Core

(** Declarative content keyed to an inspected chart target. Metadata contains
    source identities but no callback or native pointer; ordinary Views supply
    the content through [View.chart ~inspection_content]. Native arbitrary-child
    integration is experimental; broader widget/lifecycle qualification is ongoing. *)
module Target : sig
  type t [@@deriving equal, sexp_of]

  val cartesian : series:Chart_data.Series_id.t -> datum:Chart_data.Datum_id.t -> t
  val slice : Chart_data.Datum_id.t -> t
  val radar : series:Chart_data.Series_id.t -> axis:Chart_data.Datum_id.t -> t
  val candlestick : Chart_data.Datum_id.t -> t
  val node : Chart_data.Node_id.t -> t
  val edge : Chart_data.Edge_id.t -> t

  (** Singular targets discard source positions and retain stable IDs. Sum/Mean
      and OHLC targets retain the exact selection and publication; even length-one
      aggregates are not singular targets. The target borrows [data] and checks
      its application and resource identity when metadata is bound. Both
      publication numbers must be positive. Aggregate content cannot transfer
      to a newer publication merely
      because endpoints/count match. *)
  val of_selection
    :  Chart_selection.t
    -> data:Chart_resource.t
    -> data_revision:int64
    -> data_generation:int64
    -> t Or_error.t

  module Expert : sig
    val to_wire : t -> Gpuio_protocol.Chart_inspection_content_wire.Target.t

    val of_wire
      :  Gpuio_protocol.Chart_inspection_content_wire.Target.t
      -> data:Chart_resource.t
      -> t Or_error.t
  end
end

module Container : sig
  type t =
    | Card
    | Overlay
  [@@deriving equal, sexp_of]
end

module Entry : sig
  type 'view t

  val create : target:Target.t -> ?container:Container.t -> 'view -> 'view t
  val target : _ t -> Target.t
  val container : _ t -> Container.t
  val content : 'view t -> 'view
end

type 'view t

(** At most 128 entries with distinct wire targets. Duplicate admission ignores
    collection order, generic content, container choice and application owner:
    one chart cannot mount content from multiple applications. Content is not
    serialized by the metadata codec. Retaining a target does not make it present
    in a dataset. *)
val create : 'view Entry.t list -> 'view t Or_error.t

val empty : 'view t

module Expert : sig
  val entries : 'view t -> 'view Entry.t list

  (** Aggregate targets bound to a different application/resource become hidden
      metadata slots. Generic content remains retained; no borrowed handle gains
      authority through this conversion. Singular IDs are local to the chart. *)
  val metadata
    :  _ t
    -> data:Chart_resource.t
    -> Gpuio_protocol.Chart_inspection_content_wire.t
end
