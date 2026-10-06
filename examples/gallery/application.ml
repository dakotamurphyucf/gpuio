open Core
open Gpuio
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Effect = Bonsai.Effect
module Bonsai = Bonsai.Cont
module Page = Gpuio_gallery_model.Page
module Appearance = Gpuio_gallery_model.Appearance
module Theme_selection = Gpuio_gallery_model.Theme_selection

let ok = Or_error.ok_exn

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

let run () =
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
       let motion = Bonsai.Expert.Var.create Animation.Preference.System in
       let windows = ref [] in
       let serial = ref 0 in
       let rec open_window () =
         windows := List.filter !windows ~f:(fun w -> not (App.Window.is_closed w));
         if List.length !windows < 4
         then (
           incr serial;
           let page = Bonsai.Expert.Var.create Page.Presentation in
           let appearance = Bonsai.Expert.Var.create (Theme_selection.initial ()) in
           let scale = Bonsai.Expert.Var.create Appearance.Scale.Comfortable in
           let window_snapshot = Bonsai.Expert.Var.create None in
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
               (Component.component
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
             Effect.of_thunk (fun () ->
               Bonsai.Expert.Var.set window_snapshot (Some snapshot);
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
               Effect.of_thunk (fun () ->
                 Eio.traceln
                   "GALLERY_CLOSE_REQUEST window=%d reason=%s"
                   window_number
                   (Sexp.to_string (Window.Close_reason.sexp_of_t reason));
                 Window.Close_decision.Allow));
           windows := window :: !windows)
       in
       App.on_reopen app (fun () -> Effect.of_thunk open_window);
       open_window ();
       Desktop_session.ready desktop)
  |> function
  | Ok (App.Launch_outcome.Exited | Forwarded) -> ()
  | Error error -> raise_s [%sexp (error : Gpuio.Desktop.Error.t)]
;;
