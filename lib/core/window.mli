open Core

(** Native window configuration and observations. Sizes and positions are logical
    pixels. Wayland position is compositor-owned and is not a global coordinate guarantee. *)
module Chrome = Gpuio_protocol.Window_wire.Chrome

module Backend = Gpuio_protocol.Window_wire.Backend
module Capabilities = Gpuio_protocol.Window_wire.Capabilities

(** Snapshot width/height describe the outer native bounds. Content dimensions
    describe the drawable viewport. They may differ by titlebar/decorations.
    [title] retains the application-configured title and successful [Set_title]
    commands; it does not observe external window-manager retitling. *)
module Snapshot = Gpuio_protocol.Window_wire.Snapshot

(** [Resize] requests a content size; asynchronous compositor transitions are
    observed separately. Other commands operate on the current window. *)
module Command = Gpuio_protocol.Window_wire.Command

module Error = Gpuio_protocol.Window_wire.Error

module Config : sig
  type t

  val create
    :  ?focus:bool
    -> ?chrome:Chrome.t
    -> ?resizable:bool
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
