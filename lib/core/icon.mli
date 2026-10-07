open Core

(** A monochrome SVG icon. The native foreground color (including inherited
    theme/hover/disabled styling) tints its alpha mask. *)
module Transform : sig
  type t [@@deriving equal, sexp_of]

  val identity : t

  (** Scale about the assigned icon box center, rotate clockwise, then translate.
      All inputs are finite. Scale is [-64,64], rotation [-360,360] degrees and
      translation [-16384,16384] logical pixels. Negative scale reflects; zero
      scale collapses artwork. Layout, input and accessibility bounds do not move.
      Fit and rounded clipping apply before the transform. Explicit overflow and
      ancestor clips apply afterward; native menus clip to their 16-pixel slot.
      Angle, scale, translation and foreground changes reuse the decoded mask.
      Fit/size/density/corner changes prepare a replacement asynchronously. *)
  val create
    :  ?scale_x:float
    -> ?scale_y:float
    -> ?rotation_degrees:float
    -> ?translate_x:float
    -> ?translate_y:float
    -> unit
    -> t Or_error.t

  val rotate_degrees : float -> t Or_error.t
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Requires an SVG registration; rejects a raster-format handle. Descriptions
      use the same meaningful/decorative contract as images. *)
  val create
    :  asset:Asset.Handle.t
    -> description:Image.Description.t
    -> ?fit:Image.Fit.t
    -> ?transform:Transform.t
    -> unit
    -> t Or_error.t

  (** Replace the complete transformation; [None] restores the default. *)
  val with_transform : t -> Transform.t option -> t
end

module Decoration : sig
  type t [@@deriving equal, sexp_of]

  (** A decorative SVG for a control's icon slot. Defaults to 16 logical pixels
      square and inherits foreground. Explicit styles override those defaults.
      The containing control supplies the accessible name and owns activation. *)
  val create
    :  asset:Asset.Handle.t
    -> ?style:Style.t
    -> ?transform:Transform.t
    -> unit
    -> t Or_error.t
end

module Expert : sig
  val image : Config.t -> Image.Config.t
  val transform : Config.t -> Transform.t option
  val transform_to_wire : Transform.t -> Gpuio_protocol.Icon_transform_wire.t
  val decoration : Decoration.t -> Config.t * Style.t
end
