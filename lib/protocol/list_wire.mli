open Core

val max_logical_rows : int
val max_id_runs : int
val max_active_rows : int

module Id_run : sig
  (** Consecutive positive logical IDs, independent of native node handles. *)
  type t =
    { first : int64
    ; count : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Order : sig
  type t =
    { revision : int64
    ; runs : Id_run.t list
    }
  [@@deriving bin_io, equal, sexp_of]

  (** Validate counts, arithmetic, identity uniqueness and aggregate bounds
      without allocating one entry per logical row. *)
  val validate : t -> unit Or_error.t
end

module Scroll_policy : sig
  type t =
    | Keep_position
    | Follow_tail_when_at_end
  [@@deriving bin_io, equal, sexp_of]
end

module Config : sig
  type t =
    { estimated_height : float
    ; overscan : float
    ; max_active : int64
    ; scroll_policy : Scroll_policy.t
    ; scrollbar : bool
    ; managed : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  val validate : t -> unit Or_error.t
end

module Row : sig
  type t =
    { id : int64
    ; node : Node_id.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Scroll_target : sig
  type t =
    | Offset of int64 * float
    | Reveal of int64
    | End
  [@@deriving bin_io, equal, sexp_of]
end

module Scroll_request : sig
  type t =
    { serial : int64
    ; target : Scroll_target.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Viewport : sig
  type t =
    { order_revision : int64
    ; visible_first : int64
    ; visible_last : int64
    ; requested : int64 list
    ; pinned : int64 list
    ; anchor : (int64 * float) option
    ; following_tail : bool
    ; at_start : bool
    ; at_end : bool
    ; budget_exhausted : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  val validate : t -> unit Or_error.t
end
