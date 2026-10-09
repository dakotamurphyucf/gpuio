open Core

(** Plain-text clipboard payloads. Native writes belong to [Gpuio_eio.Clipboard];
    construction does not read or change the system clipboard. *)
module Text : sig
  type t [@@deriving equal, sexp_of]

  (** At most 256 KiB of valid UTF-8 without NUL. Empty text is allowed. Newlines,
      tabs and Unicode are preserved; no normalization or format sniffing occurs. *)
  val of_string : string -> t Or_error.t

  val to_string : t -> string
  val max_bytes : int
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
    | Native_failure
  [@@deriving equal, sexp_of]
end
