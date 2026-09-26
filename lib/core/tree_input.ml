open Core
module W = Gpuio_protocol.Tree_input_wire

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

let filter_map t ~f =
  match t with
  | Navigate (direction, selection) -> Some (Navigate (direction, selection))
  | Select (key, selection) -> Option.map (f key) ~f:(fun key -> Select (key, selection))
  | Focus key -> Option.map (f key) ~f:(fun key -> Focus key)
  | Set_expanded (key, expanded) ->
    Option.map (f key) ~f:(fun key -> Set_expanded (key, expanded))
  | Activate key -> Option.map (f key) ~f:(fun key -> Activate key)
  | Select_active selection -> Some (Select_active selection)
  | Activate_active -> Some Activate_active
  | Typeahead input -> Some (Typeahead input)
  | Set_selected (key, selected) ->
    Option.map (f key) ~f:(fun key -> Set_selected (key, selected))
;;

module Expert = struct
  let selection = function
    | W.Selection.Replace -> Tree_state.Selection.Replace
    | Toggle -> Toggle
    | Range { extend } -> Range { extend }
  ;;

  let navigation = function
    | W.Navigation.Previous -> Tree_state.Navigation.Previous
    | Next -> Next
    | First -> First
    | Last -> Last
    | Parent -> Parent
    | Child -> Child
  ;;

  let of_wire request ~find_key =
    if not (W.Request.valid request)
    then None
    else (
      let input =
        match request with
        | W.Request.Navigate (direction, gesture) ->
          Some (Navigate (navigation direction, Option.map gesture ~f:selection))
        | Select (key, gesture) -> Some (Select (key, selection gesture))
        | Focus key -> Some (Focus key)
        | Set_expanded (key, expanded) -> Some (Set_expanded (key, expanded))
        | Activate key -> Some (Activate key)
        | Select_active gesture -> Some (Select_active (selection gesture))
        | Activate_active -> Some Activate_active
        | Set_selected (key, selected) -> Some (Set_selected (key, selected))
        | Typeahead { text; reset; cycle } ->
          Tree_typeahead.Input.create ~reset ~cycle text
          |> Result.ok
          |> Option.map ~f:(fun input -> Typeahead input)
      in
      Option.bind input ~f:(fun input -> filter_map input ~f:find_key))
  ;;
end
