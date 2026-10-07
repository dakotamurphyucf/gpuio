open Core
open Gpuio

(** Finite, named sample values. Selection is owned by the Assets page's Bonsai
    state; this module only describes and renders the controls. *)
type t =
  | Default
  | Quarter_turn
  | Mirror
  | Stretch
  | Offset
  | Combined
  | Collapsed
[@@deriving equal]

val label : t -> string
val transform : t -> Icon.Transform.t option

val controls
  :  Palette.t
  -> selected:t
  -> on_select:(t -> unit Bonsai.Effect.t)
  -> Gpuio_bonsai.View.t
