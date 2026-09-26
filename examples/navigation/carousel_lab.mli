open Core

module Action : sig
  type t =
    | Navigate of Gpuio.Carousel.Request.t
    | Toggle_axis
    | Toggle_looping
    | Toggle_autoplay
    | Toggle_lifetime
    | Toggle_disabled
    | Trim
    | Restore
    | Increment
end

module Model : sig
  type t

  val selected_label : t -> string
  val axis : t -> Gpuio.Carousel.Axis.t
  val hidden : t -> Gpuio.Content_policy.t
  val counter : t -> int
  val requests : t -> int
end

module Observation : sig
  type t =
    { model : Model.t
    ; inject : Action.t -> unit Bonsai.Effect.t
    ; editor : Gpuio_eio.Text_input.t
    }
end

val initial_draft : string

(** The Bonsai component and editor controller stay active regardless of native
    content policy. Unmount deliberately recreates the native draft from its
    initial text; application data tasks belong to the enclosing workspace. *)
val component
  :  observed:Observation.t option ref
  -> Gpuio_eio.App.Window.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
