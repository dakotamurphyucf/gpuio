open Core

(** Native window configuration and observations. Sizes and positions are logical
    pixels. Wayland position is compositor-owned and is not a global coordinate guarantee. *)
module Chrome = Gpuio_protocol.Window_wire.Chrome

module Backend = Gpuio_protocol.Window_wire.Backend
module Capabilities = Gpuio_protocol.Window_wire.Capabilities

(** Current compositor decoration state, distinct from the requested chrome.
    Tiled edges must not acquire application frame padding or resize handles. *)
module Tiling = Gpuio_protocol.Window_wire.Tiling

module Decorations = Gpuio_protocol.Window_wire.Decorations

(** Live platform controls, intersected with native minimize/resize policy.
    Availability may change while a Wayland window is open. *)
module Controls = Gpuio_protocol.Window_wire.Controls

module Presentation = Gpuio_protocol.Window_wire.Presentation

(** Resolved native window appearance, independent of application color tokens.
    [is_dark] combines ordinary/vibrant variants. Platforms need not produce every
    variant; this is not an observation of unreported high-contrast settings. *)
module Appearance = Gpuio_protocol.Window_wire.Appearance

(** Snapshot width/height describe the outer native bounds. Content dimensions
    describe the drawable viewport. They may differ by titlebar/decorations.
    [title] retains the application-configured title and successful [Set_title]
    commands; it does not observe external window-manager retitling. [document]
    observes native represented-file/edited state where available; [None] means
    the platform cannot provide that observation. [presentation] reports actual
    decorations, tiled edges and supported controls; it is not the requested
    [Chrome] value. [appearance] reports the native window appearance at observation,
    including its initial state. Follow changes to implement an application-owned
    system/light/dark policy; [set_theme] alone does not change OS decorations. *)
module Snapshot = Gpuio_protocol.Window_wire.Snapshot

(** [Resize] requests a content size; asynchronous compositor transitions are
    observed separately. [Minimize] requests native minimization without closing
    the window or cancelling its scope. Its response is the normal state snapshot,
    which has no minimized-state field and does not acknowledge completion of the
    OS transition. Minimize, zoom and fullscreen return [Unsupported] when their
    live platform capability/policy forbids the request. Other commands operate
    on the current window.
    [Set_edited] and [Set_document] return [Unsupported] on the pinned Linux
    backends. Neither operation saves a file or installs an unsaved-work decision. *)
module Command : sig
  type t =
    | Observe
    | Set_title of string
    | Resize of float * float
    | Activate
    | Zoom
    | Toggle_fullscreen
    | Set_edited of bool
    | Set_document of Gpuio_protocol.Window_wire.Document.t
    | Minimize
  [@@deriving equal, sexp_of]

  val validate : t -> unit Or_error.t

  module Expert : sig
    val to_wire : t -> Gpuio_protocol.Window_wire.Command.t
  end
end

module Input : sig
  module Kind = Gpuio_protocol.Window_wire.Input_kind

  (** Metadata-only observation of an eligible native text input. No editable
      value is copied. Identity remains bound to the exact window/node generation. *)
  type t [@@deriving equal, sexp_of]

  val kind : t -> Kind.t
  val same_text_input : t -> Text_input.Snapshot.t -> bool
  val same_otp_input : t -> Otp_input.Snapshot.t -> bool
  val same_number_input : t -> Number_input.Snapshot.t -> bool
  val same_color_input : t -> Color_input.Snapshot.t -> bool

  module Expert : sig
    val of_wire
      :  window:Gpuio_protocol.Window_id.t
      -> Gpuio_protocol.Window_wire.Input.t
      -> t
  end
end

module Error = Gpuio_protocol.Window_wire.Error

module Document : sig
  type t [@@deriving equal, sexp_of]

  (** Native document metadata, separate from application persistence. Omitting
      [path] clears the represented file. Paths preserve native Unix bytes. *)
  val create : ?path:File_path.t -> edited:bool -> unit -> t

  val path : t -> File_path.t option
  val edited : t -> bool
  val of_snapshot : Snapshot.t -> t option

  module Expert : sig
    val to_wire : t -> Gpuio_protocol.Window_wire.Document.t
  end
end

module Config : sig
  type t

  val create
    :  ?focus:bool
    -> ?chrome:Chrome.t
    -> ?resizable:bool
    -> ?frame:Window_frame.t
    -> title:string
    -> width:float
    -> height:float
    -> unit
    -> t Or_error.t

  module Expert : sig
    val to_wire : t -> Gpuio_protocol.Window_wire.Config.t
  end
end

module Close_reason : sig
  type t =
    | Window_close
    | Application_quit
  [@@deriving equal, sexp_of]
end

module Close_decision : sig
  type t =
    | Allow
    | Keep_open
  [@@deriving equal, sexp_of]
end
