open Core

(** Pure native file-dialog configuration. Selecting a path does not open, create
    or write a file; application filesystem I/O uses Eio. Runtime presentation
    and cancellation are separate adapter concerns; constructors do neither. *)
module Open : sig
  module Selection : sig
    type t =
      | Files
      | Directories
      | Files_and_directories
    [@@deriving equal, sexp_of]
  end

  type t [@@deriving equal, sexp_of]

  (** Defaults: files, single selection, title "Open", accept label "Open".
      Mixed selection requires an explicit platform capability; the adapter must
      reject an unsupported mode instead of silently changing it. A directory is
      an initial navigation hint and is not checked for existence. Labels must be
      nonblank UTF-8 without NUL and at most 4096 bytes. *)
  val create
    :  ?selection:Selection.t
    -> ?multiple:bool
    -> ?title:string
    -> ?accept_label:string
    -> ?directory:File_path.t
    -> unit
    -> t Or_error.t

  val selection : t -> Selection.t
  val multiple : t -> bool
  val title : t -> string
  val accept_label : t -> string
  val directory : t -> File_path.t option
end

module Save : sig
  type t [@@deriving equal, sexp_of]

  (** Defaults: title "Save", accept label "Save". [suggested_name] is a single
      UTF-8 filename, 1..255 bytes, without slash or NUL, other than "." or "..".
      The initial directory is not checked for existence. The chosen result must
      preserve the OS-selected filename without extension rewriting. Native
      overwrite confirmation does not itself write or reserve the destination. *)
  val create
    :  directory:File_path.t
    -> suggested_name:string
    -> ?title:string
    -> ?accept_label:string
    -> unit
    -> t Or_error.t

  val directory : t -> File_path.t
  val suggested_name : t -> string
  val title : t -> string
  val accept_label : t -> string
end
