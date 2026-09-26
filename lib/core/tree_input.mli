open Core

(** Native tree intents after resolving monotonic managed-list IDs to collection
    keys. Relative requests preserve delivery order. These do not change state
    or assert OS focus; adapters reduce them against the current source and state. *)
type 'key t =
  | Navigate of Tree_state.Navigation.t * Tree_state.Selection.t option
  | Select of 'key * Tree_state.Selection.t
  | Focus of 'key
  | Set_expanded of 'key * bool
  | Activate of 'key
  | Select_active of Tree_state.Selection.t
  | Activate_active
  | Typeahead of Tree_typeahead.Input.t
  | Set_selected of 'key * bool
[@@deriving sexp_of]

(** None discards an obsolete target. Relative requests need no key lookup. *)
val filter_map : 'a t -> f:('a -> 'b option) -> 'b t option

module Expert : sig
  val of_wire
    :  Gpuio_protocol.Tree_input_wire.Request.t
    -> find_key:(int64 -> 'key option)
    -> 'key t option
end
