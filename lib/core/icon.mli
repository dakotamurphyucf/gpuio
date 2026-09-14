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

module Expert : sig
  val image : Config.t -> Image.Config.t
end
