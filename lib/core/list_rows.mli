open Core

module Key : sig
  (** Incarnation-safe row identity, separate from reusable application keys.
      Filtering/reorder preserve surviving identities; deletion/reinsertion does
      not, including when the intermediate deletion was never rendered. *)
  type t [@@deriving compare, equal, sexp_of]

  include Comparator.S with type t := t

  val to_view_key : t -> Key.t
end

module Kind : sig
  type t =
    | Option of
        { index : int
        ; count : int
        }
    | Decoration
  [@@deriving equal, sexp_of]
end

module Layout : sig
  (** Metadata only. Reuse until membership/order, query eligibility or decoration
      classification changes; streamed payload changes do not require rebuilding.
      Option positions/counts exclude decorations and include disabled options. *)
  type ('key, 'cmp) t

  (** Decoration keys must be unique loaded members and ineligible when visible.
      This represents section headings/footers without exposing them as options.
      Construction is bounded by loaded metadata, at most one million rows. *)
  val create
    :  ('key, 'cmp) List_selection.Catalog.t
    -> ?decorations:'key list
    -> unit
    -> ('key, 'cmp) t Or_error.t

  val catalog : ('key, 'cmp) t -> ('key, 'cmp) List_selection.Catalog.t
  val kind : ('key, 'cmp) t -> 'key -> Kind.t option
  val option_count : (_, _) t -> int
end

module Item : sig
  (** No selected/cursor state is copied into every row. Compute those properties
      for the bounded mounted subset from List_selection at render time. *)
  type ('key, 'data) t

  val target : ('key, _) t -> 'key List_collection.Item_ref.t
  val key : ('key, _) t -> 'key
  val data : (_, 'data) t -> 'data
  val kind : (_, _) t -> Kind.t
  val is_disabled : (_, _) t -> bool

  (** Decorations retain the renderer's own semantics. Option metadata uses the
      logical option index/count rather than the mounted subset. *)
  val accessibility
    :  (_, _) t
    -> label:string
    -> selected:bool
    -> Accessibility.t option Or_error.t
end

(** Visible payload projection over an application-owned collection. Hidden
    records remain loaded in the original source; projection never removes their
    membership references or selected preferences. It creates no row graphs. *)
type ('key, 'data, 'cmp) t

(** Layout must refer to the source's current Identity snapshot. Point [set]
    updates share that identity; structural edits require a matching layout. *)
val create
  :  ('key, 'data, 'cmp) List_collection.t
  -> layout:('key, 'cmp) Layout.t
  -> ('key, 'data, 'cmp) t Or_error.t

(** With unchanged layout, visit only changed payload entries via persistent-map
    sharing and update affected visible rows. Hidden changes leave the projected
    collection physically unchanged. Layout changes may rebuild visible metadata;
    surviving item keys and the original source identity remain stable. No payload
    comparison is required. Keep one accepted previous projection across updates. *)
val update
  :  ('key, 'data, 'cmp) t
  -> ('key, 'data, 'cmp) List_collection.t
  -> layout:('key, 'cmp) Layout.t
  -> ('key, 'data, 'cmp) t Or_error.t

val source : ('key, 'data, 'cmp) t -> ('key, 'data, 'cmp) List_collection.t
val layout : ('key, _, 'cmp) t -> ('key, 'cmp) Layout.t

val collection
  :  ('key, 'data, _) t
  -> (Key.t, ('key, 'data) Item.t, Key.comparator_witness) List_collection.t

val find : ('key, 'data, _) t -> Key.t -> ('key, 'data) Item.t option

(** Only current visible targets resolve. Hidden, removed/reincarnated and foreign
    references return None; holding a target retains no application payload. *)
val item_key : ('key, _, 'cmp) t -> 'key List_collection.Item_ref.t -> Key.t option
