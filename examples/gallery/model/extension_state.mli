open Core

(** Bounded latest-state controller for the independently packaged counter.
    One command can be pending. Property/step changes wait for its matching
    acknowledgement; hiding and disabling input do not cancel explicit commands.
    Observations carry their captured generation. Reset replaces the generation;
    departure advances it and clears pending commands so obsolete observations
    cannot mutate the next visit and commands are never replayed on remount. *)
type t

module Action : sig
  type t =
    | Observe of int64 * int Gpuio.Extension.Event.t
    | Set_property
    | Send_command
    | Toggle_step
    | Toggle_disabled
    | Toggle_visible
    | Reset
    | Depart
end

val initial : t
val apply : t -> Action.t -> t
val value : t -> int
val step : t -> int
val generation : t -> int64
val command : t -> (int64 * int) option
val disabled : t -> bool
val visible : t -> bool
val notice : t -> string
