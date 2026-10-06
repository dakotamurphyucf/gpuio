(** Optional native integration/resource checks. Ordinary startup does not depend
    on their assertions or paint-readiness waits. *)
module Context : sig
  type t =
    { initial : Signal_studio_model.Workspace.t
    ; state : Ui.Snapshot.t Bonsai.Cont.Expert.Var.t
    ; window : Gpuio_eio.App.Window.t
    ; scene : Gpuio_eio.Canvas.t
    ; signal : Gpuio_eio.Chart.t
    ; chart_ready : int64 ref
    ; extension_mounts : int ref
    ; extension_ack : int64 ref
    ; ack : int64 ref
    ; motion_events : int ref
    ; layout : string ref
    ; ensure_window : ?focus:bool -> unit -> Gpuio_eio.App.Window.t
    ; set_run : int -> unit
    ; update : (Ui.Snapshot.t -> Ui.Snapshot.t) -> unit
    ; command : Gpuio.Canvas.Command.Action.t -> int64
    ; on_extension : int Gpuio.Extension.Event.t -> unit Bonsai.Effect.t
    ; publish : Signal_studio_model.Workspace.t -> unit
    ; stop : unit -> unit
    ; document_controller : Documents.t
    ; run_alerts : Signal_studio_notifications.Run_alerts.t
    }
end

val run
  :  Context.t
  -> env:Eio_unix.Stdenv.base
  -> app:Gpuio_eio.App.t
  -> self_test:bool
  -> workload:bool
  -> unavailable_check:bool
  -> document_path:Gpuio.File_path.t option
  -> bool
