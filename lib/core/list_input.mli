open Core

(** Ordered native list intents, independent of selected state and physical focus.
    Keyed targets are resolved through the current managed-list identities; relative
    intents are reduced against current application state, never a stale cursor. *)
type 'key t =
  | Navigate of List_selection.Navigation.t * List_selection.Gesture.t option
  | Select of 'key * List_selection.Gesture.t
  | Focus of 'key
  | Select_active of List_selection.Gesture.t
  | Confirm of 'key * List_selection.Confirmation.t
  | Confirm_active of List_selection.Confirmation.t
  | Context of 'key
  | Context_active
  | Set_selected of 'key * bool
  | Cancel
[@@deriving sexp_of]

val filter_map : 'a t -> f:('a -> 'b option) -> 'b t option

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** [epoch] identifies application source/query policy. Change it when old
      requests must become obsolete, including replacing a source with reused row
      keys or accepting a new query. It is not a native generation counter.
      Cursor/busy changes preserve pending relative input. Disabled/navigation
      policy, query ownership and epoch changes retire it automatically after
      native acceptance. As with other asynchronous view callbacks, capture the
      application epoch and validate it against current reducer state too: an old
      accepted callback may already be queued while a new render is in flight.

      [cursor] is a logical row key, possibly not mounted yet. [query] names a
      direct sibling single-line input by its View key (normally its controller
      key). Either sibling order is supported; one query may serve only one list.
      The input owns editing/composition; the list receives eligible navigation,
      confirmation, context and cancel intents. No query means root focus only. *)
  val create
    :  epoch:Key.t
    -> ?cursor:Key.t
    -> ?query:Key.t
    -> ?selection_on_navigation:bool
    -> ?disabled:bool
    -> ?busy:bool
    -> unit
    -> t

  val epoch : t -> Key.t
  val cursor : t -> Key.t option
  val query : t -> Key.t option
  val selection_on_navigation : t -> bool
  val disabled : t -> bool
  val busy : t -> bool
end

module Expert : sig
  val same_interaction : Config.t -> Config.t -> bool

  val of_wire
    :  Gpuio_protocol.List_input_wire.Request.t
    -> find_key:(int64 -> 'key option)
    -> 'key t option
end
