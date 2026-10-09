open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module P = Choice_picker
module Controller = Gpuio_eio.Choice_picker

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn
let id value = Choice.Id.of_string value |> ok

let group key label choices =
  P.Group.create
    ~id:(P.Group.Id.of_string key |> ok)
    ~label
    (Choice.Collection.create
       (List.map choices ~f:(fun (key, label, disabled) ->
          Choice.create ~id:(id key) ~label ~disabled () |> ok))
     |> ok)
  |> ok
;;

let options =
  P.Collection.grouped
    [ group
        "create"
        "Create"
        [ "research", "Research", false; "draft", "Drafting", false ]
    ; group "finish" "Finish" [ "review", "Review", false; "export", "Export", true ]
    ]
  |> ok
;;

let config selected =
  P.Config.create
    ~label:"Workspace capabilities"
    ~options
    ~selected
    ~search:Substring
    ~clearable:true
    ~placeholder:"Choose capabilities"
    ~search_placeholder:"Find a capability…"
    ()
  |> ok
;;

let capabilities window palette graph =
  let selected, request =
    B.state_machine0
      ~default_model:(P.Selection.multiple [] |> ok)
      ~equal:P.Selection.equal
      ~apply_action:(fun _ selected request ->
        P.Config.apply_request (config selected) request)
      graph
  in
  let notice, set_notice = B.state "Choose the capabilities for this workspace" graph in
  let open B.Let_syntax in
  let picker =
    Controller.create
      window
      ~config:
        (let%arr selected = selected in
         config selected)
      ~on_event:
        (let%arr request = request
         and set_notice = set_notice in
         function
         | P.Event.Selection_requested selection ->
           request (P.Selection_request.request selection)
         | Query_changed _ -> E.Ignore
         | Open_requested _ -> E.Ignore
         | Visibility (Snapshot _ | Changed (true, _)) -> E.Ignore
         | Visibility (Changed (false, _)) ->
           set_notice "Selection kept · search draft retained")
      graph
  in
  let%arr p = palette
  and picker = picker
  and selected = selected
  and request = request
  and notice = notice in
  let appearance =
    P.Appearance.create
      ~popup_width:360.
      ~max_height:300.
      ~empty_label:"No capabilities match your search"
      ~popup_style:
        (style
           [ Background (Background.solid (Palette.surface p))
           ; Foreground (Palette.foreground p)
           ; Border_color (Palette.border p)
           ])
      ~header_style:(style [ Foreground (Palette.muted p); Font_size 12. ])
      ~option_style:
        (Style.with_state_exn Style.empty Selected [ Foreground (Palette.accent p) ])
      ()
    |> ok
  in
  let rich key title detail =
    ( id key
    , P.Option_content.create
        (V.column
           ~style:(style [ Gap (px 3.); Padding (px 4.) ])
           [ Palette.text p title; Palette.text p ~size:12. ~muted:true detail ]) )
  in
  Palette.card
    p
    ~title:"A workspace, tailored to you"
    [ Palette.text
        p
        ~muted:true
        "Search grouped choices, select several, and keep your place as you explore."
    ; Controller.view
        picker
        ~appearance
        ~style:
          (style
             [ Width (px 320.)
             ; Height (px 44.)
             ; Radius 10.
             ; Border_color (Palette.border p)
             ; Background (Background.solid (Palette.surface p))
             ; Foreground (Palette.foreground p)
             ])
        ~options:
          [ rich "research" "Research" "Explore sources and gather context"
          ; rich "draft" "Drafting" "Turn ideas into a first draft"
          ]
        ~footer:
          (V.row
             ~style:(style [ Gap (px 8.); Padding (px 6.); Align_items Center ])
             [ Palette.button p "Clear capabilities" (request Clear)
             ; Palette.text p ~size:12. ~muted:true "Multiple choices welcome"
             ])
      |> ok
    ; Palette.text
        p
        ("Selected: "
         ^
         match P.Selection.ids selected with
         | [] -> "None yet"
         | ids -> String.concat ~sep:", " (List.map ids ~f:Choice.Id.to_string))
    ; Palette.text p ~muted:true notice
    ]
;;

let component window palette graph =
  let capabilities = capabilities window palette graph in
  let cases = Choice_picker_cases.component window palette graph in
  let open B.Let_syntax in
  let%arr capabilities = capabilities
  and cases = cases in
  V.column ~style:(style [ Gap (px 20.) ]) [ capabilities; cases ]
;;
