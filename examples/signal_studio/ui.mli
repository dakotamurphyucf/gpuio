open Core
module W = Signal_studio_model.Workspace

module Snapshot : sig
  type t =
    { workspace : W.t
    ; canvas : Gpuio.Canvas_scene.Handle.t option
    ; chart : Gpuio.Chart_resource.t option
    ; command : Gpuio.Canvas.Command.t option
    ; inspector : bool
    ; compact : bool
    ; running : bool
    ; status : string
    ; extension_generation : int64
    ; extension_disabled : bool
    ; extension_visible : bool
    ; documents : Documents.State.t
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
    ; on_motion : Gpuio.Animation.Program.Event.t -> unit Bonsai.Effect.t
    }
end

val view : Snapshot.t -> Actions.t -> Gpuio_bonsai.View.t
