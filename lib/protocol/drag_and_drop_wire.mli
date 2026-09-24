open Core

exception Invalid_wire_data

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

module Origin : sig
  type t =
    | Internal
    | Desktop
  [@@deriving bin_io, equal, sexp_of]
end

module Offer : sig
  type t =
    { format : Format.t
    ; data_bytes : int64
    ; file_count : int64
    ; origin : Origin.t
    }
  [@@deriving bin_io, equal, sexp_of]

  val is_valid : t -> bool
end

module Cancel_reason : sig
  type t =
    | Escape
    | Hidden
    | Blocked
    | Disabled
    | Removed
    | Reconfigured
    | Window_closed
    | Window_inactive
  [@@deriving bin_io, equal, sexp_of]
end

module Outcome : sig
  type t =
    | Internal_drop
    | Cancelled of Cancel_reason.t
    | Unconfirmed
  [@@deriving bin_io, equal, sexp_of]
end

module Source_phase : sig
  type t =
    | Started of Payload.t
    | Desktop_offered
    | Desktop_unavailable
    | Ended of Outcome.t
  [@@deriving bin_io, equal, sexp_of]
end

module Source_sample : sig
  type t =
    { gesture : int64
    ; phase : Source_phase.t
    }
  [@@deriving bin_io, equal, sexp_of]

  val is_valid : t -> bool
end

module Rejection : sig
  type t =
    | Invalid_data
    | Limit_exceeded
  [@@deriving bin_io, equal, sexp_of]
end

module Target_phase : sig
  type t =
    | Entered of Offer.t
    | Moved
    | Left
    | Dropped of Payload.t
    | Rejected of Rejection.t
  [@@deriving bin_io, equal, sexp_of]
end

module Modifiers : sig
  type t =
    { shift : bool
    ; control : bool
    ; alt : bool
    ; command : bool
    ; function_ : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Target_sample : sig
  type t =
    { gesture : int64
    ; phase : Target_phase.t
    ; window_x : float
    ; window_y : float
    ; local_x : float
    ; local_y : float
    ; modifiers : Modifiers.t
    }
  [@@deriving bin_io, equal, sexp_of]

  val is_valid : t -> bool
end
