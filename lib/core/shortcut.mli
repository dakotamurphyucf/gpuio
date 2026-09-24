open Core

module Modifier : sig
  type t =
    | Primary
    | Control
    | Alt
    | Shift
    | Super
  [@@deriving equal, compare, sexp_of]
end

module Priority : sig
  type t =
    | Native_first
    | Override
  [@@deriving equal, sexp_of]
end

module Text_input : sig
  type t =
    | Modified_only
    | Always
    | Never
  [@@deriving equal, sexp_of]
end

type t [@@deriving equal, sexp_of]

(** A single key chord. [Primary] means Command on macOS and Control on Linux;
    [Super] means Command on macOS and Super on Linux. [key] is one Unicode scalar
    or a named key: enter, escape, tab, space, backspace, delete, insert, home,
    end, pageup, pagedown, left, right, up, down, or f1..f24.
    ASCII letters normalize to lowercase. Duplicate modifiers are rejected.
    Native_first lets native editor/actions handle the chord first. Override
    intercepts it before native actions. Modified_only permits editor shortcuts
    with Primary/Control/Alt/Super; unmodified typing stays with the editor.
    Composition suppresses shortcuts unless explicitly enabled. *)
val create
  :  key:string
  -> ?modifiers:Modifier.t list
  -> ?priority:Priority.t
  -> ?text_input:Text_input.t
  -> ?during_composition:bool
  -> unit
  -> t Or_error.t

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Wire.Shortcut.t
end
