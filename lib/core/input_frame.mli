open Core

(** Appearance and auxiliary controls around one retained native input. Prefix
    and suffix content are ordinary views supplied to [View.input_frame]. *)
type t [@@deriving equal, sexp_of]

(** [clear_label] enables a native clear button for a single-line input. Clear
    requires editable, nonempty text and no active composition; loading suppresses
    it. Clearing records undo and returns focus to this field. Labels must be
    nonblank UTF-8 without NUL, at most 4096 bytes. [gap] is in logical pixels,
    defaults to 6 and accepts finite values in 0..256.

    Loading shows a native spinner and busy state without blocking typing.
    Reveal is opt-in via [View.input_frame]'s callback; these labels follow the
    existing password privacy config. Applications own reveal state. *)
val create
  :  ?clear_label:string
  -> ?loading:bool
  -> ?loading_label:string
  -> ?show_password_label:string
  -> ?hide_password_label:string
  -> ?gap:float
  -> unit
  -> t Or_error.t

val default : t

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Editor_frame_wire.t
  val is_loading : t -> bool
  val loading_label : t -> string
  val reveal_label : t -> revealed:bool -> string
end
