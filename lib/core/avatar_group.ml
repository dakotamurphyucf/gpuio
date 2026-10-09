open Core

module Size = struct
  type t = float [@@deriving equal, sexp_of]

  let xsmall = 16.
  let small = 24.
  let medium = 48.
  let large = 80.

  let of_pixels value =
    if Float.is_finite value && Float.(value > 0. && value <= 1_000_000.)
    then Ok value
    else Or_error.error_string "avatar size must be finite and in (0,1000000] pixels"
  ;;

  let pixels t = t
end

module Item = struct
  type 'action t =
    { key : Key.t
    ; style : Style.t
    ; on_change : (Image.State.t -> 'action) option
    ; config : Avatar.Config.t
    ; fallback : 'action View.t option
    }

  let create ~key ?(style = Style.empty) ?on_change config =
    { key; style; on_change; config; fallback = None }
  ;;

  let create_with_fallback ~key ?(style = Style.empty) ?on_change config ~fallback =
    let%map.Or_error _ = View.avatar_with_fallback config ~fallback in
    { key; style; on_change; config; fallback = Some fallback }
  ;;

  let key t = t.key
end

let px = Length.px_exn
let key = Key.of_string_exn

let avatar_style size custom margin =
  let dimension = px (Size.pixels size) in
  Style.merge
    [ Style.create_exn
        [ Font_size (Float.max 1. (Float.min 4096. (Size.pixels size *. 0.3))) ]
    ; custom
    ; Style.create_exn
        [ Position Relative
        ; Width dimension
        ; Height dimension
        ; Min_width dimension
        ; Max_width dimension
        ; Min_height dimension
        ; Max_height dimension
        ; Grow 0.
        ; Shrink 0.
        ; Margin_left (px margin)
        ; Margin_right (px 0.)
        ]
    ]
;;

let ellipsis ?(size = Size.medium) ?(style = Style.empty) ~description () =
  let config =
    Avatar.Config.create
      ~fallback:(Avatar.Fallback.create "⋯" |> Or_error.ok_exn)
      ~description
      ()
  in
  View.avatar ~style:(avatar_style size style 0.) config
;;

let create
      ?key:group_key
      ?(style = Style.empty)
      ?(size = Size.medium)
      ?(limit = 3)
      ?(overlap = 0.3)
      ?overflow
      items
  =
  let open Or_error.Let_syntax in
  let%bind () =
    if limit < 0
    then Or_error.error_string "avatar group limit must be nonnegative"
    else if not (Float.is_finite overlap && Float.(overlap >= 0. && overlap < 1.))
    then Or_error.error_string "avatar group overlap must be finite and in [0,1)"
    else Ok ()
  in
  let%bind () =
    match List.find_a_dup (List.map items ~f:Item.key) ~compare:Key.compare with
    | None -> Ok ()
    | Some key -> Or_error.errorf "duplicate avatar item key: %s" (Key.to_string key)
  in
  let visible = List.take items limit in
  let omitted = List.length items - List.length visible in
  let avatars =
    List.mapi visible ~f:(fun index item ->
      let make =
        match item.Item.fallback with
        | None -> View.avatar
        | Some fallback ->
          fun ?key ?style ?on_change config ->
            View.avatar_with_fallback ?key ?style ?on_change config ~fallback
            |> Or_error.ok_exn
      in
      make
        ~key:item.Item.key
        ~style:
          (avatar_style
             size
             item.style
             (if index = 0 then 0. else -.(Size.pixels size *. overlap)))
        ?on_change:item.on_change
        item.config)
  in
  let body =
    if List.is_empty avatars
    then None
    else
      Some
        (View.row
           ~key:(key "items")
           ~style:
             (Style.create_exn
                [ Wrap No_wrap; Gap (px 0.); Shrink 0.; Align_items Center ])
           avatars)
  in
  let overflow =
    if omitted = 0
    then None
    else
      Option.map overflow ~f:(fun content ->
        View.row
          ~key:(key "overflow")
          ~style:
            (Style.create_exn
               [ Shrink 0.; Margin_left (px (if List.is_empty avatars then 0. else 4.)) ])
          [ content ~omitted ~size ])
  in
  Ok
    (View.row
       ?key:group_key
       ~style:
         (Style.merge
            [ style
            ; Style.create_exn
                [ Display Flex
                ; Direction Row
                ; Wrap No_wrap
                ; Gap (px 0.)
                ; Align_items Center
                ]
            ])
       (List.filter_opt [ body; overflow ]))
;;
