open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module D = Presentation.Description_list
module Editor = Gpuio_eio.Text_input

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let full = Length.percent_exn 100.
let key = Key.of_string_exn

let named label children =
  V.column ~style:(style [ Width full; Min_width (px 0.); Gap (px 4.) ]) children
  |> fun view ->
  V.with_accessibility view (Accessibility.create ~role:Group ~label () |> ok) |> ok
;;

let component window palette graph =
  let columns, next_columns =
    B.state_machine0 ~default_model:3 ~apply_action:(fun _ n () -> 1 + (n % 10)) graph
  in
  let vertical, toggle_vertical = B.toggle ~default_model:false graph in
  let bordered, toggle_bordered = B.toggle ~default_model:true graph in
  let mixed, toggle_mixed = B.toggle ~default_model:false graph in
  let separators, toggle_separators = B.toggle ~default_model:false graph in
  let reversed, toggle_reversed = B.toggle ~default_model:false graph in
  let proportional, toggle_proportional = B.toggle ~default_model:false graph in
  let expanded, toggle_expanded = B.toggle ~default_model:false graph in
  let show_action, toggle_action = B.toggle ~default_model:true graph in
  let refined, toggle_refined = B.toggle ~default_model:false graph in
  let width, next_width =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ n () -> (n + 1) % 4) graph
  in
  let size, next_size =
    B.state_machine0
      ~default_model:D.Size.Medium
      ~apply_action:(fun _ s () ->
        match s with
        | XSmall -> Small
        | Small -> Medium
        | Medium -> Large
        | Large -> XSmall)
      graph
  in
  let actions, act =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ n () -> n + 1) graph
  in
  let editor =
    Editor.create
      window
      ~initial_text:"Retained draft"
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Description value" () |> ok))
      graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and columns = columns
  and next_columns = next_columns
  and vertical = vertical
  and toggle_vertical = toggle_vertical
  and bordered = bordered
  and toggle_bordered = toggle_bordered
  and mixed = mixed
  and toggle_mixed = toggle_mixed
  and separators = separators
  and toggle_separators = toggle_separators
  and reversed = reversed
  and toggle_reversed = toggle_reversed
  and proportional = proportional
  and toggle_proportional = toggle_proportional
  and expanded = expanded
  and toggle_expanded = toggle_expanded
  and show_action = show_action
  and toggle_action = toggle_action
  and refined = refined
  and toggle_refined = toggle_refined
  and width = width
  and next_width = next_width
  and size = size
  and next_size = next_size
  and actions = actions
  and act = act
  and editor = editor in
  let width = [| 720.; 607.3; 541.; 333.3 |].(width) in
  let spans =
    if mixed
    then [ 1; Int.min 2 columns; 1; 1; 1; columns; 1 ]
    else List.init 12 ~f:(fun _ -> 1)
  in
  let items =
    List.mapi spans ~f:(fun n span ->
      let term =
        named
          (sprintf "Description term %d" n)
          (if n = 0
           then
             [ V.button
                 ~key:(key "term-action")
                 ~accessible_name:"Description term action"
                 ~style:(style [ Padding (px 0.); Font_size 12. ])
                 ~on_click:(act ())
                 "A"
             ]
           else [ V.text (String.of_char (Char.of_int_exn (Char.to_int 'A' + n))) ])
      in
      let definition =
        named
          (sprintf "Description definition %d" n)
          ([ V.text (sprintf "Value %d" n) ]
           @ (if n = 0
              then
                [ (Editor.view
                     ~style:(style [ Width full; Height (px 30.); Min_width (px 0.) ])
                     editor
                   |> fun view -> V.with_key view (key "editor"))
                ]
              else [])
           @ (if n = 1 && show_action
              then
                [ V.button
                    ~key:(key "value-action")
                    ~accessible_name:"Description value action"
                    ~style:(style [ Padding (px 1.); Font_size 12. ])
                    ~on_click:(act ())
                    "Open"
                ]
              else [])
           @
           if n = 2 && expanded
           then
             [ V.text
                 "A longer explanation wraps naturally within its cell. 京都 · This value \
                  can change while your draft stays in place."
             ]
           else [])
      in
      D.Item.create
        ~key:(Key.of_int n)
        ~span
        ?term_style:
          (Option.some_if
             refined
             (style [ Background (Background.solid (Palette.border p)); Padding (px 3.) ]))
        ?definition_style:(Option.some_if refined (style [ Padding (px 5.) ]))
        ~term:[ term ]
        ~definition:[ definition ]
        ()
      |> ok)
  in
  let items = if reversed then List.rev items else items in
  let items =
    if separators
    then
      List.concat_mapi items ~f:(fun n item ->
        if n = 3
        then
          [ D.Item.separator ~key:(key "separator-a") ()
          ; D.Item.separator ~key:(key "separator-b") ()
          ; item
          ]
        else [ item ])
    else items
  in
  let view =
    D.create
      (Palette.appearance p)
      ~columns
      ~axis:(if vertical then Vertical else Horizontal)
      ~size
      ~bordered
      ~label_width:(if proportional then Length.percent_exn 25. else px 64.)
      ~style:(style [ Width (px width); Max_width full ])
      items
    |> ok
  in
  let view =
    V.with_accessibility
      view
      (Accessibility.create ~role:Description_list ~label:"Workspace details" () |> ok)
    |> ok
  in
  let size_name =
    match size with
    | XSmall -> "XS"
    | Small -> "S"
    | Medium -> "M"
    | Large -> "L"
  in
  let checkbox label checked on_toggle =
    V.checkbox ~state:(if checked then Checked else Unchecked) ~on_toggle label
  in
  V.column
    ~style:(style [ Gap (px 12.) ])
    [ Palette.text p ~muted:true "Details with structure. Controls with continuity."
    ; view
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ Palette.button p (sprintf "Description columns: %d" columns) (next_columns ())
        ; Palette.button p ("Description size: " ^ size_name) (next_size ())
        ; Palette.button p (sprintf "Description width: %.1f" width) (next_width ())
        ]
    ; V.row
        ~style:(style [ Gap (px 12.); Wrap Wrap ])
        [ checkbox "Vertical description" vertical toggle_vertical
        ; checkbox "Bordered description" bordered toggle_bordered
        ; checkbox "Mixed description spans" mixed toggle_mixed
        ; checkbox "Description separators" separators toggle_separators
        ]
    ; V.row
        ~style:(style [ Gap (px 12.); Wrap Wrap ])
        [ checkbox "Reverse description entries" reversed toggle_reversed
        ; checkbox "Proportional description labels" proportional toggle_proportional
        ; checkbox "Long description value" expanded toggle_expanded
        ; checkbox "Description value action visible" show_action toggle_action
        ; checkbox "Refine description slots" refined toggle_refined
        ]
    ; Palette.text p ~muted:true (sprintf "Description actions: %d" actions)
    ]
;;
