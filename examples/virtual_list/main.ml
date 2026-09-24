open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Pager = Gpuio_eio.List_paging
module Collection = Gpuio.List_collection
module Virtual_list = Gpuio_bonsai.Virtual_list
module View = Gpuio_bonsai.View
module Editor = Gpuio_eio.Text_input

let style = Gpuio.Style.create_exn
let px = Gpuio.Length.px_exn
let key id = Gpuio.Key.of_string (Int.to_string id) |> Or_error.ok_exn
let row id = id, sprintf "Message %d\nA variable-height conversation row." id
let latest = 300

let config =
  Virtual_list.Config.create
    ~height:(Estimated 110.)
    ~overscan:200.
    ~max_active:32
    ~scroll:Follow_tail_when_at_end
    ()
  |> Or_error.ok_exn
;;

let start scope f =
  ignore
    (Scope.start scope ~f ~on_result:(fun result ->
       E.of_thunk (fun () -> Or_error.ok_exn result))
     |> Or_error.ok_exn
     : Scope.Task.t)
;;

(* Test orchestration enters effects through the UI scheduler, just as native
   events and producer completions do. This helper does not run inside Bonsai. *)
let perform scope ui_effect =
  let promise, resolver = Eio.Promise.create () in
  ignore
    (Scope.start
       scope
       ~f:(fun () -> ())
       ~on_result:(fun result ->
         Or_error.ok_exn result;
         E.map ui_effect ~f:(Eio.Promise.resolve resolver))
     |> Or_error.ok_exn
     : Scope.Task.t);
  Eio.Promise.await promise
;;

let component ~pager ~stream_response ~observed window graph =
  let open B.Let_syntax in
  (* Application preference lifetime is independent of the transient row graph. *)
  let starred, set_starred = B.state Int.Set.empty graph in
  let snapshot = Pager.value pager in
  let paging = B.return (Pager.controls pager) in
  let output =
    Virtual_list.paged
      (module Int)
      snapshot
      ~paging
      ~row_key:key
      ~config
      ~style:(B.return (style [ Height (px 380.); Width (px 650.); Shrink 0. ]))
      ~render_row:(fun ~key:id ~data ~lifetime:_ graph ->
        let expanded, set_expanded = B.state false graph in
        let%arr id = id
        and data = data
        and expanded = expanded
        and set_expanded = set_expanded
        and starred = starred
        and set_starred = set_starred in
        let is_starred = Set.mem starred id in
        View.column
          ~style:
            (style
               [ Padding (px 10.)
               ; Gap (px 6.)
               ; Min_height (px (if expanded then 160. else 100.))
               ; Shrink 0.
               ])
          [ View.text data
          ; View.row
              ~style:(style [ Gap (px 12.) ])
              [ View.button
                  ~on_click:
                    (set_starred
                       (if is_starred then Set.remove starred id else Set.add starred id))
                  (if is_starred then "Unstar (persistent)" else "Star (persistent)")
              ; View.button
                  ~on_click:(set_expanded (not expanded))
                  (if expanded then "Collapse" else "Expand (this visit)")
              ]
          ; View.text (if expanded then "Transient details reset after eviction." else "")
          ])
      graph
  in
  B.Edge.after_display
    (let%arr output = output in
     E.of_thunk (fun () -> observed := Some (Or_error.ok_exn output)))
    graph;
  let editor =
    Editor.create
      window
      ~config:
        (B.return
           (Gpuio.Text_input.Config.create
              ~mode:Multiline
              ~label:"Prompt"
              ~placeholder:"Type a prompt; Enter streams a simulated reply"
              ~submit_on_enter:true
              ~min_rows:2
              ~max_rows:4
              ()
            |> Or_error.ok_exn))
      ~on_submit:
        (B.return (fun submission ->
           E.of_thunk (fun () ->
             stream_response (Gpuio.Text_input.Submission.text submission))))
      graph
  in
  let%arr output = output
  and snapshot = snapshot
  and paging = paging
  and editor = editor in
  let output = Or_error.ok_exn output in
  let before = Sexp.to_string_hum (Pager.Status.sexp_of_t snapshot.before) in
  View.column
    ~style:(style [ Padding (px 16.); Gap (px 10.) ])
    [ View.text
        (sprintf
           "%d loaded messages / %d active rows / history: %s"
           (Collection.length snapshot.items)
           (Virtual_list.Output.active_rows output)
           before)
    ; Virtual_list.Output.view output
    ; View.row
        ~style:(style [ Gap (px 12.) ])
        [ View.button
            ~on_click:
              (Virtual_list.Paging.request paging ~generation:snapshot.generation Before)
            "Load older"
        ; View.button
            ~on_click:
              (Virtual_list.Paging.retry paging ~generation:snapshot.generation Before)
            "Retry older"
        ; View.button
            ~on_click:
              (Virtual_list.Controller.jump_to_latest
                 (Virtual_list.Output.controller output))
            "Jump to latest"
        ; View.button
            ~on_click:(E.of_thunk (fun () -> stream_response "A simulated response"))
            "Stream response"
        ]
    ; Editor.view ~style:(style [ Width (px 650.); Height (px 76.) ]) editor
    ]
;;

let run ~self_test =
  App.run (fun env app ->
    let clock = Eio.Stdenv.clock env in
    let conversation =
      Scope.child (App.scope app) ~name:"conversation" |> Or_error.ok_exn
    in
    let observed = ref None in
    let page_started = ref false in
    let release_page, release = Eio.Promise.create () in
    let initial =
      Collection.of_alist (module Int) (List.init 200 ~f:(fun i -> row (101 + i)))
      |> Or_error.ok_exn
    in
    let pager =
      Pager.create
        ~scope:conversation
        initial
        ~before:(More (Some "101"))
        ~after:End
        ~load:(fun request ->
          page_started := true;
          if self_test then Eio.Promise.await release_page else Eio.Time.sleep clock 0.4;
          match Pager.Request.direction request with
          | Before ->
            Ok { Pager.Page.rows = List.init 100 ~f:(fun i -> row (i + 1)); next = End }
          | After -> Ok { Pager.Page.rows = []; next = End })
      |> Or_error.ok_exn
    in
    let stream_done = ref false in
    let streaming = ref false in
    let chunks =
      Gpuio_eio.Stream.create ~scope:conversation ~capacity:16 ~on_batch:(fun chunks ->
        E.of_thunk (fun () ->
          Pager.set pager ~key:latest ~data:(List.last_exn chunks) |> Or_error.ok_exn))
      |> Or_error.ok_exn
    in
    let stream_response prompt =
      if not !streaming
      then (
        streaming := true;
        stream_done := false;
        start conversation (fun () ->
          Exn.protect
            ~f:(fun () ->
              for i = 1 to 20 do
                Eio.Time.sleep clock 0.02;
                Gpuio_eio.Stream.push
                  chunks
                  (sprintf
                     "%s\n%s"
                     prompt
                     (String.concat ~sep:" " (List.init i ~f:(fun _ -> "streaming"))))
                |> Or_error.ok_exn
              done;
              stream_done := true)
            ~finally:(fun () -> streaming := false)))
    in
    let window =
      App.open_window
        app
        ~title:"GPUIO — Managed conversation"
        ~width:700.
        ~height:650.
        (component ~pager ~stream_response ~observed)
      |> Or_error.ok_exn
    in
    if self_test
    then
      start (App.scope app) (fun () ->
        let wait f =
          Eio.Time.with_timeout_exn clock 15. (fun () ->
            while not (f ()) do
              Eio.Time.sleep clock 0.01
            done)
        in
        let output () = Option.value_exn !observed in
        let viewport () = Virtual_list.Output.viewport (output ()) in
        let at_end () = Option.exists (viewport ()) ~f:(fun v -> v.at_end) in
        wait (fun () -> Option.is_some !observed && at_end ());
        assert (Virtual_list.Output.active_rows (output ()) <= 32);
        perform
          conversation
          (E.of_thunk (fun () -> Pager.request pager Before |> Or_error.ok_exn));
        wait (fun () -> !page_started);
        perform
          conversation
          (Virtual_list.Controller.scroll_to
             (Virtual_list.Output.controller (output ()))
             ~offset:17.
             200
           |> Or_error.ok_exn);
        wait (fun () ->
          Option.exists (viewport ()) ~f:(fun v ->
            Option.exists v.anchor ~f:(fun (k, _) -> Gpuio.Key.equal k (key 200))));
        perform conversation (E.of_thunk (fun () -> stream_response "Offscreen response"));
        wait (fun () -> !stream_done);
        Eio.Promise.resolve release ();
        wait (fun () -> Collection.length (Pager.items pager) = 300);
        wait (fun () -> Option.is_some (viewport ()));
        let anchor, offset = Option.value_exn (Option.value_exn (viewport ())).anchor in
        assert (Gpuio.Key.equal anchor (key 200));
        assert (Float.(abs (offset -. 17.) < 0.1));
        assert (Scope.is_active conversation);
        assert (
          String.is_prefix
            (Option.value_exn (Collection.find (Pager.items pager) latest))
            ~prefix:"Offscreen response");
        perform
          conversation
          (Virtual_list.Controller.jump_to_latest
             (Virtual_list.Output.controller (output ())));
        wait at_end;
        assert (Virtual_list.Output.active_rows (output ()) <= 32);
        let rendered, resolver = Eio.Promise.create () in
        App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
          E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
        |> Or_error.ok_exn;
        Eio.Time.with_timeout_exn clock 5. (fun () -> Eio.Promise.await rendered);
        Eio.Flow.copy_string
          "MANAGED_LIST_PASS paging=true offscreen_stream=true anchor=true tail=true \
           bounded_rows=true\n"
          (Eio.Stdenv.stdout env);
        App.shutdown app))
;;

let () = run ~self_test:(Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test"))
