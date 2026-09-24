open Core

(** Pure image configuration. This API neither decodes pixels nor performs I/O.
    Use [View.image] to place it; the native runtime owns decoding and rendering. *)
module Fit : sig
  type t =
    | Fill
    | Contain
    | Cover
    | Scale_down
    | None
  [@@deriving equal, sexp_of]
end

module Description : sig
  type t [@@deriving equal, sexp_of]

  (** Explicitly decorative: omit the image from the accessibility tree. *)
  val decorative : t

  (** Meaningful image: nonblank UTF-8 without NUL, at most 4096 bytes. *)
  val label : string -> t Or_error.t
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Defaults to [Contain]. Descriptions are required, including when the
      deliberate choice is [Description.decorative]. Styles provide layout. *)
  val create
    :  asset:Asset.Handle.t
    -> description:Description.t
    -> ?fit:Fit.t
    -> unit
    -> t

  val asset : t -> Asset.Handle.t
  val fit : t -> Fit.t
  val description : t -> Description.t
end

module Error : sig
  type t =
    | Wrong_application
    | Released
    | Invalid_data
    | Unsupported
    | Resource_limit
    | Native_failure
  [@@deriving equal, sexp_of]
end

module Metadata : sig
  type t [@@deriving equal, sexp_of]

  (** Pixel dimensions, distinct from logical layout dimensions. Width/height
      are positive and at most 16384; frame count is 1..120 and the combined
      full-frame BGRA footprint is at most 64 MiB. *)
  val create : width_px:int -> height_px:int -> frames:int -> t Or_error.t

  val width_px : t -> int
  val height_px : t -> int
  val frames : t -> int
end

module State : sig
  type t =
    | Loading
    | Ready of Metadata.t
    | Failed of Error.t
  [@@deriving equal, sexp_of]
end

module Expert : sig
  val label : Description.t -> string option

  (** Check before translating the asset ID. A foreign owner must become a
      local image failure, never an aliased resource in another application. *)
  val check_owner : Config.t -> owner:Asset.Expert.Owner.t -> (unit, Error.t) Result.t

  val to_wire
    :  Config.t
    -> owner:Asset.Expert.Owner.t option
    -> Gpuio_protocol.Wire.Image.Config.t

  val state_of_wire : Gpuio_protocol.Wire.Image.State.t -> State.t Or_error.t
end
