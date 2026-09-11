open Core

(** Colors are concrete RGBA values or named theme references. Theme resolution
    happens before submission; missing names produce a recoverable error. *)
type t [@@deriving equal, sexp_of]
val rgba : red:int -> green:int -> blue:int -> alpha:int -> t Or_error.t
val rgb_exn : int -> t
val token : string -> t Or_error.t
val token_exn : string -> t
module Expert : sig
  type value = Rgba of int64 | Token of string
  val value : t -> value
end
