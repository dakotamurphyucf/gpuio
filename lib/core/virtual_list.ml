open Core
module W = Gpuio_protocol.List_wire

module Axis = struct
  type t =
    | Vertical
    | Horizontal
  [@@deriving equal, sexp_of]
end

module Extent = struct
  type t =
    | Estimated of float
    | Fixed of float
  [@@deriving equal, sexp_of]
end

module Height = Extent

module Scroll_policy = struct
  type t =
    | Keep_position
    | Follow_tail_when_at_end
  [@@deriving equal, sexp_of]
end

module Config = struct
  type t =
    { axis : Axis.t
    ; extent : Extent.t
    ; overscan : float
    ; max_active : int
    ; scroll : Scroll_policy.t
    ; scrollbar : bool
    }
  [@@deriving equal, sexp_of]

  let to_wire t ~managed : W.Config.t =
    { estimated_height =
        (match t.extent with
         | Estimated value | Fixed value -> value)
    ; overscan = t.overscan
    ; max_active = Int64.of_int t.max_active
    ; scroll_policy =
        (match t.scroll with
         | Keep_position -> Keep_position
         | Follow_tail_when_at_end -> Follow_tail_when_at_end)
    ; scrollbar = t.scrollbar
    ; managed
    }
  ;;

  let make
        ?(overscan = 256.)
        ?(max_active = 4096)
        ?(scroll = Scroll_policy.Keep_position)
        ?(scrollbar = true)
        ~axis
        ~extent
        ()
    =
    let t = { axis; extent; overscan; max_active; scroll; scrollbar } in
    let open Or_error.Let_syntax in
    let%map () = W.Config.validate (to_wire t ~managed:true) in
    t
  ;;

  let create ?overscan ?max_active ?scroll ?scrollbar ~height () =
    make ?overscan ?max_active ?scroll ?scrollbar ~axis:Vertical ~extent:height ()
  ;;

  let horizontal ?overscan ?max_active ?scroll ?scrollbar ~width () =
    make ?overscan ?max_active ?scroll ?scrollbar ~axis:Horizontal ~extent:width ()
  ;;

  let axis t = t.axis
  let extent t = t.extent
  let height = extent
  let max_active t = t.max_active
end

module Order = struct
  type t =
    { keys : Key.t list
    ; membership : String.Set.t
    }

  let create keys =
    if List.length keys > W.max_logical_rows
    then Or_error.error_string "virtual list logical-row limit exceeded"
    else (
      let membership = List.map keys ~f:Key.to_string |> String.Set.of_list in
      if Set.length membership <> List.length keys
      then Or_error.error_string "virtual list keys must be unique"
      else Ok { keys; membership })
  ;;

  let keys t = t.keys
  let length t = Set.length t.membership
  let mem t key = Set.mem t.membership (Key.to_string key)
end

module Viewport = struct
  type t =
    { visible_first : int
    ; visible_last : int
    ; requested : Key.t list
    ; pinned : Key.t list
    ; anchor : (Key.t * float) option
    ; following_tail : bool
    ; at_start : bool
    ; at_end : bool
    ; budget_exhausted : bool
    }
  [@@deriving equal, sexp_of]
end

module Scroll_request = struct
  module Target = struct
    type t =
      | Offset of Key.t * float
      | Reveal of Key.t
      | End
      | Focus_tree_row of Key.t
    [@@deriving equal, sexp_of]
  end

  type t =
    { serial : int64
    ; target : Target.t
    }
  [@@deriving equal, sexp_of]

  let make ~serial target =
    if Int64.(serial < 1L)
    then Or_error.error_string "virtual list scroll serial must be positive"
    else Ok { serial; target }
  ;;

  let to_row ~serial ?(offset = 0.) key =
    if not (Float.is_finite offset && Float.(offset >= 0. && offset <= 1_000_000.))
    then Or_error.error_string "virtual list scroll offset is out of bounds"
    else make ~serial (Offset (key, offset))
  ;;

  let reveal ~serial key = make ~serial (Reveal key)
  let focus_tree_row ~serial key = make ~serial (Focus_tree_row key)
  let to_end ~serial () = make ~serial End
end

module Expert = struct
  let axis_to_wire : Axis.t -> W.Axis.t = function
    | Vertical -> Vertical
    | Horizontal -> Horizontal
  ;;

  let config_to_wire = Config.to_wire

  let row_style t =
    let open Style.Property in
    let sizing =
      match Config.axis t, Config.extent t with
      | Vertical, Estimated _ ->
        [ Width (Length.percent_exn 100.); Min_height (Length.px_exn 1.) ]
      | Horizontal, Estimated _ ->
        [ Height (Length.percent_exn 100.); Min_width (Length.px_exn 1.) ]
      | Vertical, Fixed value ->
        [ Width (Length.percent_exn 100.)
        ; Height (Length.px_exn value)
        ; Min_height (Length.px_exn value)
        ; Max_height (Length.px_exn value)
        ; Overflow_y Hidden
        ]
      | Horizontal, Fixed value ->
        [ Height (Length.percent_exn 100.)
        ; Width (Length.px_exn value)
        ; Min_width (Length.px_exn value)
        ; Max_width (Length.px_exn value)
        ; Overflow_x Hidden
        ]
    in
    Style.create_exn sizing
  ;;

  let viewport_of_wire (t : W.Viewport.t) ~find_key =
    let open Or_error.Let_syntax in
    let%bind () = W.Viewport.validate t in
    let key id =
      Option.value_map
        (find_key id)
        ~default:(Or_error.error_string "unknown virtual list row identity")
        ~f:Or_error.return
    in
    let%bind requested = List.map t.requested ~f:key |> Or_error.all in
    let%bind pinned = List.map t.pinned ~f:key |> Or_error.all in
    let%map anchor =
      match t.anchor with
      | None -> Ok None
      | Some (id, offset) ->
        let%map key = key id in
        Some (key, offset)
    in
    { Viewport.visible_first = Int64.to_int_exn t.visible_first
    ; visible_last = Int64.to_int_exn t.visible_last
    ; requested
    ; pinned
    ; anchor
    ; following_tail = t.following_tail
    ; at_start = t.at_start
    ; at_end = t.at_end
    ; budget_exhausted = t.budget_exhausted
    }
  ;;

  let scroll_to_wire (t : Scroll_request.t) ~find_id =
    let open Or_error.Let_syntax in
    let id key =
      Option.value_map
        (find_id key)
        ~default:(Or_error.error_string "virtual list scroll key is absent")
        ~f:Or_error.return
    in
    let%map target : W.Scroll_target.t Or_error.t =
      match t.target with
      | Offset (key, offset) ->
        let%map id = id key in
        W.Scroll_target.Offset (id, offset)
      | Reveal key ->
        let%map id = id key in
        W.Scroll_target.Reveal id
      | Focus_tree_row key ->
        let%map id = id key in
        W.Scroll_target.Focus_tree_row id
      | End -> Ok W.Scroll_target.End
    in
    { W.Scroll_request.serial = t.serial; target }
  ;;
end
