open Core

module Token : sig
  (** Identity of a particular user choice, not value equality of its palette.
      Even selecting the same preset again invalidates an older file request. *)
  type t

  val equal : t -> t -> bool
end

type t

val initial : unit -> t
val token : t -> Token.t
val preference : t -> Appearance.Preference.t
val profile : t -> Theme_profile.t option
val choose : t -> Appearance.Preference.t -> t
val resolve : t -> native:Gpuio.Window.Appearance.t option -> Appearance.t

(** Failure preserves the last good selection. A result captured before another
    user choice is ignored, including a choice with equal colors. *)
val complete : t -> token:Token.t -> Theme_profile.t Or_error.t -> t
