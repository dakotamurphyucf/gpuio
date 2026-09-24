open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module Combo = Gpuio.Combobox
module Input = Gpuio.Text_input
module Controller = Gpuio_eio.Combobox
module View = Gpuio_bonsai.View

let id value = Gpuio.Choice.Id.of_string value |> Or_error.ok_exn

let options =
  [ "fast", "Fast responses"; "deep", "Detailed reasoning" ]
  |> List.map ~f:(fun (key, label) ->
    Gpuio.Choice.create ~id:(id key) ~label () |> Or_error.ok_exn)
  |> Gpuio.Choice.Collection.create
  |> Or_error.ok_exn
;;

let component ~self_test ~completed window graph =
  let open B.Let_syntax in
  let selected, set_selected = B.state None graph in
  let shown, set_shown = B.state true graph in
  let config =
    let%arr selected = selected in
    Combo.Config.create
      ~label:"Search reasoning mode"
      ~options
      ~selected
      ~auto_focus:true
      ()
    |> Or_error.ok_exn
  in
  let on_select =
    let%arr set_selected = set_selected in
    fun selection -> set_selected (Some (Combo.Selection.id selection))
  in
  let editor = Controller.create window ~config ~on_select graph in
  let observed = B.map editor ~f:Controller.snapshot in
  let sleep = B.Clock.sleep graph in
  let started = ref false in
  B.Edge.on_change
    observed
    ~equal:(Option.equal Input.Snapshot.equal)
    ~callback:
      (let%arr editor = editor
       and config = config
       and sleep = sleep
       and set_shown = set_shown in
       fun observation ->
         let open E.Let_syntax in
         let%bind begin_test =
           E.of_thunk (fun () ->
             if self_test && Option.is_some observation && not !started
             then (
               started := true;
               true)
             else false)
         in
         if not begin_test
         then E.Ignore
         else (
           let expect result =
             E.of_thunk (fun () ->
               Result.ok_or_failwith
                 (Result.map_error result ~f:(fun error ->
                    Sexp.to_string (Input.Command_error.sexp_of_t error))))
           in
           let%bind first =
             Controller.replace editor ~selection:End ~undo:Reset "é界" >>= expect
           in
           (* Construct a boundary value to test the public conditional command.
             Actual native selection dispatch is covered by native_controls. *)
           let intent =
             Combo.Expert.selection config ~id:(id "deep") ~snapshot:first
             |> Or_error.ok_exn
           in
           let%bind accepted =
             Controller.replace_if_unchanged editor intent "Detailed reasoning" >>= expect
           in
           let%bind () =
             E.of_thunk (fun () ->
               assert (String.equal (Input.Snapshot.text accepted) "Detailed reasoning"))
           in
           let%bind stale =
             Controller.replace_if_unchanged editor intent "must not replace"
           in
           let%bind () =
             E.of_thunk (fun () ->
               assert (
                 Result.equal
                   Input.Snapshot.equal
                   Input.Command_error.equal
                   stale
                   (Error Stale_revision)))
           in
           let%bind undone = Controller.command editor Undo >>= expect in
           let%bind () =
             E.of_thunk (fun () ->
               assert (String.equal (Input.Snapshot.text undone) "é界"))
           in
           let%bind () = set_shown false in
           let%bind () = sleep (Time_ns.Span.of_sec 0.1) in
           let%bind stale = Controller.focus editor in
           let%bind () =
             E.of_thunk (fun () ->
               assert (
                 Result.equal
                   Input.Snapshot.equal
                   Input.Command_error.equal
                   stale
                   (Error Stale_editor)))
           in
           E.of_thunk (fun () ->
             completed := true;
             App.Window.close window)))
    graph;
  let%arr editor = editor
  and selected = selected
  and shown = shown in
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 16.); Gap (Gpuio.Length.px_exn 12.) ])
    ([ View.text "Search and choose a reasoning mode" ]
     @ (if shown
        then
          [ Controller.view
              ~style:(Gpuio.Style.create_exn [ Height (Gpuio.Length.px_exn 36.) ])
              editor
          ]
        else [])
     @ [ View.text
           ("Selected: "
            ^ Option.value_map selected ~default:"none" ~f:Gpuio.Choice.Id.to_string)
       ; View.text
           ("Query: "
            ^ Option.value_map
                (Controller.snapshot editor)
                ~default:""
                ~f:Input.Snapshot.text)
       ; View.button ~on_click:(E.of_thunk (fun () -> App.Window.close window)) "Close"
       ])
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  App.run (fun _ app ->
    App.open_window
      app
      ~title:"GPUIO editable choices"
      ~width:440.
      ~height:320.
      (component ~self_test ~completed)
    |> Or_error.ok_exn
    |> fun (_ : App.Window.t) -> ());
  if self_test
  then (
    assert !completed;
    print_endline
      "GPUIO_COMBOBOX_PUBLIC_OK: native mount/observation, exact conditional \
       replacement, stale revision, undo, unmount and close")
;;
