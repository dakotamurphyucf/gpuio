open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

let component palette graph =
  let open_, set_open = B.state false graph in
  let fixed, toggle_fixed = B.toggle ~default_model:true graph in
  let roomy, toggle_roomy = B.toggle ~default_model:false graph in
  let corner, set_corner = B.state Placement.Corner.Top_left graph in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr set_open = set_open in
       set_open false)
    graph;
  let%arr p = palette
  and open_ = open_
  and set_open = set_open
  and fixed = fixed
  and toggle_fixed = toggle_fixed
  and roomy = roomy
  and toggle_roomy = toggle_roomy
  and corner = corner
  and set_corner = set_corner in
  let viewport_margin = if roomy then 48. else 8. in
  let placement =
    if fixed
    then Placement.at_point ~corner ~viewport_margin ~x:560. ~y:400. () |> ok
    else Placement.create ~viewport_margin ~offset:8. () |> ok
  in
  let controls () =
    [ V.switch ~checked:fixed ~on_toggle:toggle_fixed "Place at window point"
    ; V.switch ~checked:roomy ~on_toggle:toggle_roomy "Roomier window margin"
    ; V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        (List.map
           [ Placement.Corner.Top_left, "Top left"
           ; Top_right, "Top right"
           ; Bottom_left, "Bottom left"
           ; Bottom_right, "Bottom right"
           ]
           ~f:(fun (value, label) ->
             Palette.button
               p
               ~selected:(Placement.Corner.equal corner value)
               label
               (set_corner value)))
    ]
  in
  Palette.card
    p
    ~title:"Place a panel precisely"
    [ Palette.text
        p
        ~muted:true
        "Choose a corner at window point (560, 400), or attach the panel to its button. \
         Resize the window to see its margin adapt. Controls inside the open panel move \
         it without closing it."
    ; V.popover
        ~config:
          (Overlay.Config.create ~label:"Placement preview" ~width:340. ~placement ()
           |> ok)
        ~style:
          (style
             [ Padding (px 20.)
             ; Radius 14.
             ; Background (Background.solid (Palette.surface p))
             ; Foreground (Palette.foreground p)
             ])
        ~on_dismiss:(fun _ -> set_open false)
        ~anchor:(Palette.button p "Open placement preview" (set_open true))
        (if open_
         then
           Some
             (V.column
                ~style:(style [ Gap (px 16.) ])
                ([ Palette.text p ~size:20. "A place for everything" ]
                 @ controls ()
                 @ [ Palette.button p "Close placement preview" (set_open false) ]))
         else None)
    ]
;;
