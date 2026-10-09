open Core

(** Checked rich-header descriptions for [View.Expert.managed_table] and the
    Bonsai table constructors. Rust renders submitted Views inside native leaf
    or group cells; no renderer callback crosses into OCaml. *)
module Target : sig
  type t [@@deriving compare, equal, sexp_of]

  val column : Table_column.Id.t -> t

  (** Zero-based group level 0..3 and 1..64 distinct column IDs. Membership is a
      set, independent of display order. The complete set must match one group
      at that level in the submitted schema; labels are not identities. *)
  val group : level:int -> columns:Table_column.Id.t list -> t Or_error.t

  val matches : t -> Table_column.Collection.t -> bool
end

type 'view t

(** [key] is stable content identity, separate from its current target. *)
val create : key:Key.t -> target:Target.t -> 'view -> 'view t

val key : _ t -> Key.t
val target : _ t -> Target.t
val content : 'view t -> 'view

(** At most 320 entries, with distinct keys/targets and exact schema membership.
    This bounds header slots, not arbitrary content or the normal View budgets. *)
val validate_all : _ t list -> columns:Table_column.Collection.t -> unit Or_error.t

module Expert : sig
  val target_to_wire : Target.t -> Gpuio_protocol.Table_header_wire.t
  val target_of_wire : Gpuio_protocol.Table_header_wire.t -> Target.t Or_error.t
end
