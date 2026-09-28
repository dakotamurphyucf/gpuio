open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let source = "/workspace/projects/native-studio/src/main.ml"

let cursors : (string * Style.Cursor.t) array =
  [| "Arrow", Arrow
   ; "Text", Ibeam
   ; "Pointer", Pointer
   ; "Crosshair", Crosshair
   ; "Move", Move
   ; "Not allowed", Not_allowed
   ; "Horizontal resize", Resize_horizontal
   ; "Vertical resize", Resize_vertical
   ; "Grab", Grab
   ; "Grabbing", Grabbing
   ; "Vertical text", Ibeam_vertical
   ; "Column resize", Resize_column
   ; "Row resize", Resize_row
   ; "Northwest–southeast resize", Resize_nw_se
   ; "Northeast–southwest resize", Resize_ne_sw
   ; "Left resize", Resize_left
   ; "Right resize", Resize_right
   ; "Up resize", Resize_up
   ; "Down resize", Resize_down
   ; "Alias", Alias
   ; "Copy", Copy
   ; "Context menu", Context_menu
  |]
;;

let component palette graph =
  let index, next =
    B.state_machine0
      ~default_model:0
      ~apply_action:(fun _ index () -> (index + 1) % Array.length cursors)
      graph
  in
  let narrow, toggle_width = B.toggle ~default_model:false graph in
  let open B.Let_syntax in
  let%arr p = palette
  and index = index
  and next = next
  and narrow = narrow
  and toggle_width = toggle_width in
  let label, cursor = cursors.(index) in
  let width = if narrow then 140. else 250. in
  let sample label overflow =
    V.column
      ~style:(style [ Gap (px 6.); Shrink 0. ])
      [ Palette.text p ~muted:true label
      ; V.text
          ~key:(Key.of_string_exn label)
          ~style:
            (style
               [ Width (px width)
               ; Font_size 16.
               ; Font_family "Menlo"
               ; Foreground (Palette.foreground p)
               ; White_space No_wrap
               ; Text_overflow overflow
               ; Overflow Hidden
               ])
          source
      ]
  in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ Palette.card
        p
        ~title:"Keep the part that matters"
        [ Palette.text
            p
            ~muted:true
            "Truncation changes the painted text. The full path remains available to \
             assistive technology."
        ; Palette.button
            p
            (if narrow then "Widen text previews" else "Narrow text previews")
            toggle_width
        ; Palette.text p (sprintf "Text preview width: %.0f" width)
        ; V.row
            ~style:(style [ Gap (px 20.); Wrap Wrap ])
            [ sample "Clip" Clip
            ; sample "End ellipsis" Ellipsis
            ; sample "Start ellipsis" Ellipsis_start
            ]
        ]
    ; Palette.card
        p
        ~title:"A cursor for the task"
        [ Palette.button p "Next cursor" (next ())
        ; Palette.text
            p
            (sprintf "Cursor %d of %d: %s" (index + 1) (Array.length cursors) label)
        ; (V.column
             ~style:
               (style
                  [ Width (px 340.)
                  ; Height (px 100.)
                  ; Padding (px 20.)
                  ; Radius 12.
                  ; Background (Background.solid (Palette.background p))
                  ; Border_width 1.
                  ; Border_color (Palette.accent p)
                  ; Cursor cursor
                  ])
             [ Palette.text p "Move your pointer over this surface" ]
           |> fun view ->
           V.with_accessibility
             view
             (Accessibility.create ~role:Group ~label:"Cursor preview surface" () |> ok)
           |> ok)
        ; Palette.text
            p
            ~muted:true
            "Cursor artwork follows the platform. Column and horizontal resize share a \
             shape on macOS, as do row and vertical resize."
        ]
    ]
;;
