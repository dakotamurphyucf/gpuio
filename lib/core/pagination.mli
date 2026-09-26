open Core

module Request : sig
  type t = private
    | Page of int
    | First
    | Previous
    | Next
    | Last
  [@@deriving equal, sexp_of]

  val page : int -> t Or_error.t
  val first : t
  val previous : t
  val next : t
  val last : t
end

module Item : sig
  type t =
    | Page of int
    | Gap of
        { first : int
        ; last : int
        }
  [@@deriving equal, sexp_of]
end

type t [@@deriving equal, sexp_of]

val max_pages : int

(** One-based pages, total 0..[max_pages] (1,000,000,000). Empty datasets have
    no selection. An explicit [current] must exist; otherwise select the first
    page when nonempty. [siblings] (0..4, default 1) determines the neighborhood
    on either side of the current page. Disabled state suppresses user requests,
    not programmatic changes. This model never loads data or schedules work. *)
val create
  :  total_pages:int
  -> ?current:int
  -> ?siblings:int
  -> ?disabled:bool
  -> unit
  -> t Or_error.t

val total_pages : t -> int
val current : t -> int option
val siblings : t -> int
val is_disabled : t -> bool
val with_disabled : t -> bool -> t

(** Clamp to the new last page on shrink; clear on zero; choose page 1 on growth
    from zero. Reject invalid counts. *)
val with_total_pages : t -> int -> t Or_error.t

(** Explicit selection rejects nonexistent pages, including an empty dataset. *)
val select : t -> page:int -> t Or_error.t

(** Apply against the latest model in the application's reducer, in delivery
    order. Boundary steps, stale absolute pages and all disabled requests are
    no-ops. Relative steps do not use a stale rendered current page. *)
val apply_request : t -> Request.t -> t

(** Endpoints, current neighborhood and inclusive gaps; a one-page gap is a
    Page. At most 13 items even for [max_pages], O([siblings]) time/storage. *)
val items : t -> Item.t list
