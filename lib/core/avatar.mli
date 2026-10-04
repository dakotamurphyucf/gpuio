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

(** Optional deterministic identity colors. This never parses names or changes
    [Config]; callers explicitly apply the resulting style. *)
module Palette : sig
  module Appearance : sig
    type t =
      | Light
      | Dark
    [@@deriving equal, sexp_of]
  end

  type t [@@deriving equal, sexp_of]

  (** Hash the exact key bytes using FNV-1a-32 (offset 2166136261, prime 16777619,
      wrap modulo 2^32), then take modulo 12. No normalization, case folding,
      process seed or Rust hash implementation participates. This is a stable
      visual mapping, not a cryptographic identifier. *)
  val for_key : Key.t -> appearance:Appearance.t -> t

  (** The same mapping on the explicit fallback bytes. Changing initials may
      change the palette; use [for_key] when identity should survive that change. *)
  val for_fallback : Fallback.t -> appearance:Appearance.t -> t

  val index : t -> int
  val background : t -> Color.t
  val foreground : t -> Color.t
  val border : t -> Color.t

  (** Three color declarations only; does not set border width, size, shape,
      source, or opacity. Merge caller overrides after this style as usual.
      The fixed opaque foreground/background pairs meet a 4.5:1 nominal sRGB
      contrast target. Images, opacity, modified colors and rendered glyph
      antialiasing are outside that numerical guarantee. *)
  val style : t -> Style.t
end
