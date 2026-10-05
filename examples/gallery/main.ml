open Core
open Gpuio
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Page = Gpuio_gallery_model.Page
module Appearance = Gpuio_gallery_model.Appearance
module Theme_selection = Gpuio_gallery_model.Theme_selection

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let full = Length.percent_exn 100.

let component
      ~save_settings
      ~load_theme
      ~searchable
      ~edit_filters
      ~app
      ~desktop
      ~motion
      ~open_window
      ~page
      ~appearance
      ~scale
      ~custom_chrome
      ~window_snapshot
      window
      graph
  =
  let open B.Let_syntax in
  let effective_appearance =
    let%arr preference = B.Expert.Var.value appearance
    and snapshot = B.Expert.Var.value window_snapshot in
    Theme_selection.resolve
      preference
      ~native:(Option.map snapshot ~f:(fun s -> s.Window.Snapshot.appearance))
  in
  let page_value = B.Expert.Var.value page in
  let palette =
    let%arr a = effective_appearance
    and selection = B.Expert.Var.value appearance
    and s = B.Expert.Var.value scale in
    Palette.create ?profile:(Theme_selection.profile selection) a s
  in
  B.Edge.on_change
    (B.map palette ~f:Palette.theme)
    ~equal:Theme.equal
    ~callback:
      (B.return (fun theme -> E.of_thunk (fun () -> App.Window.set_theme window theme)))
    graph;
  let content =
    Pages.component
      ~load_theme
      ~theme_selection:appearance
      ~save_settings
      ~searchable
      ~edit_filters
      ~app
      ~desktop
      ~motion
      window
      ~page:page_value
      ~palette
      graph
  in
  let%arr page_value = page_value
  and p = palette
  and content = content
  and appearance_value = effective_appearance
  and appearance_preference = B.Expert.Var.value appearance
  and scale_value = B.Expert.Var.value scale
  and window_snapshot = B.Expert.Var.value window_snapshot in
  let action f = E.of_thunk f in
  let navigation =
    List.map Page.all ~f:(fun candidate ->
      Palette.button
        p
        ~selected:(Page.equal candidate page_value)
        (Page.title candidate)
        (action (fun () -> B.Expert.Var.set page candidate)))
  in
  let workspace =
    V.row
      ~style:
        (style
           [ Width full
           ; Height full
           ; Background (Background.solid (Palette.background p))
           ; Foreground (Palette.foreground p)
           ])
      [ V.column
          ~style:
            (style
               [ Width (px 236.)
               ; Height full
               ; Shrink 0.
               ; Padding (px 22.)
               ; Gap (px 14.)
               ; Background (Background.solid (Palette.surface p))
               ; Border_right_width 1.
               ; Border_color (Palette.border p)
               ])
          [ Palette.text p ~size:24. "GPUIO"
          ; Palette.text p ~muted:true "COMPONENT STUDIO"
          ; Presentation.separator (Palette.appearance p) ()
          ; V.column
              ~style:
                (style [ Grow 1.; Min_height (px 0.); Overflow_y Scroll; Gap (px 6.) ])
              navigation
          ; Palette.text p ~muted:true "Built with OCaml.\nRendered natively."
          ]
      ; V.column
          ~style:
            (style
               [ Grow 1.; Min_width (px 0.); Height full; Padding (px 30.); Gap (px 22.) ])
          [ V.row
              ~style:(style [ Align_items Center; Gap (px 10.) ])
              [ V.column
                  ~style:(style [ Grow 1.; Gap (px 8.) ])
                  [ Palette.text p ~size:30. (Page.title page_value)
                  ; Palette.text p ~muted:true (Page.description page_value)
                  ]
              ; Palette.button
                  p
                  (Appearance.label appearance_value)
                  (action (fun () ->
                     B.Expert.Var.set
                       appearance
                       (Theme_selection.choose
                          (B.Expert.Var.get appearance)
                          (Appearance.Preference.Explicit
                             (Appearance.toggle appearance_value)))))
              ; Palette.button
                  p
                  ~selected:
                    (Appearance.Preference.equal
                       (Theme_selection.preference appearance_preference)
                       System)
                  "Follow system"
                  (action (fun () ->
                     B.Expert.Var.set
                       appearance
                       (Theme_selection.choose
                          (B.Expert.Var.get appearance)
                          Appearance.Preference.System)))
              ; Palette.button
                  p
                  (Appearance.Scale.label scale_value)
                  (action (fun () ->
                     B.Expert.Var.set scale (Appearance.Scale.next scale_value)))
              ; Palette.button p "New window" (action open_window)
              ]
          ; (V.column
               ~key:(Key.of_string ("preview-" ^ Page.key page_value) |> ok)
               ~style:
                 (style
                    [ Grow 1.
                    ; Min_height (px 0.)
                    ; Overflow_y Scroll
                    ; Padding_right (px 8.)
                    ])
               [ content ]
             |> fun view ->
             V.with_accessibility
               view
               (Accessibility.create ~role:Group ~label:"Component preview" () |> ok)
             |> ok)
          ]
      ]
  in
  if not custom_chrome
  then workspace
  else (
    let fullscreen =
      Option.exists window_snapshot ~f:(fun s -> s.Window.Snapshot.fullscreen)
    in
    let title_style =
      style
        [ Min_height (px 48.)
        ; Gap (px 12.)
        ; Padding_right (px 16.)
        ; Background (Background.solid (Palette.surface p))
        ; Border_bottom_width 1.
        ; Border_color (Palette.border p)
        ]
    in
    let title_children =
      [ Palette.text p ~muted:true "GPUIO  /  COMPONENT STUDIO"
      ; V.column ~style:(style [ Grow 1. ]) []
      ; Palette.button
          p
          ~disabled:
            (not
               (Option.exists window_snapshot ~f:(fun s ->
                  s.Window.Snapshot.presentation.controls.fullscreen)))
          (if fullscreen then "Leave fullscreen" else "Fullscreen")
          (E.map (App.Window.command window Toggle_fullscreen) ~f:(fun _ -> ()))
      ]
      @
      match App.window_capabilities app, window_snapshot with
      | Some capabilities, Some snapshot ->
        [ V.window_controls
            ~backend:capabilities.backend
            ~snapshot
            ~style:(style [ Gap (px 6.); Shrink 0. ])
            ~button_style:
              (style
                 [ Padding (px 10.)
                 ; Radius 8.
                 ; Font_size 13.
                 ; Foreground (Palette.foreground p)
                 ; Background (Background.solid (Palette.surface p))
                 ]
               |> fun s ->
               Style.with_state_exn
                 s
                 Hovered
                 [ Background (Background.solid (Palette.border p)) ])
            ~on_minimize:(E.map (App.Window.command window Minimize) ~f:(fun _ -> ()))
            ~on_zoom:(E.map (App.Window.command window Zoom) ~f:(fun _ -> ()))
            ~on_close:(action (fun () -> App.Window.request_close window))
            ()
        ]
      | _ -> []
    in
    let title_bar =
      match App.window_capabilities app with
      | Some capabilities ->
        V.title_bar
          ~backend:capabilities.backend
          ~fullscreen
          ~style:title_style
          title_children
      | None ->
        (* The initial component precedes the native handshake. The first window
           observation supplies a backend before installing native regions. *)
        V.row ~style:title_style title_children
    in
    V.column
      ~style:(style [ Width full; Height full; Border_color (Palette.border p) ])
      [ title_bar; V.column ~style:(style [ Grow 1.; Min_height (px 0.) ]) [ workspace ] ])
;;

let check_catalogs () =
  let catalog = App.extension_catalog () |> ok in
  if not (List.exists catalog ~f:(Extension.Schema.equal Gpuio_example_counter.schema))
  then failwith "The gallery backend must provide the example.counter schema";
  let profiles = App.document_profile_catalog () |> ok in
  if
    not
      (List.exists
         profiles
         ~f:(Document.Profile.Schema.equal Gpuio_example_document.schema))
  then failwith "The gallery backend must provide the example.document schema"
;;

let main () =
  check_catalogs ();
  let background = Array.exists (Sys.get_argv ()) ~f:(String.equal "--background") in
  let custom_chrome =
    Array.exists (Sys.get_argv ()) ~f:(String.equal "--custom-chrome")
  in
  let trace_windows =
    Array.exists (Sys.get_argv ()) ~f:(String.equal "--trace-windows")
  in
  let trace_appearance =
    Array.exists (Sys.get_argv ()) ~f:(String.equal "--trace-window-appearance")
  in
  let startup_links =
    Array.filter_map (Sys.get_argv ()) ~f:(String.chop_prefix ~prefix:"--open-uri=")
    |> Array.to_list
  in
  let document_defaults =
    if Array.exists (Sys.get_argv ()) ~f:(String.equal "--document-defaults")
    then
      Document.Defaults.create
        ~selection_format:Markdown
        ~text_style:(Document.Style.create ~paragraph_gap_rem:1.8 () |> ok)
        ()
      |> ok
    else Document.Defaults.empty
  in
  App.run_desktop
    ~document_defaults
    Desktop_session.identity
    ~startup_links
    (fun env app ->
       let edit_filters = Edit_filter_preview.prepare () |> ok in
       let save_settings path contents =
         Gpuio_gallery_files.Settings_file.save
           Eio.Path.(Eio.Stdenv.fs env / Gpuio.File_path.to_string path)
           ~random:(Eio.Stdenv.secure_random env)
           contents
       in
       let load_theme path =
         Gpuio_gallery_files.Theme_file.load
           Eio.Path.(Eio.Stdenv.fs env / Gpuio.File_path.to_string path)
       in
       let desktop = Desktop_session.create app in
       let motion = B.Expert.Var.create Animation.Preference.System in
       let windows = ref [] in
       let serial = ref 0 in
       let rec open_window () =
         windows := List.filter !windows ~f:(fun w -> not (App.Window.is_closed w));
         if List.length !windows < 4
         then (
           incr serial;
           let page = B.Expert.Var.create Page.Presentation in
           let appearance = B.Expert.Var.create (Theme_selection.initial ()) in
           let scale = B.Expert.Var.create Appearance.Scale.Comfortable in
           let window_snapshot = B.Expert.Var.create None in
           let search_scope = Scope.child (App.scope app) ~name:"gallery-search" |> ok in
           let searchable =
             match
               Selectable_preview.create
                 ~scope:search_scope
                 ~clock:(Eio.Stdenv.mono_clock env)
             with
             | Ok searchable -> searchable
             | Error error ->
               Scope.cancel search_scope;
               Error.raise error
           in
           let window =
             App.open_window
               app
               ~theme:
                 (Palette.theme
                    (Palette.create Appearance.Dark Appearance.Scale.Comfortable))
               ~focus:(not background)
               ~chrome:(if custom_chrome then Custom else Standard)
               ~title:(sprintf "GPUIO · Component Studio %d" !serial)
               ~width:1120.
               ~height:820.
               (component
                  ~load_theme
                  ~save_settings
                  ~searchable
                  ~edit_filters
                  ~app
                  ~desktop
                  ~motion
                  ~open_window
                  ~page
                  ~appearance
                  ~scale
                  ~custom_chrome
                  ~window_snapshot)
             |> function
             | Ok window -> window
             | Error error ->
               Scope.cancel search_scope;
               Error.raise error
           in
           let window_number = !serial in
           let last_observed_appearance = ref None in
           App.Window.on_change window (fun snapshot ->
             E.of_thunk (fun () ->
               B.Expert.Var.set window_snapshot (Some snapshot);
               if
                 trace_appearance
                 && not
                      (Option.equal
                         Window.Appearance.equal
                         !last_observed_appearance
                         (Some snapshot.appearance))
               then (
                 last_observed_appearance := Some snapshot.appearance;
                 Eio.traceln
                   "GALLERY_NATIVE_APPEARANCE window=%d dark=%b"
                   window_number
                   (Window.Appearance.is_dark snapshot.appearance))));
           ignore
             (Scope.on_cancel (App.Window.scope window) (fun () ->
                Scope.cancel search_scope)
              |> ok
              : unit -> unit);
           if trace_windows
           then
             App.Window.set_close_handler window (fun reason ->
               E.of_thunk (fun () ->
                 Eio.traceln
                   "GALLERY_CLOSE_REQUEST window=%d reason=%s"
                   window_number
                   (Sexp.to_string (Window.Close_reason.sexp_of_t reason));
                 Window.Close_decision.Allow));
           windows := window :: !windows)
       in
       App.on_reopen app (fun () -> E.of_thunk open_window);
       open_window ();
       Desktop_session.ready desktop)
  |> function
  | Ok (App.Launch_outcome.Exited | Forwarded) -> ()
  | Error error -> raise_s [%sexp (error : Gpuio.Desktop.Error.t)]
;;

let () =
  if Array.exists (Sys.get_argv ()) ~f:(String.equal "--check-catalogs")
  then (
    check_catalogs ();
    Eio_main.run (fun env ->
      Gpuio_eio.Output.write
        (Eio.Stdenv.stdout env)
        "GALLERY_CATALOGS_PASS counter=1 document_profile=1\n"))
  else if Array.exists (Sys.get_argv ()) ~f:(String.equal "--print-info-plist")
  then (
    let package =
      Desktop_package.macos_info_plist
        Desktop_session.identity
        ~executable:"gpuio-studio"
        ~version:"0.1.0"
        ~build:"1"
      |> ok
    in
    Eio_main.run (fun env ->
      Gpuio_eio.Output.write (Eio.Stdenv.stdout env) (Desktop_package.contents package)))
  else main ()
;;
