open Core
open Gpuio

let key = Key.of_string_exn

let card ~title children =
  let style =
    Style.create_exn
      [ Padding (Length.px_exn 16.)
      ; Gap (Length.px_exn 10.)
      ; Radius 10.
      ; Border_width 1.
      ; Border_color (Color.token_exn "muted")
      ; User_select true
      ; Selection_color (Color.rgb_exn 0x486a98)
      ]
  in
  View.column
    ~style
    (View.text ~key:(key "title") ~style:(Style.create_exn [ Font_weight 600 ]) title
     :: children)
;;

let counter ~value ~on_increment =
  let style =
    Style.create_exn [ Padding (Length.px_exn 12.) ]
    |> fun t ->
    Style.with_state_exn
      t
      Focused
      [ Background (Background.solid (Color.rgb_exn 0x4e52a8)) ]
    |> fun t ->
    Style.with_state_exn
      t
      Hovered
      [ Background (Background.solid (Color.rgb_exn 0x467ad8)) ]
    |> fun t ->
    Style.with_state_exn
      t
      Pressed
      [ Background (Background.solid (Color.rgb_exn 0x264a88)) ]
  in
  View.row
    ~key:(key "counter")
    ~style:(Style.create_exn [ Gap (Length.px_exn 12.); Align_items Center ])
    [ View.button
        ~key:(key "increment")
        ~style
        ~accessible_name:"Increment counter"
        ~on_click:on_increment
        "+1"
    ; View.text ~key:(key "value") (sprintf "Count: %d" value)
    ]
;;

let app ~value ~on_increment ~on_theme =
  let style =
    Style.create_exn
      [ Width (Length.percent_exn 100.)
      ; Padding (Length.px_exn 24.)
      ; Gap (Length.px_exn 16.)
      ; Background (Background.solid (Color.token_exn "background"))
      ; Foreground (Color.token_exn "foreground")
      ; Font_size 16.
      ]
  in
  View.column
    ~style
    [ View.text
        ~key:(key "heading")
        ~style:(Style.create_exn [ Font_size 24.; Font_weight 700 ])
        "GPUIO · typed OCaml components"
    ; counter ~value ~on_increment
    ; View.button ~key:(key "theme") ~on_click:on_theme "Change theme"
    ; View.grid
        ~key:(key "cards")
        ~columns:2
        ~style:(Style.create_exn [ Gap (Length.px_exn 16.) ])
        [ card
            ~title:"Native interaction"
            [ View.text
                "Hover, press, or focus a button. Enter and Space activate it; Tab moves \
                 focus."
            ]
        ; card
            ~title:"Selectable Unicode text"
            [ View.text
                "Select this text with the mouse, or focus it and use Cmd/Ctrl+A and \
                 Cmd/Ctrl+C.\n\
                 OCaml → GPUI · 👨‍👩‍👧‍👦"
            ]
        ]
      |> Or_error.ok_exn
    ]
;;
