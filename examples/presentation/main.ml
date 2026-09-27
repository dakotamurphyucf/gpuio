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

let component ~assets ~avatar_status ~phase ~observed ~editor_ref ~rating_ref window graph
  =
  let dark, set_dark = B.state true graph in
  let invalid, set_invalid = B.state false graph in
  let checked, toggle = B.toggle ~default_model:true graph in
  let show_loading, toggle_loading = B.toggle ~default_model:true graph in
  let animate_loading, toggle_animation = B.toggle ~default_model:true graph in
  let avatar_mode, set_avatar_mode = B.state Avatar_mode.Initials graph in
  let avatar_state, set_avatar_state = B.state "Initials only" graph in
  let rating, inject_rating =
    B.state_machine0
      ~default_model:Rating_action.initial
      ~apply_action:(fun _ model action -> Rating_action.apply model action)
      graph
  in
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
     and editor = editor
     and rating = rating
     and inject_rating = inject_rating in
     E.of_thunk (fun () ->
       observed := phase;
       editor_ref := Some editor;
       rating_ref := Some (rating, inject_rating)))
    graph;
  let%arr dark = dark
  and set_dark = set_dark
  and invalid = invalid
  and set_invalid = set_invalid
  and checked = checked
  and toggle = toggle
  and show_loading = show_loading
  and toggle_loading = toggle_loading
  and animate_loading = animate_loading
  and toggle_animation = toggle_animation
  and assets = B.Expert.Var.value assets
  and avatar_mode = avatar_mode
  and set_avatar_mode = set_avatar_mode
  and avatar_state = avatar_state
  and set_avatar_state = set_avatar_state
  and rating = rating
  and inject_rating = inject_rating
  and notice = notice
  and set_notice = set_notice
  and editor = editor
  and phase = phase in
  let dark = if phase < 0 then dark else phase % 2 = 0 in
  let invalid = if phase < 0 then invalid else phase = 1 in
  let show_loading = if phase < 0 then show_loading else phase < 2 in
  let animate_loading = if phase < 0 then animate_loading else phase = 0 in
  let avatar_mode =
    if phase < 0
    then avatar_mode
    else (
      match phase % 3 with
      | 1 -> Avatar_mode.Picture
      | 2 -> Invalid_picture
      | _ -> Initials)
  in
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
  let indicator kind label =
    let config = Loading.Config.create ~kind ~label ~animated:animate_loading () |> ok in
    View.loading
      ~config
      ~style:
        (style
           [ Foreground muted
           ; Width
               (px
                  (match kind with
                   | Spinner -> 24.
                   | Skeleton | Shimmer -> 220.))
           ; Height
               (px
                  (match kind with
                   | Spinner -> 24.
                   | Skeleton | Shimmer -> 12.))
           ])
      ()
  in
  let loading =
    P.group_box
      p
      ~header:(View.text "Background work")
      [ View.row
          ~style:(style [ Gap (px 8.) ])
          [ button
              toggle_loading
              (if show_loading then "Hide indicators" else "Show indicators")
          ; button
              toggle_animation
              (if animate_loading then "Static indicators" else "Animate indicators")
          ]
      ; View.column
          ~style:
            (style
               [ Gap (px 12.)
               ; Min_height (px 72.)
               ; Visibility (if show_loading then Visible else Hidden)
               ])
          [ indicator Skeleton "Preparing content"
          ; indicator Shimmer "Loading preview"
          ; View.row
              ~style:(style [ Align_items Center; Gap (px 8.) ])
              [ indicator Spinner "Loading workspace"
              ; View.text "Preparing your workspace…"
              ]
          ]
      ]
  in
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
  let avatar_config =
    let asset =
      Option.bind assets ~f:(fun assets ->
        match avatar_mode with
        | Picture -> Some assets.Avatar_assets.image
        | Invalid_picture -> Some assets.invalid
        | Initials -> None)
    in
    Avatar.Config.create
      ?asset
      ~fallback:(Avatar.Fallback.create "A" |> ok)
      ~description:(Image.Description.label "Aster avatar" |> ok)
      ()
  in
  let on_avatar_change (state : Image.State.t) =
    E.Many
      [ E.of_thunk (fun () -> avatar_status := Some state)
      ; set_avatar_state
          (match state with
           | Loading -> "Loading avatar"
           | Ready _ -> "Avatar image ready"
           | Failed _ -> "Image unavailable — showing initials")
      ]
  in
  let feedback =
    P.group_box
      p
      ~header:(View.text "Response feedback")
      [ Form.field
          (Form.Field.create
             ~label:"Response quality"
             ~help:"Use arrow keys or choose a star. Select that star again to clear."
             ()
           |> ok)
          ~help_style:(style [ Foreground muted ])
          ~control:
            (View.rating
               ~config:rating
               ~style:(style [ Foreground accent ])
               ~on_request:(fun request -> inject_rating (Rating_action.Request request))
               ())
          ()
        |> ok
      ; View.text
          (sprintf
             "Rating: %d of %d"
             (Rating.Config.value rating)
             (Rating.Config.maximum rating))
      ; View.row
          ~style:(style [ Gap (px 6.); Wrap Wrap ])
          [ button
              (inject_rating Toggle_read_only)
              (if Rating.Config.is_read_only rating
               then "Allow rating edits"
               else "Read-only rating")
          ; button
              (inject_rating Toggle_disabled)
              (if Rating.Config.is_disabled rating
               then "Enable rating"
               else "Disable rating")
          ]
      ]
  in
  let assistant =
    P.message
      p
      ~author:"Aster"
      ~detail:"Local assistant"
      ~avatar:
        (View.avatar
           ~on_change:on_avatar_change
           ~style:
             (style
                [ Width (px 36.)
                ; Height (px 36.)
                ; Foreground canvas
                ; Background (Background.solid accent)
                ])
           avatar_config)
      ~footer:feedback
      (P.bubble
         p
         (View.column
            ~style:(style [ Gap (px 10.) ])
            [ View.text
                "I organized your workspace. Your draft stays intact as you explore the \
                 components."
            ; View.row
                ~style:(style [ Gap (px 6.); Wrap Wrap ])
                [ button (set_avatar_mode Picture) "Use image"
                ; button (set_avatar_mode Invalid_picture) "Simulate failure"
                ; button (set_avatar_mode Initials) "Use initials"
                ]
            ; View.text
                ~style:(style [ Foreground muted; Font_size 12. ])
                (match avatar_mode with
                 | Initials -> "Initials only"
                 | Picture | Invalid_picture -> avatar_state)
            ]))
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
            [ settings; loading ]
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
  let content_check =
    Array.exists (Sys.get_argv ()) ~f:(String.equal "--content-check")
  in
  let completed = ref false in
  App.run (fun env app ->
    let phase = B.Expert.Var.create (-1)
    and observed = ref (-2)
    and editor_ref = ref None
    and assets = B.Expert.Var.create None
    and avatar_status = ref None
    and rating_ref = ref None in
    let window =
      App.open_window
        app
        ~focus:true
        ~title:"GPUIO Component Studio"
        ~width:(if content_check then 440. else 1040.)
        ~height:860.
        (if content_check
         then Content_cases.component
         else component ~assets ~avatar_status ~phase ~observed ~editor_ref ~rating_ref)
      |> ok
    in
    Avatar_assets.load env app assets;
    if self_test
    then
      Scope.start
        (App.scope app)
        ~f:(fun () ->
          let clock = Eio.Stdenv.clock env in
          Eio.Time.with_timeout_exn clock 20. (fun () ->
            while Option.is_none (B.Expert.Var.get assets) do
              Eio.Time.sleep clock 0.005
            done;
            let await_avatar predicate =
              while not (Option.exists !avatar_status ~f:predicate) do
                Eio.Time.sleep clock 0.005
              done
            in
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
            await_avatar (function
              | Ready _ -> true
              | Loading | Failed _ -> false);
            let during_error = read () in
            let cleared = frame 2 in
            await_avatar (function
              | Failed Invalid_data -> true
              | Loading | Ready _ | Failed _ -> false);
            let after = read () in
            let (_ : int64) = frame 3 in
            assert (Int64.(first < error && error < cleared));
            assert (Text_input.Snapshot.equal initial during_error);
            assert (Text_input.Snapshot.equal initial after);
            Scope.Expert.enqueue (App.Window.scope window) (fun () ->
              let _, inject = Option.value_exn !rating_ref in
              E.Expert.handle
                (E.Many
                   (List.init 4 ~f:(fun _ ->
                      inject (Rating_action.Request Rating.Request.increase)))));
            while Rating.Config.value (fst (Option.value_exn !rating_ref)) <> 5 do
              Eio.Time.sleep clock 0.005
            done;
            Scope.Expert.enqueue (App.Window.scope window) (fun () ->
              let _, inject = Option.value_exn !rating_ref in
              E.Expert.handle
                (E.Many
                   [ inject (Request (Rating.Request.toggle 5 |> ok))
                   ; inject Toggle_read_only
                   ; inject (Request Rating.Request.increase)
                   ]));
            while not (Rating.Config.is_read_only (fst (Option.value_exn !rating_ref))) do
              Eio.Time.sleep clock 0.005
            done;
            assert (Rating.Config.value (fst (Option.value_exn !rating_ref)) = 0)))
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
      "GPUIO_PRESENTATION_PUBLIC_OK: theme, form validation, avatar \
       ready/failure/fallback, rating reducer bursts/read-only, stable editor and \
       shutdown")
;;
