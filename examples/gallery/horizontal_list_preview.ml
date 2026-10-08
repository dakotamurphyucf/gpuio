open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module L = Gpuio_bonsai.Virtual_list
module C = List_collection

let ok = Or_error.ok_exn
let px = Length.px_exn
let full = Length.percent_exn 100.
let style = Style.create_exn
let initial_count = 10_000
let extent id = 260. +. (Float.of_int (Int.abs id % 4) *. 24.)

module Action = struct
  type t =
    | Grow of int
    | Reverse
    | Prepend
    | Append
end

let insert source ~before =
  if C.length source >= initial_count + 100
  then source
  else (
    let ids = C.keys source in
    let id =
      if before
      then (List.min_elt ids ~compare:Int.compare |> Option.value_exn) - 1
      else (List.max_elt ids ~compare:Int.compare |> Option.value_exn) + 1
    in
    C.splice
      source
      ~at:(if before then 0 else C.length source)
      ~remove:0
      [ id, extent id ]
    |> ok)
;;

let update source = function
  | Action.Grow id ->
    (match C.find source id with
     | None -> source
     | Some extent -> C.set source ~key:id ~data:(Float.min 500. (extent +. 32.)) |> ok)
  | Reverse -> C.reorder source (List.rev (C.keys source)) |> ok
  | Prepend -> insert source ~before:true
  | Append -> insert source ~before:false
;;

let component palette scrollbar graph =
  let source, change_source =
    B.state_machine0
      ~default_model:
        (C.of_alist (module Int) (List.init initial_count ~f:(fun id -> id, extent id))
         |> ok)
      ~apply_action:(fun _ source action -> update source action)
      graph
  in
  let horizontal, set_horizontal = B.state true graph in
  let open B.Let_syntax in
  let config =
    let%arr horizontal = horizontal
    and p = palette in
    let make =
      if horizontal
      then Virtual_list.Config.horizontal ~width:(Estimated (Palette.size p 300.))
      else Virtual_list.Config.create ~height:(Estimated (Palette.size p 300.))
    in
    make ~overscan:220. ~max_active:16 ~scroll:Follow_tail_when_at_end () |> ok
  in
  let output =
    L.component_with_config
      ~scrollbar
      (module Int)
      source
      ~row_key:Key.of_int
      ~config
      ~style:
        (B.map palette ~f:(fun p ->
           style [ Width full; Height (px (Palette.size p 360.)); Shrink 0. ]))
      ~accessibility:(B.return (Accessibility.create ~label:"Research cards" () |> ok))
      ~render_row:(fun ~key:id ~data:extent ~lifetime:_ graph ->
        let clicks, set_clicks = B.state 0 graph in
        let%arr p = palette
        and id = id
        and extent = extent
        and horizontal = horizontal
        and clicks = clicks
        and set_clicks = set_clicks in
        let dimensions =
          if horizontal
          then [ Style.Property.Width (px (Palette.size p extent)); Height full ]
          else [ Style.Property.Height (px (Palette.size p extent)); Width full ]
        in
        V.column
          ~style:(style (dimensions @ [ Shrink 0.; Padding (px 8.) ]))
          [ V.column
              ~style:
                (style
                   [ Width full
                   ; Height full
                   ; Padding (px 18.)
                   ; Gap (px 12.)
                   ; Background (Background.solid (Palette.surface p))
                   ; Border_color (Palette.border p)
                   ; Border_width 1.
                   ; Radius 12.
                   ; Overflow_x Hidden
                   ; Overflow_y Hidden
                   ])
              [ Palette.text p ~muted:true (sprintf "FIELD NOTE / %05d" id)
              ; Palette.text
                  p
                  ~size:20.
                  (List.nth_exn
                     [ "Quiet patterns"
                     ; "A wider view"
                     ; "Small discoveries"
                     ; "Future directions"
                     ]
                     (Int.abs id % 4))
              ; Palette.text
                  p
                  ~muted:true
                  "Keep an idea in view. Every card has a stable identity and its own \
                   measured size."
              ; Palette.button
                  p
                  (sprintf "Local reactions · %d" clicks)
                  (set_clicks (clicks + 1))
              ]
          ])
      graph
  in
  let%arr p = palette
  and output = output
  and source = source
  and change_source = change_source
  and horizontal = horizontal
  and set_horizontal = set_horizontal in
  let output = ok output in
  let controller = L.Output.controller output in
  let first, _ = C.nth source 0 |> Option.value_exn in
  let last, _ = C.nth source (C.length source - 1) |> Option.value_exn in
  let visible =
    L.Output.viewport output
    |> Option.bind ~f:(fun viewport -> viewport.anchor)
    |> Option.map ~f:(fun (id, _) -> Key.to_string id |> Int.of_string)
  in
  let grow =
    Option.value_map visible ~default:Bonsai.Effect.Ignore ~f:(fun id ->
      change_source (Grow id))
  in
  Palette.card
    p
    ~title:"A collection that moves with you"
    [ V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ Palette.button p "First card" (L.Controller.scroll_to controller first |> ok)
        ; Palette.button
            p
            "Middle"
            (L.Controller.reveal
               controller
               (fst (C.nth source (C.length source / 2) |> Option.value_exn)))
        ; Palette.button p "Follow latest" (L.Controller.jump_to_latest controller)
        ; Palette.button p "Grow visible card" ~disabled:(Option.is_none visible) grow
        ; Palette.button p "Reverse order" (change_source Reverse)
        ; Palette.button
            p
            "Prepend"
            ~disabled:(C.length source >= initial_count + 100)
            (change_source Prepend)
        ; Palette.button
            p
            "Append"
            ~disabled:(C.length source >= initial_count + 100)
            (change_source Append)
        ; Palette.button
            p
            (if horizontal then "Use vertical axis" else "Use horizontal axis")
            (set_horizontal (not horizontal))
        ]
    ; L.Output.view output
    ; Palette.text
        p
        (sprintf
           "%d cards · %d / 16 mounted · Last key %d"
           (C.length source)
           (L.Output.active_rows output)
           last)
    ; Palette.text
        p
        ~muted:true
        "Scroll along the active axis. React to a card, then change axes: surviving rows \
         keep their local state. Grow or reorder cards while reading to check the \
         anchor."
    ]
;;
