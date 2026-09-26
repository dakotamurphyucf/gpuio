open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Loader = Gpuio_eio.Tree_loading
module Tree = Gpuio_bonsai.Tree
module Data = Gpuio.Tree
module State = Gpuio.Tree_state
module Rows = Gpuio.Tree_rows
module View = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let style = Gpuio.Style.create_exn
let px = Gpuio.Length.px_exn

let config =
  Gpuio.Virtual_list.Config.create ~height:(Fixed 30.) ~overscan:90. ~max_active:32 ()
  |> ok
;;

let start scope f =
  ignore
    (Scope.start scope ~f ~on_result:(fun result -> E.of_thunk (fun () -> ok result))
     |> ok
     : Scope.Task.t)
;;

let perform scope ui_effect =
  let promise, resolver = Eio.Promise.create () in
  ignore
    (Scope.start
       scope
       ~f:(fun () -> ())
       ~on_result:(fun result ->
         ok result;
         E.map ui_effect ~f:(Eio.Promise.resolve resolver))
     |> ok
     : Scope.Task.t);
  Eio.Promise.await promise
;;

let component loader observed _window graph =
  let open B.Let_syntax in
  let mode, set_mode = B.state State.Mode.Multiple graph in
  let notice, set_notice = B.state "Select a file, or expand a directory." graph in
  let on_action =
    let%arr set_notice = set_notice in
    function
    | Gpuio.Tree_interaction.Action.None -> E.Ignore
    | Activate id -> set_notice ("Activated " ^ Data.Id.to_string id)
    | Move _ -> set_notice "Move proposal awaits application approval."
  in
  let output =
    Tree.component
      (Loader.value loader)
      ~config
      ~label:"Workspace files"
      ~style:
        (B.return
           (style [ Grow 1.; Min_height (px 0.); Width (Gpuio.Length.percent_exn 100.) ]))
      ~mode
      ~initial_expanded:(B.return [ Filesystem.root_id ])
      ~loading:(B.return (Loader.controls loader))
      ~on_action
      graph
  in
  B.Edge.after_display
    (let%arr output = output in
     E.of_thunk (fun () -> observed := Or_error.ok output))
    graph;
  let%arr output = output
  and mode = mode
  and set_mode = set_mode
  and notice = notice in
  let body, summary =
    match output with
    | Error error -> View.text (Error.to_string_hum error), "Tree unavailable"
    | Ok output ->
      ( Tree.Output.view output
      , sprintf
          "%d loaded · %d mounted · %d selected"
          (Data.length
             (Gpuio.Tree_loading.Snapshot.tree
                (Rows.source (Tree.Output.projection output))))
          (Tree.Output.active_rows output)
          (List.length (State.selected (Tree.Output.state output))) )
  in
  View.column
    ~style:
      (style
         [ Width (Gpuio.Length.percent_exn 100.)
         ; Height (Gpuio.Length.percent_exn 100.)
         ; Padding (px 20.)
         ; Gap (px 12.)
         ])
    [ View.text ~style:(style [ Font_size 24. ]) "Workspace explorer"
    ; View.text
        ~style:(style [ Foreground (Gpuio.Color.token_exn "muted") ])
        "A native managed tree · OCaml state · scoped Eio loading"
    ; View.row
        ~style:(style [ Gap (px 10.) ])
        [ View.button
            ~on_click:
              (set_mode (if State.Mode.equal mode Single then Multiple else Single))
            (if State.Mode.equal mode Single
             then "Switch to multiple selection"
             else "Switch to single selection")
        ; View.button
            ~on_click:
              (E.of_thunk (fun () -> Loader.reset loader (Filesystem.initial ()) |> ok))
            "Reload"
        ]
    ; body
    ; View.text summary
    ; View.text notice
    ]
;;

let run ~self_test =
  App.run (fun env app ->
    let clock = Eio.Stdenv.clock env in
    let scope = Scope.child (App.scope app) ~name:"filesystem tree" |> ok in
    let root = Eio.Stdenv.cwd env in
    let loader =
      Loader.create ~scope (Filesystem.initial ()) ~load:(Filesystem.load root) |> ok
    in
    let observed = ref None in
    let window =
      App.open_window
        app
        ~title:"GPUIO — Tree Lab"
        ~width:760.
        ~height:620.
        (component loader observed)
      |> ok
    in
    if self_test
    then
      start (App.scope app) (fun () ->
        let wait stage f =
          try
            Eio.Time.with_timeout_exn clock 20. (fun () ->
              while not (f ()) do
                Eio.Time.sleep clock 0.01
              done)
          with
          | Eio.Time.Timeout ->
            let state =
              Option.map !observed ~f:(fun output ->
                State.expanded (Tree.Output.state output), Tree.Output.viewport output)
            in
            failwithf
              "tree self-test timed out: %s; %s"
              stage
              (Sexp.to_string_hum
                 [%sexp
                   (state
                    : (Data.Id.t list * Gpuio.Virtual_list.Viewport.t option) option)])
              ()
        in
        let output () = Option.value_exn !observed in
        let loaded path =
          Option.exists !observed ~f:(fun output ->
            Data.find
              (Tree.Output.projection output
               |> Rows.source
               |> Gpuio.Tree_loading.Snapshot.tree)
              (Filesystem.id path |> ok)
            |> Option.is_some)
        in
        let target path =
          Tree.Output.target (output ()) (Filesystem.id path |> ok)
          |> Or_error.tag ~tag:("rendered filesystem target " ^ path)
          |> ok
        in
        let expand path =
          let output = output () in
          perform
            scope
            (Tree.Controller.set_expanded
               (Tree.Output.controller output)
               (target path)
               true);
          perform
            scope
            (Tree.Controller.reveal (Tree.Output.controller output) (target path))
        in
        wait "root directory" (fun () -> Option.is_some !observed && loaded "test");
        expand "test";
        wait "test directory" (fun () -> loaded "test/virtual_list");
        expand "test/virtual_list";
        wait "virtual_list directory" (fun () ->
          loaded "test/virtual_list/tree_widget_test.ml");
        let destination = "test/virtual_list/tree_widget_test.ml" in
        perform
          scope
          (Tree.Controller.reveal
             (Tree.Output.controller (output ()))
             ~focus:true
             (target destination));
        wait "native focus pin" (fun () ->
          let output = output () in
          let row =
            Rows.item_key (Tree.Output.projection output) (Filesystem.id destination |> ok)
          in
          Option.exists row ~f:(fun row ->
            Option.exists (Tree.Output.viewport output) ~f:(fun viewport ->
              List.mem viewport.pinned (Rows.Key.to_view_key row) ~equal:Gpuio.Key.equal)));
        assert (Tree.Output.active_rows (output ()) <= 32);
        assert (List.is_empty (State.selected (Tree.Output.state (output ()))));
        perform
          scope
          (Tree.Controller.set_selected
             (Tree.Output.controller (output ()))
             (target destination)
             true);
        wait "selection" (fun () ->
          State.is_selected
            (Tree.Output.state (output ()))
            (Filesystem.id destination |> ok));
        let stale =
          Tree.Controller.reveal
            (Tree.Output.controller (output ()))
            ~focus:true
            (target destination)
        in
        perform
          scope
          (E.of_thunk (fun () -> Loader.reset loader (Filesystem.initial ()) |> ok));
        perform scope stale;
        wait "reset" (fun () ->
          loaded "test" && List.is_empty (State.selected (Tree.Output.state (output ()))));
        assert (not (loaded "test/virtual_list"));
        let rendered, resolver = Eio.Promise.create () in
        App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
          E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
        |> ok;
        Eio.Time.with_timeout_exn clock 5. (fun () -> Eio.Promise.await rendered);
        Eio.Flow.copy_string
          "TREE_WIDGET_PASS eio_filesystem=true lazy_children=true native_focus_pin=true \
           bounded_rows=true selection=true reset=true\n"
          (Eio.Stdenv.stdout env);
        App.shutdown app))
;;

let () = run ~self_test:(Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test"))
