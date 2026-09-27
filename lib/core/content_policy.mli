open Core

(** Native content lifetime while its panel is inactive. This does not determine
    Bonsai computation or application task lifetime; use explicit Bonsai branching
    and application Eio scopes for those. *)
type t =
  | Retain
  (** Keep keyed native children, editor buffers and list state mounted. Hidden
      children cannot receive keyboard, pointer, accessibility or IME input. *)
  | Unmount
  (** Remove native children while hidden; showing them creates new native leases.
      The enclosing panel and its disclosure trigger retain their identity. *)
[@@deriving equal, sexp_of]
