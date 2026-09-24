open Core

(** A monochrome SVG icon. The native foreground color (including inherited
    theme/hover/disabled styling) tints its alpha mask. *)
module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Requires an SVG registration; rejects a raster-format handle. Descriptions
      use the same meaningful/decorative contract as images. *)
  val create
    :  asset:Asset.Handle.t
    -> description:Image.Description.t
    -> ?fit:Image.Fit.t
    -> unit
    -> t Or_error.t
end

module Decoration : sig
  type t [@@deriving equal, sexp_of]

  (** A decorative SVG for a control's icon slot. Defaults to 16 logical pixels
      square and inherits foreground. Explicit styles override those defaults.
      The containing control supplies the accessible name and owns activation. *)
  val create : asset:Asset.Handle.t -> ?style:Style.t -> unit -> t Or_error.t
end

module Expert : sig
  val image : Config.t -> Image.Config.t
  val decoration : Decoration.t -> Config.t * Style.t
end
