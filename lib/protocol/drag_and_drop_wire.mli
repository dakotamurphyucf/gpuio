open Core

(** Internal data codec. Variant order is part of the binary format. Readers
    validate declared lengths before allocation and reject invalid whole values.
    Writers operate on wire data; application constructors live in Gpuio. *)
module Format : sig
  type t =
    | Text
    | Files
    | Custom of string
  [@@deriving bin_io, equal, compare, sexp_of]
end

module File : sig
  type t =
    { path : string
    ; is_directory : bool option
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Payload : sig
  type t =
    | Text of string
    | Files of File.t list
    | Custom of
        { kind : string
        ; data : string
        }
  [@@deriving bin_io, equal, sexp_of]
end

module Source : sig
  type t =
    { label : string
    ; payload : Payload.t
    ; disabled : bool
    ; allow_desktop_files : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Target : sig
  type t =
    { label : string
    ; accepted_formats : Format.t list
    ; disabled : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end
