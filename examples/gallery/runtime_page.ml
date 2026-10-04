open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module App = Gpuio_eio.App

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

let component app window palette graph =
  let input_preview = Window_input_preview.component window palette graph in
  let selection_preview = Window_selection_preview.component window palette graph in
  let snapshot, set_snapshot = B.state None graph in
  let notice, set_notice = B.state "No native command requested" graph in
  let open B.Let_syntax in
  let refresh =
    B.map set_snapshot ~f:(fun set_snapshot ->
      E.bind
        (E.of_thunk (fun () -> App.diagnostics app, App.Window.snapshot window))
        ~f:(fun value -> set_snapshot (Some value)))
  in
  B.Edge.lifecycle ~on_activate:refresh graph;
  let%arr p = palette
  and snapshot = snapshot
  and refresh = refresh
  and notice = notice
  and set_notice = set_notice
  and input_preview = input_preview
  and selection_preview = selection_preview in
  let observe =
    E.bind (App.Window.command window Observe) ~f:(function
      | Ok value ->
        E.Many
          [ set_notice
              (sprintf
                 "Observed content: %.0f × %.0f"
                 value.content_width
                 value.content_height)
          ; refresh
          ]
      | Error error -> set_notice (Sexp.to_string_hum [%sexp (error : Window.Error.t)]))
  in
  let minimize =
    E.bind (App.Window.command window Minimize) ~f:(function
      | Ok _ -> set_notice "Minimize requested"
      | Error error -> set_notice (Sexp.to_string_hum [%sexp (error : Window.Error.t)]))
  in
  let choose =
    E.bind
      (Gpuio_eio.File_dialog.open_
         window
         ~config:
           (File_dialog.Open.create
              ~title:"Choose a gallery file"
              ~accept_label:"Choose"
              ()
            |> ok))
      ~f:(function
        | Ok None -> set_notice "File selection cancelled"
        | Ok (Some files) ->
          set_notice
            (sprintf
               "Selected %d file(s). No file contents were read."
               (List.length files))
        | Error error ->
          set_notice (Sexp.to_string_hum [%sexp (error : File_dialog.Error.t)]))
  in
  let resources =
    match snapshot with
    | None -> [ Palette.text p "Waiting for a resource snapshot…" ]
    | Some (d, geometry) ->
      [ Palette.text p (sprintf "Windows: %d" d.windows)
      ; Palette.text p (sprintf "Documents: %d" d.documents)
      ; Palette.text
          p
          (sprintf "Images: %d · Charts: %d · Canvases: %d" d.assets d.charts d.canvases)
      ; Palette.text p (sprintf "Scopes: %d · Tasks: %d" d.scopes.scopes d.scopes.tasks)
      ; Palette.text
          p
          (sprintf
             "Pending requests: %d · Queued commands: %d"
             d.pending_requests
             d.queued_commands)
      ; Palette.text
          p
          (sprintf
             "Registered source bytes: %d"
             (d.asset_source_bytes
              + d.document_source_bytes
              + d.chart_data_bytes
              + d.canvas_scene_bytes))
      ; Palette.text
          p
          (match geometry with
           | None -> "Window geometry not yet observed"
           | Some g ->
             sprintf
               "Content %.0f × %.0f · Outer %.0f × %.0f logical pixels"
               g.content_width
               g.content_height
               g.width
               g.height)
      ]
  in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ input_preview
    ; selection_preview
    ; Palette.card
        p
        ~title:"A window on the runtime"
        ((Palette.button p "Refresh resource counts" refresh :: resources)
         @ [ Palette.text
               p
               ~muted:true
               "These are application-wide registration counts and reserved source \
                bytes, not total process or GPU memory."
           ])
    ; Palette.card
        p
        ~title:"At home on the desktop"
        [ V.row
            ~style:(style [ Gap (px 12.); Wrap Wrap ])
            [ Palette.button p "Observe this window" observe
            ; Palette.button p "Minimize this window" minimize
            ; Palette.button p "Choose a file" choose
            ]
        ; Palette.text p notice
        ; Palette.text
            p
            ~muted:true
            "Use New window above to explore an independent workspace."
        ]
    ]
;;
