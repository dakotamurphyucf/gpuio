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

  (** Force-close; bypasses the application close decision and cancels the
      window scope. Use [request_close] for ordinary user commands. *)
  val close : t -> unit

  (** Coalesced asynchronous decision. Force-close invalidates a delayed answer.
      Window work remains live while a decision is pending. Default is Allow. *)
  val request_close : t -> unit

  val set_close_handler
    :  t
    -> (Gpuio.Window.Close_reason.t -> Gpuio.Window.Close_decision.t Bonsai.Effect.t)
    -> unit

  val snapshot : t -> Gpuio.Window.Snapshot.t option
  val on_change : t -> (Gpuio.Window.Snapshot.t -> unit Bonsai.Effect.t) -> unit

  (** Acknowledges current observed state. Resizing/fullscreen may complete later;
      listen to [on_change]. Requests use the exact window generation. *)
  val command
    :  t
    -> Gpuio.Window.Command.t
    -> (Gpuio.Window.Snapshot.t, Gpuio.Window.Error.t) Result.t Bonsai.Effect.t

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

    (** Correlated slider commands bound to the exact observed window/node lease. *)
    val slider_command
      :  t
      -> Gpuio.Slider.Snapshot.t
      -> Gpuio.Slider.Command.t
      -> (Gpuio.Slider.Snapshot.t, Gpuio.Slider.Command_error.t) Result.t Bonsai.Effect.t

    (** Correlated OTP commands for the exact observed window/node lease. At most
        64 requests may be pending across the app. Closing rejects new requests;
        already-admitted commands can reply before native closure. Actual closure
        completes remaining requests with [Closed].
        Applied replies must preserve policy and not precede the observed revision. *)
    val otp_input_command
      :  t
      -> Gpuio.Otp_input.Snapshot.t
      -> Gpuio.Otp_input.Command.t
      -> (Gpuio.Otp_input.Snapshot.t, Gpuio.Otp_input.Command_error.t) Result.t
           Bonsai.Effect.t

    (** Correlated calendar commands for the exact observed window/node lease.
        At most 64 calendar requests may be pending across the app. Replies must
        preserve mode and not precede the observed revision. Unsupported dates or
        navigation offsets fail with [Invalid_value] before entering the queue.
        Actual window closure completes remaining requests with [Closed]. *)
    val calendar_command
      :  t
      -> Gpuio.Calendar.Snapshot.t
      -> Gpuio.Calendar.Command.t
      -> (Gpuio.Calendar.Snapshot.t, Gpuio.Calendar.Command_error.t) Result.t
           Bonsai.Effect.t

    (** Correlated color commands for the observed window/node lease. At most 64
        color requests may be pending across the app. Replies cannot precede the
        observed revision. Closing completes remaining requests with [Closed]. *)
    val color_input_command
      :  t
      -> Gpuio.Color_input.Snapshot.t
      -> Gpuio.Color_input.Command.t
      -> (Gpuio.Color_input.Snapshot.t, Gpuio.Color_input.Command_error.t) Result.t
           Bonsai.Effect.t

    (** Correlated numeric commands bound to the observed window/node lease;
        at most 64 requests pending. Closing completes them with [Closed]. *)
    val number_input_command
      :  t
      -> Gpuio.Number_input.Snapshot.t
      -> Gpuio.Number_input.Command.t
      -> (Gpuio.Number_input.Snapshot.t, Gpuio.Number_input.Command_error.t) Result.t
           Bonsai.Effect.t
  end
end

module Expert : sig
  (** Raw correlated scene-resource protocol; at most 63 pending requests.
      Callers own release and late-reply cleanup. This registers scene data;
      it does not itself create a rendered canvas. One additional request lane
      is reserved for the scoped adapter. *)
  val canvas
    :  t
    -> Gpuio_protocol.Wire.Canvas.Request.t
    -> Gpuio_protocol.Wire.Canvas.Response.t Bonsai.Effect.t

  (** Internal correlated document-resource lane. Bounded to 64 requests.
      Public applications use the scoped document adapter. *)
  val document
    :  t
    -> Gpuio_protocol.Wire.Document.Request.t
    -> Gpuio_protocol.Wire.Document.Response.t Bonsai.Effect.t

  val register_canvas
    :  t
    -> scope:Scope.t
    -> Gpuio.Canvas_scene.t
    -> (Canvas_registry.Registration.t, Canvas_registry.Error.t) Result.t Bonsai.Effect.t

  val register_document
    :  t
    -> scope:Scope.t
    -> Gpuio.Text_source.t
    -> (Document_registry.Registration.t, Gpuio_protocol.Wire.Document.Error.t) Result.t
         Bonsai.Effect.t

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

module Diagnostics : sig
  (** Read-only UI-domain snapshot. Registry bytes are conservative OCaml-side
      reservations for encoded assets, canonical source snapshots and scenes;
      they exclude native decoded caches, GPU allocations and model payloads.
      Pending requests include correlated lifecycle, input, resource and frame
      requests; queued commands have not yet been accepted by the native host.
      Sampling creates no bridge command and does not request a frame. *)
  type t =
    { runtime : Stats.t
    ; traffic : Gpuio_native.Traffic.t
    ; scopes : Scope.Stats.t
    ; windows : int
    ; queued_jobs : int
    ; queued_commands : int
    ; pending_requests : int
    ; assets : int
    ; asset_uploads : int
    ; asset_source_bytes : int
    ; documents : int
    ; document_source_bytes : int
    ; canvases : int
    ; canvas_scene_bytes : int
    }
  [@@deriving sexp_of]
end

val diagnostics : t -> Diagnostics.t

(** Force application cleanup, bypassing decisions. *)
val shutdown : t -> unit

(** Ask all live windows before destroying any of them. A denial keeps the
    application open. New windows are rejected during a pending quit decision. *)
val request_quit : t -> unit

val window_capabilities : t -> Gpuio.Window.Capabilities.t option
val on_reopen : t -> (unit -> unit Bonsai.Effect.t) -> unit

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
  -> ?focus:bool
  -> ?chrome:Gpuio.Window.Chrome.t
  -> ?resizable:bool
  -> title:string
  -> width:float
  -> height:float
  -> (Window.t -> unit Bonsai.Effect.t Gpuio.View.t Bonsai.Computation.t)
  -> Window.t Or_error.t

val open_window_config
  :  t
  -> ?theme:Gpuio.Theme.t
  -> Gpuio.Window.Config.t
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

(** Read the linked component schemas on the OS main thread before [run].
    This initializes the selected backend and freezes native registration.
    Compare with package definitions before constructing application windows. *)
val extension_catalog : unit -> Gpuio.Extension.Schema.t list Or_error.t
