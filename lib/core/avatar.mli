open Core

module Fallback : sig
  type t [@@deriving equal, sexp_of]

  (** Explicit initials or a short symbol; no name parsing or locale assumptions.
      Nonblank UTF-8, at most 128 bytes, without ASCII control characters. *)
  val create : string -> t Or_error.t

  val text : t -> string
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** The optional image uses the normal scoped asset registration. Missing,
      loading or failed image data displays [fallback] natively. Without an asset
      there is no image observation or artificial failure. Defaults to Cover.

      A meaningful description names the avatar once, including while showing its
      fallback; Decorative omits it from accessibility. Default size is 32x32
      logical pixels with circular corners. Ordinary styles override size/colors/
      radius/font. Fallback text is centered and shrunk to fit, never expanded
      beyond the requested font size. Changing source or fallback retains the
      native avatar identity; source replacement invalidates old observations.

      No file I/O, decoding, registration ownership or callbacks live in this value. *)
  val create
    :  ?asset:Asset.Handle.t
    -> ?fit:Image.Fit.t
    -> fallback:Fallback.t
    -> description:Image.Description.t
    -> unit
    -> t

  val fallback : t -> Fallback.t
  val description : t -> Image.Description.t
end

module Expert : sig
  val image : Config.t -> Image.Config.t option

  val to_wire
    :  Config.t
    -> owner:Asset.Expert.Owner.t option
    -> Gpuio_protocol.Avatar_wire.Config.t
end
