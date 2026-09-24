open Core

(** Application runner. [run] owns GPUI on the OS main thread and starts one
    OCaml UI domain running Eio. Initialization, components and effects run on
    that UI domain. All methods below enforce that ownership. *)
type t

module Stats : sig
  type t =
    { turns : int
    ; clock_ticks : int
    ; commits : int
    ; rendered : int
    ; completed_jobs : int
    }
  [@@deriving sexp_of]
end

module Window : sig
  type t

  val scope : t -> Scope.t
  val close : t -> unit
  val is_closed : t -> bool
  val set_theme : t -> Gpuio.Theme.t -> unit

  (** At most one request per open window. Call after activation; requests
      before native opening return an error. This observes a render callback,
      not physical screen presentation. *)
  val request_frame
    :  t
    -> on_rendered:(revision:int64 -> unit Bonsai.Effect.t)
    -> unit Or_error.t

  module Expert : sig
    (** Correlated native picker, scoped to this exact window generation. Only
        one request may be pending per window. Closing returns [Closed]. *)
    val file_dialog
      :  t
      -> Gpuio.File_dialog.Request.t
      -> (Gpuio.File_path.t list option, Gpuio.File_dialog.Error.t) Result.t
           Bonsai.Effect.t

    val file_dialog_capabilities
      :  t
      -> (Gpuio.File_dialog.Capabilities.t, Gpuio.File_dialog.Error.t) Result.t
           Bonsai.Effect.t

    (** Correlated native commands for controller adapters. Captures the exact
        editor lease; a delayed effect never targets a remounted replacement. *)
    val editor_command
      :  t
      -> Gpuio.Text_input.Snapshot.t
      -> Gpuio.Text_input.Command.t
      -> (Gpuio.Text_input.Snapshot.t, Gpuio.Text_input.Command_error.t) Result.t
           Bonsai.Effect.t
  end
end

module Expert : sig
  (** Correlated raw registration protocol, bounded to 63 pending requests;
      a separate lane is reserved for scoped upload/cleanup.
      This is not the scoped public asset API: callers own release/late-reply
      cleanup. A completed upload contains encoded data, not decoded pixels. *)
  val asset
    :  t
    -> Gpuio_protocol.Wire.Asset.Request.t
    -> Gpuio_protocol.Wire.Asset.Response.t Bonsai.Effect.t

  val register_asset
    :  t
    -> scope:Scope.t
    -> Gpuio.Asset.Source.t
    -> (Asset_registry.Registration.t, Asset_registry.Error.t) Result.t Bonsai.Effect.t
end

val scope : t -> Scope.t
val stats : t -> Stats.t
val shutdown : t -> unit

(** Application-wide native motion policy. [System] follows available platform
    preferences and defaults to full motion when no preference is available.
    [Reduce] and [Full] override the platform until [System] is selected again.
    This also controls indeterminate progress. Changes are asynchronous and
    coalesced before submission; no per-frame OCaml callbacks are introduced.
    Calls after shutdown are ignored. *)
val set_motion : t -> Gpuio.Animation.Preference.t -> unit

(** Up to 32 simultaneously live windows; closed slots are reused with new
    generations. Width/height are logical pixels in [1,16384]. Titles contain
    1..4096 UTF-8 bytes without NUL. The component factory is invoked on the UI domain. *)
val open_window
  :  t
  -> ?theme:Gpuio.Theme.t
  -> title:string
  -> width:float
  -> height:float
  -> (Window.t -> unit Bonsai.Effect.t Gpuio.View.t Bonsai.Computation.t)
  -> Window.t Or_error.t

(** Defaults: 60 Hz shared monotonic timer, 1024 tasks, exit on last window,
    [System] motion preference. The initial motion policy precedes opening windows.
    With [exit_on_last_window=false], application/conversation work may continue
    with no windows; call [shutdown] to finish. Parameter validation raises.
    [tick_hz] is in [0.01,240]; [max_tasks] is in [1,65536].
    Application/task callback failures propagate after native and Eio cleanup. *)
val run
  :  ?tick_hz:float
  -> ?max_tasks:int
  -> ?exit_on_last_window:bool
  -> ?motion:Gpuio.Animation.Preference.t
  -> (Eio_unix.Stdenv.base -> t -> unit)
  -> unit
