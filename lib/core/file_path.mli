open Core

(** An absolute native filesystem path on the supported Unix platforms (macOS
    and Linux). The string contains path bytes, which need not be UTF-8. This
    value is neither a display label nor an Eio filesystem capability. *)
type t [@@deriving equal, compare, sexp_of]

val max_bytes : int

(** Preserve the exact bytes, including repeated separators and dot components.
    Require a leading slash, no NUL, and at most [max_bytes] bytes. Does not access
    the filesystem, resolve aliases/symlinks, normalize, or check existence. *)
val of_string : string -> t Or_error.t

(** The original path bytes. Use an appropriate Eio filesystem capability for
    subsequent I/O. Do not pass this directly to a UI label requiring UTF-8. *)
val to_string : t -> string
