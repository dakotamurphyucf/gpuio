open Core

module Axis : sig
  type t =
    | Horizontal
    | Vertical
  [@@deriving equal, sexp_of]
end

module Config : sig
  (** Rust owns live geometry. Initial size is read on mount and when
      [reset_generation] or the axis changes; ordinary rerenders do not override a
      drag. Generations must not decrease for the same mounted split.
      Resizing the container redistributes native sizes. Children retain their
      identities when resetting geometry. Minimum sizes remain constraints;
      an undersized parent may clip content, so applications should choose a
      responsive layout or a sufficiently large parent. *)
  type t [@@deriving equal, sexp_of]

  val create
    :  label:string
    -> ?axis:Axis.t
    -> ?initial_first:float
    -> ?minimum_first:float
    -> ?maximum_first:float
    -> ?minimum_second:float
    -> ?keyboard_step:float
    -> ?reset_generation:int64
    -> unit
    -> t Or_error.t
end

module Snapshot : sig
  (** Observed native sizes in logical pixels, delivered after a completed
      pointer/keyboard/accessibility resize, not for every pointer movement. *)
  type t = private
    { first : float
    ; second : float
    }
  [@@deriving equal, sexp_of]
end

module Expert : sig
  val to_wire : Config.t -> Gpuio_protocol.Split_wire.Config.t
  val snapshot_of_wire : Gpuio_protocol.Split_wire.Snapshot.t -> Snapshot.t Or_error.t
  val generation : Config.t -> int64
end
