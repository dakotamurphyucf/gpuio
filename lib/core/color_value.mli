open Core

(** Concrete selected colors, independent of theme references. Pure values own
    no native state. Conversions use encoded sRGB, not linear-light RGB. *)
module Hsla : sig
  type t [@@deriving equal, sexp_of]

  (** Finite hue in [0,360], with 360 canonicalized to zero. Other channels
      must be finite and in [0,1]. No clamping of invalid input. *)
  val create
    :  hue_degrees:float
    -> saturation:float
    -> lightness:float
    -> alpha:float
    -> t Or_error.t

  val hue_degrees : t -> float
  val saturation : t -> float
  val lightness : t -> float
  val alpha : t -> float
end

module Rgba : sig
  type t [@@deriving equal, compare, sexp_of]

  (** Straight-alpha sRGB bytes, each in [0,255]. *)
  val create : red:int -> green:int -> blue:int -> alpha:int -> t Or_error.t

  val red : t -> int
  val green : t -> int
  val blue : t -> int
  val alpha : t -> int

  (** HSL operates on encoded sRGB; quantize with floor(x*255+0.5).
      Achromatic conversion uses zero hue. Native editing hue memory is separate. *)
  val of_hsla : Hsla.t -> t

  val to_hsla : t -> Hsla.t
  val to_color : t -> Color.t

  (** Strict ASCII 3/4/6/8 hex digits, optionally preceded by '#'. *)
  val of_hex : string -> t Or_error.t

  (** Uppercase [#RRGGBB] when opaque, otherwise [#RRGGBBAA]. *)
  val to_hex : t -> string
end

module Value : sig
  type t =
    | Empty
    | Color of Rgba.t
  [@@deriving equal, sexp_of]
end

module Alpha_policy : sig
  type t =
    | Allow_alpha
    | Opaque_only
  [@@deriving equal, sexp_of]
end

module Hex_draft : sig
  module Error : sig
    type t =
      | Syntax
      | Too_long
    [@@deriving equal, sexp_of]
  end

  type t = private
    | Empty
    | Incomplete
    | Invalid of Error.t
    | Valid of Rgba.t
  [@@deriving equal, sexp_of]

  (** Does not trim whitespace or rewrite the supplied text. Empty input is
      [Empty]; '#' and 1/2/5/7 hex digits are [Incomplete]; 3/4/6/8 digits
      are [Valid]. More than nine total bytes is [Invalid Too_long]. *)
  val parse : string -> t
end
