open Core
open Gpuio
module App = Gpuio_eio.App
module Effect = Bonsai.Effect
module Bonsai = Bonsai.Cont
module Appearance = Gpuio_gallery_model.Appearance
module Theme_selection = Gpuio_gallery_model.Theme_selection

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
  let open Bonsai.Let_syntax in
  let effective_appearance =
    let%arr preference = Bonsai.Expert.Var.value appearance
    and snapshot = Bonsai.Expert.Var.value window_snapshot in
    Theme_selection.resolve
      preference
      ~native:(Option.map snapshot ~f:(fun s -> s.Window.Snapshot.appearance))
  in
  let page_value = Bonsai.Expert.Var.value page in
  let palette =
    let%arr a = effective_appearance
    and selection = Bonsai.Expert.Var.value appearance
    and s = Bonsai.Expert.Var.value scale in
    Palette.create ?profile:(Theme_selection.profile selection) a s
  in
  Bonsai.Edge.on_change
    (Bonsai.map palette ~f:Palette.theme)
    ~equal:Theme.equal
    ~callback:
      (Bonsai.return (fun theme ->
         Effect.of_thunk (fun () -> App.Window.set_theme window theme)))
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
  and appearance_preference = Bonsai.Expert.Var.value appearance
  and scale_value = Bonsai.Expert.Var.value scale
  and window_snapshot = Bonsai.Expert.Var.value window_snapshot in
  let action f = Effect.of_thunk f in
  let actions : Shell.Actions.t =
    { select_page =
        (fun candidate -> action (fun () -> Bonsai.Expert.Var.set page candidate))
    ; toggle_appearance =
        action (fun () ->
          Bonsai.Expert.Var.set
            appearance
            (Theme_selection.choose
               (Bonsai.Expert.Var.get appearance)
               (Appearance.Preference.Explicit (Appearance.toggle appearance_value))))
    ; follow_system =
        action (fun () ->
          Bonsai.Expert.Var.set
            appearance
            (Theme_selection.choose
               (Bonsai.Expert.Var.get appearance)
               Appearance.Preference.System))
    ; next_scale =
        action (fun () -> Bonsai.Expert.Var.set scale (Appearance.Scale.next scale_value))
    ; open_window = action open_window
    ; toggle_fullscreen =
        Effect.map (App.Window.command window Toggle_fullscreen) ~f:(fun _ -> ())
    ; minimize = Effect.map (App.Window.command window Minimize) ~f:(fun _ -> ())
    ; zoom = Effect.map (App.Window.command window Zoom) ~f:(fun _ -> ())
    ; close = action (fun () -> App.Window.request_close window)
    }
  in
  Shell.view
    ~palette:p
    ~content
    ~actions
    ~snapshot:
      { Shell.Snapshot.page = page_value
      ; appearance = appearance_value
      ; preference = appearance_preference
      ; scale = scale_value
      ; window_snapshot
      ; capabilities = App.window_capabilities app
      ; custom_chrome
      }
;;
