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
  | Move of
      { source : 'key
      ; destination : 'key
      ; placement : Tree_interaction.Placement.t
      }
[@@deriving sexp_of]

(** None discards an obsolete target. Move requires both endpoints to resolve.
    Relative requests need no key lookup. A move is a proposal, never a mutation. *)
val filter_map : 'a t -> f:('a -> 'b option) -> 'b t option

module Expert : sig
  val of_wire
    :  Gpuio_protocol.Tree_input_wire.Request.t
    -> find_key:(int64 -> 'key option)
    -> 'key t option
end
