open Core

(** Private image wire types. Variant/field order is part of the protocol. *)
module Fit : sig
  type t =
    | Fill
    | Contain
    | Cover
    | Scale_down
    | None
  [@@deriving bin_io, equal, sexp_of]
end

module Error : sig
  type t =
    | Wrong_application
    | Released
    | Invalid_data
    | Unsupported
    | Resource_limit
    | Native_failure
  [@@deriving bin_io, equal, sexp_of]
end

module Source : sig
  type t =
    | Reference of Resource_id.t
    | Unavailable of Error.t
  [@@deriving bin_io, equal, sexp_of]
end

module Config : sig
  type t =
    { source : Source.t
    ; fit : Fit.t
    ; label : string option
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Metadata : sig
  type t =
    { width_px : int64
    ; height_px : int64
    ; frames : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module State : sig
  type t =
    | Loading
    | Ready of Metadata.t
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]
end
