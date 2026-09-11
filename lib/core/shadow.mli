open Core

type t [@@deriving equal, sexp_of]
val create
  :  ?inset:bool
  -> color:Color.t
  -> offset_x:float
  -> offset_y:float
  -> blur:float
  -> spread:float
  -> unit
  -> t Or_error.t
module Expert : sig
  type description = { color:Color.t; offset_x:float; offset_y:float; blur:float; spread:float; inset:bool }
  val describe : t -> description
end
