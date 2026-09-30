open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module T = Presentation.Tag

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let key = Key.of_string_exn

module Paint = struct
  type t =
    | Default
    | Override
    | Unset
end

let custom =
  T.Palette.create
    ~background:(Color.rgb_exn 0x543c9d)
    ~foreground:(Color.rgb_exn 0xffffff)
    ~border:(Color.rgb_exn 0x917bd2)
;;

let component palette graph =
  let variant, next_variant =
    B.state_machine0
      ~default_model:T.Variant.Secondary
      ~apply_action:(fun _ v () ->
        match v with
        | Primary -> Secondary
        | Secondary -> Danger
        | Danger -> Success
        | Success -> Warning
        | Warning -> Info
        | Info -> Custom custom
        | Custom _ -> Primary)
      graph
  in
  let size, next_size =
    B.state_machine0
      ~default_model:T.Size.Medium
      ~apply_action:(fun _ v () ->
        match v with
        | XSmall -> Small
        | Small -> Medium
        | Medium -> Large
        | Large -> XSmall)
      graph
  in
  let paint, next_paint =
    B.state_machine0
      ~default_model:Paint.Default
      ~apply_action:(fun _ v () ->
        match v with
        | Default -> Paint.Override
        | Override -> Unset
        | Unset -> Default)
      graph
  in
  let outline, toggle_outline = B.toggle ~default_model:false graph in
  let label, toggle_label = B.toggle ~default_model:true graph in
  let rich, toggle_rich = B.toggle ~default_model:true graph in
  let rounded, toggle_rounded = B.toggle ~default_model:false graph in
  let reversed, toggle_reversed = B.toggle ~default_model:false graph in
  let actions, act =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ n () -> n + 1) graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and variant = variant
  and next_variant = next_variant
  and size = size
  and next_size = next_size
  and paint = paint
  and next_paint = next_paint
  and outline = outline
  and toggle_outline = toggle_outline
  and label = label
  and toggle_label = toggle_label
  and rich = rich
  and toggle_rich = toggle_rich
  and rounded = rounded
  and toggle_rounded = toggle_rounded
  and reversed = reversed
  and toggle_reversed = toggle_reversed
  and actions = actions
  and act = act in
  let variant_name =
    match variant with
    | Primary -> "Primary"
    | Secondary -> "Secondary"
    | Danger -> "Danger"
    | Success -> "Success"
    | Warning -> "Warning"
    | Info -> "Info"
    | Custom _ -> "Custom"
  in
  let size_name =
    match size with
    | XSmall -> "XS"
    | Small -> "S"
    | Medium -> "M"
    | Large -> "L"
  in
  let paint_name, paint_style =
    match paint with
    | Paint.Default -> "Default", Style.empty
    | Override ->
      ( "Override"
      , style [ Opacity 0.65 ] |> fun s -> Style.with_state_exn s Hovered [ Opacity 0.4 ]
      )
    | Unset -> "Unset", Style.unset (style [ Opacity 0.65 ]) ~state:Hovered Opacity
  in
  let children =
    (if label
     then [ V.text ~key:(key "label") ~style:(style [ User_select true ]) "Ready · 京都" ]
     else [])
    @
    if rich
    then
      [ V.button
          ~key:(key "action")
          ~style:
            (style
               [ Font_size 11.
               ; Padding (px 3.)
               ; Background (Background.solid (Palette.background p))
               ; Foreground (Palette.foreground p)
               ])
          ~on_click:(act ())
          "Tag action"
      ]
    else []
  in
  let variant =
    match variant with
    | T.Variant.Custom _ ->
      T.Variant.Custom
        (T.Palette.create
           ~background:(Palette.background p)
           ~foreground:(Palette.foreground p)
           ~border:(Palette.accent p))
    | Primary | Secondary | Danger | Success | Warning | Info -> variant
  in
  let tag =
    T.create
      (Palette.appearance p)
      ~key:(key "tag")
      ~variant
      ~size
      ~outline
      ~style:
        (Style.merge
           [ style [ Width (px 280.); Gap (px 8.) ]
           ; (if rounded then style [ Radius 16. ] else Style.empty)
           ; paint_style
           ])
      (if reversed then List.rev children else children)
    |> fun view ->
    V.with_accessibility
      view
      (Accessibility.create ~role:Group ~label:"Tag preview" () |> ok)
    |> ok
  in
  let checkbox label checked on_toggle =
    V.checkbox ~state:(if checked then Checked else Unchecked) ~on_toggle label
  in
  V.column
    ~style:(style [ Gap (px 12.) ])
    [ Palette.text p ~muted:true "A compact piece of context, with room for an action."
    ; V.row [ tag ]
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ Palette.button p ("Tag variant: " ^ variant_name) (next_variant ())
        ; Palette.button p ("Tag size: " ^ size_name) (next_size ())
        ; Palette.button p ("Tag hover: " ^ paint_name) (next_paint ())
        ]
    ; V.row
        ~style:(style [ Gap (px 14.); Wrap Wrap ])
        [ checkbox "Tag outline" outline toggle_outline
        ; checkbox "Tag label" label toggle_label
        ; checkbox "Tag action slot" rich toggle_rich
        ; checkbox "Round tag corners" rounded toggle_rounded
        ; checkbox "Reverse tag children" reversed toggle_reversed
        ]
    ; Palette.text p ~muted:true (sprintf "Tag actions: %d" actions)
    ]
;;
