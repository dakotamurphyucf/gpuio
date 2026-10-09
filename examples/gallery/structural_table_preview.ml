open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module T = Table_view

let ok = Or_error.ok_exn
let key = Key.of_string_exn
let style = Style.create_exn
let px = Length.px_exn

let component palette graph =
  let reversed, set_reversed = B.state false graph in
  let inspected, set_inspected = B.state "No result opened" graph in
  let open B.Let_syntax in
  let%arr p = palette
  and reversed = reversed
  and set_reversed = set_reversed
  and inspected = inspected
  and set_inspected = set_inspected in
  let open Style.Property in
  let cell ?kind ?span name content =
    T.Cell.create ~key:(key name) ?kind ?span content |> ok
  in
  let row ?(header = false) name cells =
    T.Row.create
      ~key:(key name)
      ~style:
        (style
           ([ Border_bottom_width 1.; Border_color (Palette.border p) ]
            @
            if header
            then [ Background (Background.solid (Palette.surface p)); Font_weight 600 ]
            else []))
      cells
    |> ok
  in
  let section name rows = T.Section.create ~key:(key name) rows |> ok in
  let head ?span name text =
    cell ~kind:Column_header ?span name [ Palette.text p text ]
  in
  let header =
    section
      "head"
      [ row
          ~header:true
          "group"
          [ head ~span:2 "work" "Research workspace"; head "actions" "Review" ]
      ; row
          ~header:true
          "columns"
          [ head "project" "PROJECT"; head "status" "STATUS"; head "open" "ACTION" ]
      ]
  in
  let rows =
    List.map
      [ "Memory", "Ready"; "Native interface", "In review"; "Streaming", "Ready" ]
      ~f:(fun (name, status) ->
        row
          name
          [ cell ~kind:Row_header "name" [ Palette.text p name ]
          ; cell "status" [ Palette.text p ~muted:true status ]
          ; cell "action" [ Palette.button p "Open" (set_inspected ("Opened " ^ name)) ]
          ])
  in
  let table =
    T.create
      ~columns:3
      ~label:"Workspace review summary"
      ~header
      ~footer:
        (section
           "footer"
           [ row
               "total"
               [ cell ~span:2 "summary" [ Palette.text p "3 projects" ]
               ; cell "count" [ Palette.text p "2 ready" ]
               ]
           ])
      ~caption:
        (Palette.text p ~muted:true "Grouped headings, merged cells, and native actions.")
      ~style:
        (style
           [ Foreground (Palette.foreground p)
           ; Border_width 1.
           ; Border_color (Palette.border p)
           ; Radius 8.
           ])
      ~cell_style:(style [ Padding (px (Palette.size p 12.)) ])
      [ section "projects" (if reversed then List.rev rows else rows) ]
    |> ok
  in
  Palette.card
    p
    ~title:"Structural table"
    [ V.row
        ~style:(style [ Gap (px 12.) ])
        [ Palette.button p ~selected:reversed "Reverse rows" (set_reversed (not reversed))
        ; Palette.text p inspected
        ]
    ; table
    ]
;;
