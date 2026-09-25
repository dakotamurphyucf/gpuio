open Core

module Properties : sig
  type t

  val create : value:int -> ?step:int -> unit -> t Or_error.t
end

val schema : Gpuio.Extension.Schema.t

(** [set_value] is an optional (positive sequence, new value) command. Values are
    0..100; step is 1..10. Generation increments explicitly reset native state. *)
val instance
  :  Properties.t
  -> generation:int64
  -> ?disabled:bool
  -> ?set_value:int64 * int
  -> unit
  -> int Gpuio.Extension.Instance.t Or_error.t
