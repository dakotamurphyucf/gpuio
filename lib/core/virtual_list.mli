open Core

module Height : sig
  type t =
    | Estimated of float
    | Fixed of float
  [@@deriving equal, sexp_of]
end

module Scroll_policy : sig
  type t =
    | Keep_position
    | Follow_tail_when_at_end
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Heights and overscan are logical pixels. Height must be in [1, 1_000_000].
      [max_active] includes pinned rows and must be in [1, 16384]. Native layout
      reports budget exhaustion rather than allocating an unbounded active set.
      A fixed height clips content to that height; an estimate is replaced by
      native measurement. *)
  val create
    :  ?overscan:float
    -> ?max_active:int
    -> ?scroll:Scroll_policy.t
    -> ?scrollbar:bool
    -> height:Height.t
    -> unit
    -> t Or_error.t

  val height : t -> Height.t
  val max_active : t -> int
end

module Order : sig
  (** Validated immutable key order. Share it across value-only updates. This
      contains O(n) metadata and no row views or application records. *)
  type t

  val create : Key.t list -> t Or_error.t
  val keys : t -> Key.t list
  val length : t -> int
  val mem : t -> Key.t -> bool
end

module Viewport : sig
  (** Visible indices are a half-open interval in the current logical order.
      [requested] includes overscan. [pinned] is independent of visibility.
      These observations do not themselves signify Bonsai activation. *)
  type t =
    { visible_first : int
    ; visible_last : int
    ; requested : Key.t list
    ; pinned : Key.t list
    ; anchor : (Key.t * float) option
    ; following_tail : bool
    ; at_start : bool
    ; at_end : bool
    ; budget_exhausted : bool
    }
  [@@deriving equal, sexp_of]
end

module Scroll_request : sig
  type t [@@deriving equal, sexp_of]

  (** Serials must be positive and increase for new commands. A retained command
      is executed once, after the same transaction's data updates. The managed
      Bonsai controller supplies serials for ordinary applications. *)
  val to_row : serial:int64 -> ?offset:float -> Key.t -> t Or_error.t

  val reveal : serial:int64 -> Key.t -> t Or_error.t
  val to_end : serial:int64 -> unit -> t Or_error.t
end

module Expert : sig
  val config_to_wire : Config.t -> managed:bool -> Gpuio_protocol.List_wire.Config.t
  val row_style : Config.t -> Style.t

  val viewport_of_wire
    :  Gpuio_protocol.List_wire.Viewport.t
    -> find_key:(int64 -> Key.t option)
    -> Viewport.t Or_error.t

  val scroll_to_wire
    :  Scroll_request.t
    -> find_id:(Key.t -> int64 option)
    -> Gpuio_protocol.List_wire.Scroll_request.t Or_error.t
end
