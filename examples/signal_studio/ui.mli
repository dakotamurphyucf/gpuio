open Core
module Workspace = Signal_studio_model.Workspace
module Alerts = Signal_studio_notifications.Run_alerts

module Snapshot : sig
  type t =
    { workspace : Workspace.t
    ; canvas : Gpuio.Canvas_scene.Handle.t option
    ; chart : Gpuio.Chart_resource.t option
    ; command : Gpuio.Canvas.Command.t option
    ; inspector : bool
    ; compact : bool
    ; running : bool
    ; status : string
    ; extension_generation : int64
    ; extension_command : (int64 * int) option
    ; extension_disabled : bool
    ; extension_visible : bool
    ; documents : Documents.State.t
    ; alerts : Alerts.State.t
    ; alerts_open : bool
    }
end

module Actions : sig
  type t =
    { select : Gpuio.Canvas_scene.Item_id.t -> unit Bonsai.Effect.t
    ; canvas : Gpuio.Canvas.Event.t -> unit Bonsai.Effect.t
    ; chart : Gpuio.Chart.Event.t -> unit Bonsai.Effect.t
    ; extension : int Gpuio.Extension.Event.t -> unit Bonsai.Effect.t
    ; inspector : unit Bonsai.Effect.t
    ; run : unit Bonsai.Effect.t
    ; reset : unit Bonsai.Effect.t
    ; reset_viewport : unit Bonsai.Effect.t
    ; lock_control : unit Bonsai.Effect.t
    ; hide_control : unit Bonsai.Effect.t
    ; on_layout : Gpuio.Container_query.Selection.t -> unit Bonsai.Effect.t
    ; open_document : unit Bonsai.Effect.t
    ; save_document : unit Bonsai.Effect.t
    ; reveal_document : unit Bonsai.Effect.t
    ; quit : unit Bonsai.Effect.t
    ; toggle_alerts : unit Bonsai.Effect.t
    ; close_alerts : unit Bonsai.Effect.t
    ; enable_alerts : unit Bonsai.Effect.t
    ; notify_run : unit Bonsai.Effect.t
    ; dismiss_alert : unit Bonsai.Effect.t
    ; on_motion : name:string -> Gpuio.Animation.Program.Event.t -> unit Bonsai.Effect.t
    }
end

val view : Snapshot.t -> Actions.t -> Gpuio_bonsai.View.t
