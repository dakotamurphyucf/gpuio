(** A native searchable command chooser. Application commands and callbacks
    remain in the enclosing [Command.Registry]. *)
module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Commands are unique, ordered references to enclosing registries; at most
      1024 IDs and 256 KiB total metadata. Labels/placeholder are UTF-8 without
      NUL, at most 4096 bytes; the label is nonblank. Native search matches all
      whitespace-separated query terms against Unicode-lowercase label and ID,
      preserving declared order. Query text is native-owned and limited to
      4096 bytes. It is independent of application document editors. *)
  val create
    :  label:string
    -> commands:Command.Id.t list
    -> ?placeholder:string
    -> ?dismiss_on_outside_pointer:bool
    -> unit
    -> t Core.Or_error.t

  val commands : t -> Command.Id.t list
end

module Dismissal : sig
  type t =
    | Escape
    | Outside_pointer
    | Selected of Command.Id.t
  [@@deriving equal, sexp_of]
end

module Appearance = Choice.Appearance

module Expert : sig
  val to_wire : Config.t -> Gpuio_protocol.Wire.Palette.t

  val dismissal
    :  Config.t
    -> Gpuio_protocol.Wire.Palette_dismissal.t
    -> Dismissal.t option
end
