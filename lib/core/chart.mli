open Core

module Error : sig
  type t =
    | Wrong_application
    | Unavailable_data
    | Render_limit
    | Native_failure
  [@@deriving equal, sexp_of]
end

module Metrics : sig
  (** Preparation counts, not frame latency or operating-system memory. Retained
      values count source representatives after explicit reduction, including
      zero-area values. [bytes] charges the retained geometry, meshes and hit-test index. *)
  type t =
    { source_values : int
    ; retained_values : int
    ; mesh_vertices : int
    ; quads : int
    ; bytes : int
    }
  [@@deriving equal, sexp_of]
end

module Selection = Chart_selection

module Observation : sig
  (** Selection observations identify a committed native target or an explicit
      clear. Hover and drag previews do not cross into OCaml. Source positions
      are relative to the event's data revision; they must not be applied to a
      newer publication. Native input integration is still under development. *)
  type t =
    | Ready of Metrics.t
    | Failed of Error.t
    | Selection_changed of Selection.t option
  [@@deriving equal, sexp_of]
end

module Event : sig
  (** Observations name the data publication, independently of the view-tree
      revision. [Ready] means native preparation completed, not presentation to
      the user. Failure before acquiring data uses revision/generation zero. *)
  type t =
    { data_revision : int64
    ; data_generation : int64
    ; observation : Observation.t
    }
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Borrows an application-scoped dataset. View style determines total size;
      native layout reserves room for labels. Unmount releases the view's work
      and reader, not its registration. Release/close retires displayed data.
      Published changes coalesce native preparation and preserve the last ready
      picture until its replacement is prepared. A reset or source change clears
      that picture. Wrong-application handles never cross as unchecked IDs.

      [legend] defaults to true. Dense legends scroll within the chart; labels
      may ellipsize visually while retaining their full accessible text.

      Label is nonblank, valid UTF-8, at most 1024 bytes, without NUL/CR/LF.
      Large/complex geometry can report [Render_limit]; no implicit policy change
      or gap removal is performed to force it to fit. *)
  val create
    :  data:Chart_resource.t
    -> ?label:string
    -> ?legend:bool
    -> ?options:Chart_options.t
    -> ?sampling:Chart_sampling.t
    -> ?style:Chart_style.t
    -> unit
    -> t Or_error.t

  val data : t -> Chart_resource.t
end

module Expert : sig
  val to_wire
    :  Config.t
    -> owner:Chart_resource.Expert.Owner.t option
    -> Gpuio_protocol.Chart_view_wire.Config.t

  val event
    :  data_revision:int64
    -> data_generation:int64
    -> Gpuio_protocol.Chart_view_wire.Observation.t
    -> Event.t Or_error.t
end
