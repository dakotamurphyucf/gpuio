open Core

(** One native single/range slider placement. Observations never reset its values.
    Use [view] once in the window tree; duplicate placement is rejected. *)
type t

val create
  :  App.Window.t
  -> config:Gpuio.Slider.Config.t Bonsai.Cont.t
  -> initial:Gpuio.Slider.Value.t
  -> ?on_event:(Gpuio.Slider.Event.t -> unit Bonsai.Effect.t) Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> t Bonsai.Cont.t

val view
  :  ?style:Gpuio.Style.t
  -> ?appearance:Gpuio.Slider.Appearance.t
  -> t
  -> Gpuio_bonsai.View.t

(** Last native observation, absent before mounting. A stored controller may
    outlive its placement; commands then fail with [Stale_slider]. *)
val snapshot : t -> Gpuio.Slider.Snapshot.t option

val command
  :  t
  -> Gpuio.Slider.Command.t
  -> (Gpuio.Slider.Snapshot.t, Gpuio.Slider.Command_error.t) Result.t Bonsai.Effect.t

(** Read current state without mutation or focus, including hidden/disabled sliders. *)
val read_snapshot
  :  t
  -> (Gpuio.Slider.Snapshot.t, Gpuio.Slider.Command_error.t) Result.t Bonsai.Effect.t

(** Focus a specific thumb, subject to native visibility/modal/disabled policy. *)
val focus
  :  t
  -> Gpuio.Slider.Thumb.t
  -> (Gpuio.Slider.Snapshot.t, Gpuio.Slider.Command_error.t) Result.t Bonsai.Effect.t

val cancel_drag
  :  t
  -> (Gpuio.Slider.Snapshot.t, Gpuio.Slider.Command_error.t) Result.t Bonsai.Effect.t

(** Normalize and replace, preserving single/range mode. Cancels any active drag.
    Permitted while hidden/disabled/read-only. A stale guard fails atomically. *)
val replace
  :  t
  -> ?if_revision:Gpuio.Slider.Revision.t
  -> Gpuio.Slider.Value.t
  -> (Gpuio.Slider.Snapshot.t, Gpuio.Slider.Command_error.t) Result.t Bonsai.Effect.t

(** Bind both lease and revision to [expected], even if this controller has since
    observed another native placement. *)
val replace_if_unchanged
  :  t
  -> Gpuio.Slider.Snapshot.t
  -> Gpuio.Slider.Value.t
  -> (Gpuio.Slider.Snapshot.t, Gpuio.Slider.Command_error.t) Result.t Bonsai.Effect.t
