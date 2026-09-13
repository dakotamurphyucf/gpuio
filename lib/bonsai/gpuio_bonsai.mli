open Core

(** The pure view API specialized to Bonsai effects. No driver, I/O runtime or
    scheduling policy is introduced here; window lifecycle scheduling is OCH-9. *)
module View : sig
  type t = unit Bonsai.Effect.t Gpuio.View.t

  val text : ?key:Gpuio.Key.t -> ?style:Gpuio.Style.t -> string -> t

  val button
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?accessible_name:string
    -> ?disabled:bool
    -> on_click:unit Bonsai.Effect.t
    -> string
    -> t

  val checkbox
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?accessible_name:string
    -> ?disabled:bool
    -> state:Gpuio.Check_state.t
    -> on_toggle:unit Bonsai.Effect.t
    -> string
    -> t

  val switch
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?accessible_name:string
    -> ?disabled:bool
    -> checked:bool
    -> on_toggle:unit Bonsai.Effect.t
    -> string
    -> t

  val focus_scope
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Focus_scope.t
    -> t list
    -> t

  val row : ?key:Gpuio.Key.t -> ?style:Gpuio.Style.t -> t list -> t

  val radio_group
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> config:Gpuio.Choice.Config.t
    -> on_select:(Gpuio.Choice.Id.t -> unit Bonsai.Effect.t)
    -> unit
    -> t

  val select
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> ?appearance:Gpuio.Choice.Appearance.t
    -> config:Gpuio.Choice.Config.t
    -> on_select:(Gpuio.Choice.Id.t -> unit Bonsai.Effect.t)
    -> unit
    -> t

  val combobox
    :  ?style:Gpuio.Style.t
    -> ?appearance:Gpuio.Choice.Appearance.t
    -> ?initial_text:string
    -> controller:Gpuio.Key.t
    -> config:Gpuio.Combobox.Config.t
    -> on_event:(Gpuio.Combobox.Event.t -> unit Bonsai.Effect.t)
    -> unit
    -> t Or_error.t

  val column : ?key:Gpuio.Key.t -> ?style:Gpuio.Style.t -> t list -> t

  val grid
    :  ?key:Gpuio.Key.t
    -> ?style:Gpuio.Style.t
    -> columns:int
    -> t list
    -> t Or_error.t
end
