open Core
module E = Bonsai.Effect
module View = Gpuio.View
module Style = Gpuio.Style
module Length = Gpuio.Length
module Color = Gpuio.Color

let px = Length.px_exn
let token = Color.token_exn

let height config =
  match Gpuio.Virtual_list.Config.height config with
  | Fixed height | Estimated height -> height
;;

let label item =
  View.text
    ~style:
      (Style.create_exn
         [ Grow 1.; Min_width (px 0.); White_space No_wrap; Text_overflow Ellipsis ])
    (Gpuio.Tree.Node.label item.Gpuio.Tree_rows.Item.node)
;;

let row_style config depth =
  [ Style.Property.Min_height (px (height config))
  ; Width (Length.percent_exn 100.)
  ; Align_items Center
  ; Gap (px 6.)
  ; Padding_left (px (6. +. (Float.of_int (depth - 1) *. 16.)))
  ; Padding_right (px 6.)
  ; Foreground (token "foreground")
  ]
;;

let item item ~config ~content ~on_toggle =
  let disabled = Gpuio.Tree.Node.is_disabled item.Gpuio.Tree_rows.Item.node in
  let size = Float.min 20. (height config) in
  let chevron_style =
    Style.create_exn
      [ Width (px size)
      ; Height (px size)
      ; Shrink 0.
      ; Align_items Center
      ; Justify_content Center
      ]
  in
  let chevron =
    match item.expanded with
    | None -> View.row ~style:chevron_style []
    | Some expanded ->
      let pointer =
        Gpuio.Pointer.Config.create ~label:"Toggle branch" ~disabled () |> Or_error.ok_exn
      in
      View.pointer_area
        ~style:chevron_style
        ~config:pointer
        ~on_event:(fun event ->
          match event.Gpuio.Pointer.Event.phase with
          | Released
            when Float.(
                   event.local_position.x >= 0.
                   && event.local_position.x < size
                   && event.local_position.y >= 0.
                   && event.local_position.y < size) -> on_toggle
          | Started | Moved | Released | Cancelled _ -> E.Ignore)
        [ View.text (if expanded then "⌄" else "›") ]
  in
  let style =
    (row_style config item.position.depth
     @ (if item.selected
        then [ Style.Property.Background (Gpuio.Background.solid (token "accent")) ]
        else [])
     @ if disabled then [ Style.Property.Foreground (token "muted") ] else [])
    |> Style.create_exn
  in
  View.with_accessibility
    (View.row
       ~style
       [ chevron
       ; View.row ~style:(Style.create_exn [ Grow 1.; Min_width (px 0.) ]) [ content ]
       ])
    (Gpuio.Tree_rows.Item.accessibility item)
  |> Or_error.ok_exn
;;

let boundary boundary ~config ~can_load ~request ~retry =
  let content =
    match boundary.Gpuio.Tree_rows.Boundary.status with
    | Ready ->
      View.button ~disabled:(not can_load) ~on_click:(fun () -> request) "Load children"
    | Failed error ->
      View.row
        ~style:(Style.create_exn [ Gap (px 8.); Align_items Center ])
        [ View.button ~disabled:(not can_load) ~on_click:(fun () -> retry) "Retry"
        ; View.text
            ~style:
              (Style.create_exn
                 [ Foreground (token "muted")
                 ; White_space No_wrap
                 ; Text_overflow Ellipsis
                 ])
            (Error.to_string_hum error)
        ]
    | Queued -> View.text "Waiting to load…"
    | Loading ->
      let loading =
        Gpuio.Loading.Config.create ~kind:Spinner ~label:"Loading children" ()
        |> Or_error.ok_exn
      in
      View.row
        ~style:(Style.create_exn [ Gap (px 8.); Align_items Center ])
        [ View.loading ~config:loading (); View.text "Loading…" ]
    | End -> View.text ""
  in
  View.row ~style:(Style.create_exn (row_style config boundary.depth)) [ content ]
;;
