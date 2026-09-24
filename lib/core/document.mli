open Core

module Language : sig
  (** A syntax token/extension, not an executable grammar or file path.
      Unknown languages render as plain text. *)
  type t [@@deriving equal, sexp_of]

  val plain_text : t
  val ocaml : t
  val rust : t
  val shell : t
  val json : t
  val python : t
  val javascript : t
  val typescript : t
  val yaml : t
  val toml : t
  val ini : t
  val of_string : string -> t Or_error.t
  val to_string : t -> string
end

module Mode : sig
  type t =
    | Markdown
    | Code of Language.t
    | Diff
  [@@deriving equal, sexp_of]
end

module Appearance : sig
  type t =
    | Light
    | Dark
  [@@deriving equal, sexp_of]
end

module Layout : sig
  type t =
    | Flow
    | Viewport of float
  [@@deriving equal, sexp_of]
end

module Navigation : sig
  module Side : sig
    type t =
      | Before
      | After
    [@@deriving equal, sexp_of]
  end

  type t =
    | Link of string
    | Line of
        { path : string option
        ; side : Side.t
        ; line : int
        }
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** [source] is a borrowed registered document. [Flow] fits bounded content
      into its parent; [Viewport height] owns a native vertical scroll viewport
      of 1..16384 logical pixels. Code/diff scroll horizontally without wrapping.
      Huge Markdown uses the bounded source presentation described in the
      document contract. [initially_collapsed] applies on mount/generation reset.
      [path] labels navigation; it never reads a file. [search] is literal text.

      Markdown image URLs resolve only through [images], an explicit mapping to
      application asset handles; no implicit network or filesystem acquisition.
      At most128 unique URLs, 4096 UTF-8 bytes each. *)
  val create
    :  source:Text_source.Handle.t
    -> mode:Mode.t
    -> ?appearance:Appearance.t
    -> ?layout:Layout.t
    -> ?label:string
    -> ?path:string
    -> ?line_numbers:bool
    -> ?initially_collapsed:bool
    -> ?search:string
    -> ?images:(string * Asset.Handle.t) list
    -> unit
    -> t Or_error.t

  val source : t -> Text_source.Handle.t
  val mode : t -> Mode.t
  val appearance : t -> Appearance.t
  val layout : t -> Layout.t
  val label : t -> string
  val path : t -> string option
  val line_numbers : t -> bool
  val initially_collapsed : t -> bool
  val search : t -> string
  val images : t -> (string * Asset.Handle.t) list
end

module Expert : sig
  val to_wire
    :  Config.t
    -> owner:Text_source.Expert.Owner.t option
    -> asset_owner:Asset.Expert.Owner.t option
    -> Gpuio_protocol.Document_wire.Config.t

  val navigation : Gpuio_protocol.Document_wire.Navigation.t -> Navigation.t Or_error.t
end
