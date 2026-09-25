open Core
open Gpuio
module P = Presentation
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Input = Gpuio_eio.Text_input

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn
let full = Length.percent_exn 100.

let component ~phase ~observed ~editor_ref window graph =
  let dark, set_dark = B.state true graph in
  let invalid, set_invalid = B.state false graph in
  let checked, toggle = B.toggle ~default_model:true graph in
  let notice, set_notice = B.state "All changes stay in this local demo." graph in
  let phase = B.Expert.Var.value phase in
  let editor =
    Input.create
      window
      ~initial_text:"Aster workspace"
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Workspace name" () |> ok))
      graph
  in
  let open B.Let_syntax in
  B.Edge.after_display
    (let%arr phase = phase
     and editor = editor in
     E.of_thunk (fun () ->
       observed := phase;
       editor_ref := Some editor))
    graph;
  let%arr dark = dark
  and set_dark = set_dark
  and invalid = invalid
  and set_invalid = set_invalid
  and checked = checked
  and toggle = toggle
  and notice = notice
  and set_notice = set_notice
  and editor = editor
  and phase = phase in
  let dark = if phase < 0 then dark else phase % 2 = 0 in
  let invalid = if phase < 0 then invalid else phase = 1 in
  let p = if dark then P.Appearance.dark else P.Appearance.light in
  let canvas = Color.rgb_exn (if dark then 0x131821 else 0xf5f6fa) in
  let ink = Color.rgb_exn (if dark then 0xe5eaf2 else 0x202735) in
  let muted = Color.rgb_exn (if dark then 0xa3aebe else 0x606a79) in
  let accent = Color.rgb_exn (if dark then 0xa3b5ff else 0x4058b7) in
  let action ui_effect () = ui_effect in
  let button ui_effect text =
    View.button
      ~style:
        (style
           [ Padding (px 8.)
           ; Radius 6.
           ; Foreground canvas
           ; Background (Background.solid accent)
           ; White_space No_wrap
           ])
      ~on_click:(action ui_effect)
      text
  in
  let heading text = View.text ~style:(style [ Font_size 24.; Font_weight 600 ]) text in
  let details =
    P.description_list
      p
      [ P.Description.create
          ~key:(Key.of_int 0)
          ~term:"Runtime"
          ~definition:(P.badge p ~tone:Success "Native · ready")
      ; P.Description.create
          ~key:(Key.of_int 1)
          ~term:"Languages"
          ~definition:(View.text "OCaml + Rust")
      ; P.Description.create
          ~key:(Key.of_int 2)
          ~term:"Workspace"
          ~definition:(View.text "名前 · Design lab")
      ]
  in
  let field =
    Form.Field.create
      ~label:"Workspace name"
      ~help:"A name for your local conversations."
      ?error:
        (if invalid
         then Some "This name is already in use in the simulated example."
         else None)
      ~required:true
      ()
    |> ok
  in
  let field_view =
    Form.field
      field
      ~help_style:(style [ Foreground muted ])
      ~control:
        (Input.view
           ~style:
             (style
                [ Height (px 38.)
                ; Width full
                ; Border_width 1.
                ; Border_color muted
                ; Radius 6.
                ; Padding (px 6.)
                ])
           editor)
      ()
    |> ok
  in
  let settings =
    P.settings_group
      p
      ~title:"Workspace settings"
      ~description:"Small details, clear defaults."
      [ field_view
      ; Form.field
          (Form.Field.create
             ~label:"Live responses"
             ~help:"Keep incoming text streaming while you work."
             ()
           |> ok)
          ~help_style:(style [ Foreground muted ])
          ~control:
            (View.switch
               ~style:(style [ Foreground ink ])
               ~checked
               ~on_toggle:(action toggle)
               "Stream responses")
          ()
        |> ok
      ; View.row
          ~style:(style [ Gap (px 8.) ])
          [ button
              (set_invalid (not invalid))
              (if invalid then "Clear error" else "Show validation")
          ; P.link
              p
              ~on_click:
                (action
                   (set_notice "Documentation selected — no external service is needed."))
              "Documentation"
          ]
      ]
  in
  let assistant =
    P.message
      p
      ~author:"Aster"
      ~detail:"Local assistant"
      ~avatar:(P.badge p ~tone:Accent "A")
      ~footer:(P.marker p ~tone:Success "Ready for your review")
      (P.bubble
         p
         (View.text
            "I organized your workspace. The summary is below; your draft and selections \
             stay intact while you change the appearance."))
  in
  let result =
    P.tool_result
      p
      ~title:"Workspace summary"
      ~status:(P.badge p ~tone:Success "Complete")
      ~actions:
        (button (set_notice "This result is a reproducible local fixture.") "Details")
      ~footer:
        (P.attachment
           p
           ~name:"workspace-notes.md"
           ~detail:"Markdown · local fixture"
           ~preview:(P.tag p ~tone:Accent "MD")
           ~actions:
             (button (set_notice "Preview selected for workspace-notes.md.") "Preview")
           ())
      details
  in
  View.column
    ~style:
      (style
         [ Width full
         ; Height full
         ; Background (Background.solid canvas)
         ; Foreground ink
         ; Font_size 14.
         ; Padding (px 28.)
         ; Gap (px 20.)
         ; Overflow_y Scroll
         ])
    [ View.row
        ~style:(style [ Align_items Center; Gap (px 12.) ])
        [ View.column
            ~style:(style [ Grow 1.; Gap (px 6.) ])
            [ View.text
                ~style:(style [ Foreground accent; Font_size 11.; Font_weight 600 ])
                "GPUIO / COMPONENT STUDIO"
            ; heading "A workspace that feels at home"
            ]
        ; button
            (set_dark (not dark))
            (if dark then "Light appearance" else "Dark appearance")
        ]
    ; P.banner p ~tone:Accent ~live:Polite ~title:notice []
    ; View.row
        ~style:(style [ Gap (px 24.); Align_items Start ])
        [ View.column
            ~style:
              (style [ Width (Length.percent_exn 43.); Min_width (px 0.); Gap (px 16.) ])
            [ settings; P.group_box p ~header:(View.text "At a glance") [ details ] ]
        ; View.column
            ~style:(style [ Grow 1.; Basis (px 0.); Min_width (px 0.); Gap (px 16.) ])
            [ assistant
            ; result
            ; P.empty_state
                p
                ~title:"Room for your next idea"
                ~description:"Attachments and tool results appear here."
                ~actions:
                  (button
                     (set_notice "The sample action ran successfully.")
                     "Try an action")
                ()
            ]
        ]
    ; P.separator p ()
    ; P.status_bar
        p
        ~leading:(P.marker p ~tone:Success "Local demo · no network")
        ~trailing:(P.shortcut_label p [ "⌘"; "K · display only" ])
        ()
    ]
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  App.run (fun env app ->
    let phase = B.Expert.Var.create (-1)
    and observed = ref (-2)
    and editor_ref = ref None in
    let window =
      App.open_window
        app
        ~focus:true
        ~title:"GPUIO Component Studio"
        ~width:1040.
        ~height:860.
        (component ~phase ~observed ~editor_ref)
      |> ok
    in
    if self_test
    then
      Scope.start
        (App.scope app)
        ~f:(fun () ->
          let clock = Eio.Stdenv.clock env in
          Eio.Time.with_timeout_exn clock 20. (fun () ->
            let frame value =
              B.Expert.Var.set phase value;
              while !observed <> value do
                Eio.Time.sleep clock 0.005
              done;
              let promise, resolver = Eio.Promise.create () in
              App.Window.request_frame window ~on_rendered:(fun ~revision ->
                E.of_thunk (fun () -> Eio.Promise.resolve resolver revision))
              |> ok;
              Eio.Promise.await promise
            in
            let read () =
              let promise, resolver = Eio.Promise.create () in
              Scope.Expert.enqueue (App.Window.scope window) (fun () ->
                E.Expert.handle
                  (E.map
                     (Input.read_snapshot (Option.value_exn !editor_ref))
                     ~f:(Eio.Promise.resolve resolver)));
              match Eio.Promise.await promise with
              | Ok snapshot -> snapshot
              | Error error -> raise_s [%sexp (error : Text_input.Command_error.t)]
            in
            let first = frame 0 in
            let initial = read () in
            let error = frame 1 in
            let during_error = read () in
            let cleared = frame 2 in
            let after = read () in
            assert (Int64.(first < error && error < cleared));
            assert (Text_input.Snapshot.equal initial during_error);
            assert (Text_input.Snapshot.equal initial after)))
        ~on_result:(fun result ->
          E.of_thunk (fun () ->
            ok result;
            completed := true;
            App.Window.close window))
      |> ok
      |> fun (_ : Scope.Task.t) -> ());
  if self_test
  then (
    assert !completed;
    print_endline
      "GPUIO_PRESENTATION_PUBLIC_OK: theme, form validation, stable editor and shutdown")
;;
