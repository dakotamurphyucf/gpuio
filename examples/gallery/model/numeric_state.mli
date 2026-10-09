(** One reducer owns the rating and read-only policy so queued rating requests
    are checked against the latest policy, not an earlier rendered value. *)
type t

module Action : sig
  type t =
    | Toggle_read_only
    | Cycle_rating_maximum
    | Cycle_rating_size
    | Toggle_rating_disabled
    | Toggle_rating_colors
    | Toggle_rating_step_down
    | Rate of Gpuio.Rating.Request.t
end

val initial : t
val apply : t -> Action.t -> t
val is_read_only : t -> bool
val rating : t -> Gpuio.Rating.Config.t
val custom_rating_colors : t -> bool
val step_down_rating : t -> bool
