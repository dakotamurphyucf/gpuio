open Core

(** Pure OS notification values. Native operations belong to the Eio adapter;
    these types are separate from in-app toasts. *)
module Tag : sig
  type t [@@deriving equal, compare, sexp_of]

  (** Nonblank UTF-8, at most 128 bytes, without ASCII control characters. A tag
      identifies one live logical notification in an application. *)
  val of_string : string -> t Or_error.t

  val to_string : t -> string
end

module Action_id : sig
  type t [@@deriving equal, compare, sexp_of]

  (** 1..64 ASCII letters/digits/dot/underscore/hyphen. *)
  val of_string : string -> t Or_error.t

  val to_string : t -> string
end

module Action : sig
  type t [@@deriving equal, sexp_of]

  (** Nonblank UTF-8 label up to 128 bytes, without ASCII controls. *)
  val create : Action_id.t -> label:string -> t Or_error.t

  val id : t -> Action_id.t
  val label : t -> string
end

module Sound : sig
  type t =
    | Silent
    | Default
  [@@deriving equal, sexp_of]
end

type t [@@deriving equal, sexp_of]

(** Nonblank UTF-8 title up to 256 bytes, body up to 8192 bytes. ASCII controls
    are excluded except body newline, carriage return and tab. Content is plain
    text, not markup. Up to four uniquely identified actions; OS presentation
    policy may expose fewer. Default is [Silent], empty body and no actions. *)
val create
  :  title:string
  -> ?body:string
  -> ?actions:Action.t list
  -> ?sound:Sound.t
  -> unit
  -> t Or_error.t

val title : t -> string
val body : t -> string
val actions : t -> Action.t list
val sound : t -> Sound.t

module Receipt : sig
  (** Opaque, process-local notification lifetime. Content replacement retains
      this receipt and logical tag. After dismissal or terminal OS action, posting
      the same tag creates a different receipt. Never persist it across runs or
      identify a native window by its tag. *)
  type t [@@deriving equal, compare, sexp_of]

  val tag : t -> Tag.t
end

module Authorization : sig
  type t =
    | Not_determined
    | Denied
    | Authorized
    | Provisional
    | Not_required
  [@@deriving equal, sexp_of]
end

module Capabilities : sig
  (** Snapshot of backend/server support, not proof of presentation or permission.
      [activation] is a default click on the logical notification; it does not
      identify a content revision or automatically activate a window. *)
  type t =
    { body : bool
    ; actions : bool
    ; activation : bool
    ; replacement : bool
    ; dismissal : bool
    ; permission_request : bool
    ; sound : bool
    }
  [@@deriving equal, sexp_of]
end

module Error : sig
  type t =
    | Invalid_request
    | Not_ready
    | Unsupported
    | Unavailable
    | Denied
    | Busy
    | Closed
    | Stale
    | Native_failure
  [@@deriving equal, sexp_of]
end

module Closed_reason : sig
  type t =
    | Expired
    | User
    | Platform
  [@@deriving equal, sexp_of]
end

module Event : sig
  (** At most one terminal OS event per receipt. Already-admitted events retain
      their original receipt if a later notification reuses the tag. Applications
      route using their current model and exact [App.Window.t] handles; no raw
      window slot is embedded in OS notifications. Explicit API dismissal is
      reported by its result, not a duplicate [Closed] event. *)
  type t =
    | Activated of Receipt.t
    | Action of Receipt.t * Action_id.t
    | Closed of Receipt.t * Closed_reason.t
    | Failed of Error.t
  [@@deriving equal, sexp_of]
end

module Expert : sig
  val error_of_wire : Gpuio_protocol.Notification_wire.Error.t -> Error.t

  val authorization_of_wire
    :  Gpuio_protocol.Notification_wire.Authorization.t
    -> Authorization.t

  val capabilities_of_wire
    :  Gpuio_protocol.Notification_wire.Capabilities.t
    -> Capabilities.t

  val to_wire : t -> Gpuio_protocol.Notification_wire.Content.t
  val of_wire : Gpuio_protocol.Notification_wire.Content.t -> t Or_error.t
  val receipt_to_wire : Receipt.t -> Gpuio_protocol.Notification_wire.Receipt.t
  val receipt_of_wire : Gpuio_protocol.Notification_wire.Receipt.t -> Receipt.t Or_error.t
  val event_of_wire : Gpuio_protocol.Notification_wire.Event.t -> Event.t Or_error.t
end
