open Core
module W = Gpuio_protocol.List_wire

module Height = struct
  type t =
    | Estimated of float
    | Fixed of float
  [@@deriving equal, sexp_of]
end

module Scroll_policy = struct
  type t =
    | Keep_position
    | Follow_tail_when_at_end
  [@@deriving equal, sexp_of]
end

module Config = struct
  type t =
    { height : Height.t
    ; overscan : float
    ; max_active : int
    ; scroll : Scroll_policy.t
    ; scrollbar : bool
    }
  [@@deriving equal, sexp_of]

  let to_wire t ~managed : W.Config.t =
    { estimated_height =
        (match t.height with
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

  let create
        ?(overscan = 256.)
        ?(max_active = 4096)
        ?(scroll = Scroll_policy.Keep_position)
        ?(scrollbar = true)
        ~height
        ()
    =
    let t = { height; overscan; max_active; scroll; scrollbar } in
    let open Or_error.Let_syntax in
    let%map () = W.Config.validate (to_wire t ~managed:true) in
    t
  ;;

  let height t = t.height
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
  let to_end ~serial () = make ~serial End
end

module Expert = struct
  let config_to_wire = Config.to_wire

  let row_style t =
    let sizing =
      match Config.height t with
      | Estimated _ -> [ Style.Property.Min_height (Length.px_exn 1.) ]
      | Fixed height ->
        [ Style.Property.Height (Length.px_exn height)
        ; Min_height (Length.px_exn height)
        ; Max_height (Length.px_exn height)
        ; Overflow_y Hidden
        ]
    in
    Style.create_exn (Style.Property.Width (Length.percent_exn 100.) :: sizing)
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
      | End -> Ok W.Scroll_target.End
    in
    { W.Scroll_request.serial = t.serial; target }
  ;;
end
