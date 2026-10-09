open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Input = Gpuio_eio.Text_input
module Q = Container_query

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let full = Length.percent_exn 100.
let compact = Q.Branch_id.of_string "Compact" |> ok
let wide = Q.Branch_id.of_string "Wide" |> ok
let short = Q.Branch_id.of_string "Short" |> ok

let config =
  Q.Config.create
    ~default:compact
    [ Q.Rule.create
        ~branch:short
        ~condition:(Q.Predicate.create ~height:(Q.Range.create ~maximum:230. () |> ok) ())
    ; Q.Rule.create
        ~branch:wide
        ~condition:(Q.Predicate.create ~width:(Q.Range.create ~minimum:480. () |> ok) ())
    ]
  |> ok
;;

let branch window palette ~name ~description ~inline graph =
  let count, increment =
    B.state_machine0
      ~default_model:0
      ~apply_action:(fun _ count () -> if count = Int.max_value then count else count + 1)
      graph
  in
  let editor =
    Input.create
      window
      ~initial_text:(name ^ " ideas stay here.")
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:(name ^ " layout draft") ()
            |> ok))
      graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and count = count
  and increment = increment
  and editor = editor in
  V.column
    ~style:
      (style
         [ Width full
         ; Height full
         ; Padding (px 18.)
         ; Gap (px 10.)
         ; Radius 12.
         ; Overflow_y Scroll
         ; Background (Background.solid (Palette.background p))
         ; Border_width 1.
         ; Border_color (Palette.accent p)
         ])
    [ Palette.text p ~size:20. (name ^ " layout")
    ; Palette.text p ~muted:true description
    ; (let input =
         Input.view ~style:(style [ Height (px 38.); Min_width (px 0.); Grow 1. ]) editor
       in
       let save = Palette.button p (sprintf "%s saves: %d" name count) (increment ()) in
       if inline
       then
         V.row
           ~style:(style [ Gap (px 12.); Align_items Center; Shrink 0. ])
           [ input; save ]
       else V.column ~style:(style [ Gap (px 10.); Shrink 0. ]) [ input; save ])
    ]
;;

let component window palette graph =
  let width, set_width = B.state 400. graph in
  let height, set_height = B.state 300. graph in
  let observation, observe =
    B.state_machine0
      ~default_model:(None, 0)
      ~apply_action:(fun _ (_, count) selection ->
        Some selection, if count = Int.max_value then count else count + 1)
      graph
  in
  let compact_view =
    branch
      window
      palette
      ~name:"Compact"
      ~description:"A focused place for one idea."
      ~inline:false
      graph
  in
  let wide_view =
    branch
      window
      palette
      ~name:"Wide"
      ~description:"More room to develop your next idea."
      ~inline:true
      graph
  in
  let short_view =
    branch
      window
      palette
      ~name:"Short"
      ~description:"The essentials, close at hand."
      ~inline:false
      graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and width = width
  and set_width = set_width
  and height = height
  and set_height = set_height
  and observation = observation
  and observe = observe
  and compact_view = compact_view
  and wide_view = wide_view
  and short_view = short_view in
  let selection, count = observation in
  let controls options current set =
    V.row
      ~style:(style [ Gap (px 8.); Wrap Wrap ])
      (List.map options ~f:(fun (label, value) ->
         Palette.button p ~selected:(Float.equal current value) label (set value)))
  in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ Palette.card
        p
        ~title:"A layout that finds its fit"
        [ Palette.text
            p
            ~muted:true
            "Change the available space. Each presentation keeps its own draft and saves."
        ; controls
            [ "Width 400", 400.; "Width 479", 479.; "Width 480", 480.; "Width 600", 600. ]
            width
            set_width
        ; controls
            [ "Height 200", 200.; "Height 230", 230.; "Height 300", 300. ]
            height
            set_height
        ; Palette.text p (sprintf "Offered size: %.0f × %.0f logical pixels" width height)
        ; V.column
            ~style:(style [ Overflow_x Scroll; Width full ])
            [ V.container_query
                ~style:(style [ Width (px width); Height (px height); Shrink 0. ])
                ~on_select:observe
                config
                [ compact, compact_view; wide, wide_view; short, short_view ]
              |> ok
            ]
        ; Palette.text
            p
            (match selection with
             | None -> "Waiting for a painted layout"
             | Some selection ->
               sprintf
                 "Painted layout: %s · %.0f × %.0f · observation %d"
                 (Q.Branch_id.to_string selection.Q.Selection.branch)
                 selection.width
                 selection.height
                 count)
        ]
    ; Palette.card
        p
        ~title:"Simple rules, stable state"
        [ Palette.text p "1. Below 230 pixels tall → Short, at any width."
        ; Palette.text p "2. At least 480 pixels wide → Wide."
        ; Palette.text p "3. Otherwise → Compact."
        ; Palette.text
            p
            ~muted:true
            "The first matching rule wins. Native layout selects the branch; OCaml \
             receives a notification after paint. A resize within the same branch does \
             not emit another selection."
        ; Palette.text
            p
            ~muted:true
            "Hidden layouts retain their drafts but cannot receive input. Leaving this \
             page disposes the editors; the chosen size and save counts remain."
        ]
    ]
;;
