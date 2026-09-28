(** One reducer owns the rating and read-only policy so queued rating requests
    are checked against the latest policy, not an earlier rendered value. *)
type t

module Action : sig
  type t =
    | Toggle_read_only
    | Rate of Gpuio.Rating.Request.t
end

val initial : t
val apply : t -> Action.t -> t
val is_read_only : t -> bool
val rating : t -> Gpuio.Rating.Config.t
