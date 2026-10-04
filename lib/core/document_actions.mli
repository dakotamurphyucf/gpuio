open Core

(** Custom native buttons for rich Markdown/HTML code blocks and tables.
    Callbacks are queued through [View.document ~on_action]; no code is executed. *)
module Id : sig
  type t [@@deriving equal, compare, sexp_of]

  (** 1..64 ASCII bytes: initial letter, then letters, digits, underscore, dot or dash. *)
  val of_string : string -> t Or_error.t

  val to_string : t -> string
end

module Action : sig
  type t [@@deriving equal, sexp_of]

  (** Nonempty accessible [label], at most256 UTF-8 bytes and no NUL. *)
  val create : id:Id.t -> label:string -> ?enabled:bool -> unit -> t Or_error.t

  val id : t -> Id.t
  val label : t -> string
  val enabled : t -> bool
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** At most16 unique IDs per family. Native Copy buttons default to visible.
      Nonempty custom lists require [View.document ~on_action] or a handler
      supplied by [Document.Defaults]. Config changes
      retain source, parser, selection and scroll ownership. *)
  val create
    :  ?code:Action.t list
    -> ?table:Action.t list
    -> ?copy_code:bool
    -> ?copy_table:bool
    -> unit
    -> t Or_error.t

  val default : t
  val code : t -> Action.t list
  val table : t -> Action.t list
  val copy_code : t -> bool
  val copy_table : t -> bool
end

module Source_range : sig
  (** Half-open UTF-8 bytes of the complete source block, including code fences.
      This need not equal the code payload range. HTML omits this. *)
  type t = private
    { start_byte : int
    ; end_byte : int
    }
  [@@deriving equal, sexp_of]
end

module Block : sig
  type t = private
    | Code of
        { language : string option
        ; code : string
        }
    | Table of
        { headers : string list
        ; rows : string list list
        ; markdown : string
        }
  [@@deriving equal, sexp_of]
end

module Event : sig
  (** Snapshot refers to the installed picture, possibly before a pending append.
      A queued accepted event remains self-contained after subsequent publication;
      check provenance before applying its range to a newer document. *)
  type t = private
    { action : Id.t
    ; source_revision : int64
    ; source_generation : int64
    ; source_range : Source_range.t option
    ; block : Block.t
    ; activation : Document_activation.t
    }
  [@@deriving equal, sexp_of]
end

module Expert : sig
  val to_wire
    :  Config.t
    -> epoch:int64
    -> observe:bool
    -> Gpuio_protocol.Document_actions_wire.Config.t

  val event_of_wire
    :  config:Config.t
    -> Gpuio_protocol.Document_actions_wire.Event.t
    -> Event.t Or_error.t
end
