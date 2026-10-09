open Core
module W = Gpuio_protocol.List_input_wire

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

let filter_map t ~f =
  match t with
  | Navigate (navigation, gesture) -> Some (Navigate (navigation, gesture))
  | Select (key, gesture) -> Option.map (f key) ~f:(fun key -> Select (key, gesture))
  | Focus key -> Option.map (f key) ~f:(fun key -> Focus key)
  | Select_active gesture -> Some (Select_active gesture)
  | Confirm (key, kind) -> Option.map (f key) ~f:(fun key -> Confirm (key, kind))
  | Confirm_active kind -> Some (Confirm_active kind)
  | Context key -> Option.map (f key) ~f:(fun key -> Context key)
  | Context_active -> Some Context_active
  | Set_selected (key, selected) ->
    Option.map (f key) ~f:(fun key -> Set_selected (key, selected))
  | Cancel -> Some Cancel
;;

module Config = struct
  type t =
    { epoch : Key.t
    ; cursor : Key.t option
    ; query : Key.t option
    ; selection_on_navigation : bool
    ; disabled : bool
    ; busy : bool
    }
  [@@deriving equal, sexp_of]

  let create
        ~epoch
        ?cursor
        ?query
        ?(selection_on_navigation = false)
        ?(disabled = false)
        ?(busy = false)
        ()
    =
    { epoch; cursor; query; selection_on_navigation; disabled; busy }
  ;;

  let epoch t = t.epoch
  let cursor t = t.cursor
  let query t = t.query
  let selection_on_navigation t = t.selection_on_navigation
  let disabled t = t.disabled
  let busy t = t.busy
end

module Expert = struct
  let same_interaction (a : Config.t) (b : Config.t) =
    Key.equal a.epoch b.epoch
    && Option.equal Key.equal a.query b.query
    && Bool.equal a.selection_on_navigation b.selection_on_navigation
    && Bool.equal a.disabled b.disabled
  ;;

  let navigation = function
    | W.Navigation.Previous -> List_selection.Navigation.Previous
    | Next -> Next
    | First -> First
    | Last -> Last
  ;;

  let gesture = function
    | W.Gesture.Replace -> List_selection.Gesture.Replace
    | Toggle -> Toggle
    | Range { extend } -> Range { extend }
  ;;

  let confirmation = function
    | W.Confirmation.Primary -> List_selection.Confirmation.Primary
    | Secondary -> Secondary
  ;;

  let of_wire request ~find_key =
    if not (W.Request.valid request)
    then None
    else (
      let request =
        match request with
        | W.Request.Navigate (direction, selection) ->
          Navigate (navigation direction, Option.map selection ~f:gesture)
        | Select (key, selection) -> Select (key, gesture selection)
        | Focus key -> Focus key
        | Select_active selection -> Select_active (gesture selection)
        | Confirm (key, kind) -> Confirm (key, confirmation kind)
        | Confirm_active kind -> Confirm_active (confirmation kind)
        | Context key -> Context key
        | Context_active -> Context_active
        | Set_selected (key, selected) -> Set_selected (key, selected)
        | Cancel -> Cancel
      in
      filter_map request ~f:find_key)
  ;;
end
