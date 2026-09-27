open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Pager = Gpuio_eio.Table_paging
module D = Gpuio.Table_data
module C = Gpuio.Table_column
module T = Gpuio.Table
module W = Gpuio_bonsai.Table
module View = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let id n = D.Id.of_string (Int.to_string n) |> ok
let col s = C.Id.of_string s |> ok

let start scope f =
  ignore
    (Scope.start scope ~f ~on_result:(fun result -> E.of_thunk (fun () -> ok result))
     |> ok
     : Scope.Task.t)
;;

let perform scope action =
  let promise, resolver = Eio.Promise.create () in
  ignore
    (Scope.start
       scope
       ~f:(fun () -> ())
       ~on_result:(fun result ->
         ok result;
         E.map action ~f:(Eio.Promise.resolve resolver))
     |> ok
     : Scope.Task.t);
  Eio.Promise.await promise
;;

type row =
  { number : int
  ; tool : string
  ; message : string
  }

module Load_behavior = struct
  type t =
    | Fail
    | Sample_rows
    | Until_cancelled
    | Pending of row Pager.Page.t Or_error.t Eio.Promise.t
    | Protected_pending of row Pager.Page.t Or_error.t Eio.Promise.t
end

let row n =
  { number = n
  ; tool = (if n mod 2 = 0 then "Search" else "Read file")
  ; message = sprintf "Result %06d · 日本語 · 👨‍👩‍👧‍👦" n
  }
;;

let data n = D.create (List.init n ~f:(fun n -> id n, row n)) |> ok

let initial_config =
  let columns =
    [ C.create ~id:(col "number") ~label:"EVENT" ~width:130. ~pin:Left ~sortable:true ()
      |> ok
    ; C.create ~id:(col "tool") ~label:"TOOL" ~width:180. () |> ok
    ; C.create ~id:(col "message") ~label:"RESULT" ~width:720. () |> ok
    ]
    |> C.Collection.create
    |> ok
  in
  T.Config.create
    ~columns
    ~label:"Agent event results"
    ~max_active_rows:32
    ~max_active_cells:96
    ~row_height:32.
    ()
  |> ok
;;

let with_height config height =
  T.Config.create
    ~columns:(T.Config.columns config)
    ~label:(T.Config.label config)
    ?sort:(T.Config.sort config)
    ~row_height:height
    ~max_active_rows:32
    ~max_active_cells:96
    ()
  |> ok
;;

let fill =
  Gpuio.Style.create_exn
    [ Grow 1.; Min_height (Gpuio.Length.px_exn 0.); Min_width (Gpuio.Length.px_exn 0.) ]
;;

let table_style ~light =
  Gpuio.Style.merge
    [ fill
    ; Gpuio.Style.create_exn
        [ Background
            (Gpuio.Background.solid
               (Gpuio.Color.rgb_exn (if light then 0xf4f7fc else 0x111b2b)))
        ; Foreground (Gpuio.Color.rgb_exn (if light then 0x17263c else 0xdce7f7))
        ; Border_color (Gpuio.Color.rgb_exn (if light then 0xcbd6e6 else 0x30445e))
        ; Border_width 1.
        ; Radius 12.
        ; Font_size 14.
        ]
    ]
;;

let run ~self_test ~background =
  App.run ~exit_on_last_window:(not self_test) (fun env app ->
    let cfg = B.Expert.Var.create initial_config in
    let style = B.Expert.Var.create (table_style ~light:false) in
    let observed = ref None
    and pager_ref = ref None in
    let observed_actions = ref None in
    let mounted = ref 0
    and unmounted = ref 0 in
    let load_behavior = ref Load_behavior.Fail in
    let on_request_ref = ref None in
    let actions = Event_actions.create () in
    let started = ref 0
    and active = ref 0
    and finished = ref 0 in
    let window =
      App.open_window
        app
        ~focus:(not background)
        ~title:"GPUIO — Table Lab"
        ~width:860.
        ~height:520.
        (fun window ->
           let pager =
             Pager.create
               ~scope:(App.Window.scope window)
               ~query:"initial"
               (data 100_000)
               ~before:End
               ~after:End
               ~load:(fun _ ->
                 Int.incr started;
                 Int.incr active;
                 Exn.protect
                   ~f:(fun () ->
                     match !load_behavior with
                     | Until_cancelled -> Eio.Fiber.await_cancel ()
                     | Pending result -> Eio.Promise.await result
                     | Protected_pending result ->
                       (* Bound this scripted cancellation delay so a failed
                          test can still shut down its window and workers. *)
                       Eio.Cancel.protect (fun () ->
                         Eio.Time.with_timeout_exn (Eio.Stdenv.clock env) 30. (fun () ->
                           Eio.Promise.await result))
                     | Fail ->
                       Or_error.error_string
                         "Demonstration: page unavailable; retry to continue"
                     | Sample_rows ->
                       Ok
                         { Pager.Page.rows = List.init 16 ~f:(fun n -> id n, row n)
                         ; next = End
                         })
                   ~finally:(fun () ->
                     Int.decr active;
                     Int.incr finished))
             |> ok
           in
           pager_ref := Some pager;
           let controls = Pager.controls pager in
           let on_request request =
             E.Many
               [ Event_actions.request
                   actions
                   ~generation:(Pager.snapshot pager).generation
                   request
               ; E.of_thunk (fun () ->
                   let config = B.Expert.Var.get cfg in
                   let columns = T.Config.columns config in
                   match request with
                   | T.Request.Resize widths ->
                     let columns =
                       List.map (C.Collection.to_list columns) ~f:(fun c ->
                         match List.Assoc.find widths (C.id c) ~equal:C.Id.equal with
                         | None -> c
                         | Some width -> C.with_width c width |> ok)
                       |> C.Collection.create
                            ~header_groups:(C.Collection.header_groups columns)
                       |> ok
                     in
                     B.Expert.Var.set cfg (T.Config.with_columns config columns |> ok)
                   | Move (column, before) ->
                     let columns = C.Collection.move columns ~column ~before |> ok in
                     B.Expert.Var.set cfg (T.Config.with_columns config columns |> ok)
                   | Sort (column, direction) ->
                     let snapshot = Pager.snapshot pager in
                     let rows =
                       D.to_alist snapshot.data
                       |> List.sort ~compare:(fun (_, a) (_, b) ->
                         match direction with
                         | Some T.Direction.Descending -> Int.compare b.number a.number
                         | Some Ascending | None -> Int.compare a.number b.number)
                     in
                     let source = D.reorder snapshot.data (List.map rows ~f:fst) |> ok in
                     B.Expert.Var.set
                       cfg
                       (T.Config.with_sort
                          config
                          (Option.map direction ~f:(fun direction ->
                             { T.Sort.column; direction }))
                        |> ok);
                     Pager.reset pager ~query:"sorted" source ~before:End ~after:End |> ok
                   | Select _ | Activate _ | Context _ | Copy _ -> ())
               ]
           in
           on_request_ref := Some on_request;
           fun graph ->
             let snapshot = Pager.value pager in
             let output =
               W.paged
                 snapshot
                 ~paging:(B.return controls)
                 ~config:(B.Expert.Var.value cfg)
                 ~style:(B.Expert.Var.value style)
                 ~on_request:(B.return on_request)
                 ~render_cell:(fun ~row:_ ~data ~column ~lifetime:_ graph ->
                   B.Edge.lifecycle
                     ~on_activate:(B.return (E.of_thunk (fun () -> Int.incr mounted)))
                     ~on_deactivate:(B.return (E.of_thunk (fun () -> Int.incr unmounted)))
                     graph;
                   B.map2 data column ~f:(fun row column ->
                     let text =
                       match C.Id.to_string (C.id column) with
                       | "number" -> sprintf "%06d" row.number
                       | "tool" -> row.tool
                       | _ -> row.message
                     in
                     W.Cell.text ~column:(C.id column) text))
                 graph
             in
             let event_actions =
               Event_actions.view
                 actions
                 ~snapshot
                 ~current:(fun () -> Pager.snapshot pager)
                 ~output
                 ~describe:(fun row ->
                   sprintf "Event %06d · %s\n%s" row.number row.tool row.message)
                 ~result_column:(col "message")
                 graph
             in
             let open B.Let_syntax in
             B.Edge.after_display
               (let%arr output = output
                and event_actions = event_actions in
                E.of_thunk (fun () ->
                  observed := Some (ok output);
                  observed_actions := Some event_actions))
               graph;
             let%arr output = output
             and snapshot = snapshot
             and event_actions = event_actions in
             let output = ok output in
             let selection =
               let event row = D.Row_ref.id row |> D.Id.to_string in
               match W.Output.selection output with
               | Empty -> "Select an event to inspect it"
               | Row row -> sprintf "Selected event %s" (event row)
               | Cell (row, column) ->
                 sprintf "Selected event %s · %s" (event row) (C.Id.to_string column)
               | Column column -> sprintf "Selected column %s" (C.Id.to_string column)
             in
             let footer =
               match snapshot.after with
               | Failed error ->
                 View.row
                   [ View.text (Error.to_string_hum error)
                   ; View.button
                       ~on_click:
                         (W.Paging.retry controls ~generation:snapshot.generation After)
                       "Retry"
                   ]
               | Ready | Loading | End ->
                 View.text
                   (sprintf
                      "%s rows · %d active cells · native scrolling"
                      (Int.to_string_hum (D.length snapshot.data))
                      (W.Output.active_cells output))
             in
             View.column
               ~style:
                 (Gpuio.Style.create_exn
                    [ Width (Gpuio.Length.percent_exn 100.)
                    ; Height (Gpuio.Length.percent_exn 100.)
                    ; Padding (Gpuio.Length.px_exn 20.)
                    ; Gap (Gpuio.Length.px_exn 12.)
                    ])
               [ View.text
                   ~style:(Gpuio.Style.create_exn [ Font_size 24. ])
                   "Agent event explorer"
               ; View.text "100,000 rows · pinned columns · resize, reorder and sort"
               ; W.Output.view output
               ; footer
               ; View.text selection
               ; event_actions
               ])
      |> ok
    in
    let pager = Option.value_exn !pager_ref in
    if self_test
    then
      start (App.scope app) (fun () ->
        let clock = Eio.Stdenv.clock env in
        let wait label f =
          try
            Eio.Time.with_timeout_exn clock 15. (fun () ->
              while not (f ()) do
                Eio.Time.sleep clock 0.01
              done)
          with
          | Eio.Time.Timeout -> failwith ("table example timeout: " ^ label)
        in
        let output () = Option.value_exn !observed in
        let sync f = perform (App.scope app) (E.of_thunk f) in
        let command f = perform (App.scope app) (f (W.Output.controller (output ()))) in
        let target n = W.Output.target (output ()) (id n) |> ok in
        let rec action_named name view =
          let description = Gpuio.View.Expert.describe view in
          if String.equal description.text name
          then Option.map description.on_click ~f:(fun click -> click ())
          else List.find_map description.children ~f:(action_named name)
        in
        let context_action () =
          Option.bind !observed_actions ~f:(action_named "Reveal result")
        in
        let open_context row =
          perform
            (App.scope app)
            ((Option.value_exn !on_request_ref) (T.Request.Context (Row row)));
          wait "context actions" (fun () -> Option.is_some (context_action ()));
          Option.value_exn (context_action ())
        in
        let anchor row offset =
          Option.exists
            (W.Output.viewport (output ()))
            ~f:(fun v ->
              Option.exists v.anchor ~f:(fun (key, within) ->
                Gpuio.Key.equal key (D.Expert.row_key row)
                && Float.(abs (within -. offset) < 0.6)))
        in
        let frame () =
          let p, r = Eio.Promise.create () in
          App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
            E.of_thunk (fun () -> Eio.Promise.resolve r ()))
          |> ok;
          Eio.Time.with_timeout_exn clock 5. (fun () -> Eio.Promise.await p)
        in
        wait "initial native cells" (fun () ->
          Option.exists !observed ~f:(fun o -> W.Output.active_cells o > 0));
        let middle = target 50_000 in
        command (fun c ->
          W.Controller.batch
            c
            [ Set_selection (Cell (middle, col "message")); Scroll_to (middle, 9.) ]
          |> ok);
        wait "native keyed offset" (fun () -> anchor middle 9.);
        assert (W.Output.active_cells (output ()) <= 96);
        let stale = W.Controller.reveal (W.Output.controller (output ())) (target 0) in
        sync (fun () -> B.Expert.Var.set cfg (with_height (B.Expert.Var.get cfg) 44.));
        frame ();
        wait "row-height anchor" (fun () -> anchor middle 9.);
        let stale_context = open_context middle in
        sync (fun () ->
          let snapshot = Pager.snapshot pager in
          let reordered =
            D.reorder snapshot.data (List.rev (D.keys snapshot.data)) |> ok
          in
          Pager.reset pager ~query:"reversed" reordered ~before:End ~after:End |> ok);
        frame ();
        wait "query reset native anchor" (fun () ->
          Int64.equal (Pager.snapshot pager).generation 1L
          && anchor middle 9.
          && Option.exists
               (W.Output.viewport (output ()))
               ~f:(fun v -> v.visible_first = 49_999));
        perform (App.scope app) stale;
        wait "query retires context" (fun () -> Option.is_none (context_action ()));
        perform (App.scope app) stale_context;
        frame ();
        assert (anchor middle 9.);
        ignore (open_context middle : unit E.t);
        perform (App.scope app) stale_context;
        frame ();
        assert (Option.is_some (context_action ()));
        perform
          (App.scope app)
          (Option.bind !observed_actions ~f:(action_named "Close event details")
           |> Option.value_exn);
        wait "current context closes" (fun () -> Option.is_none (context_action ()));
        let mounted_before = !mounted
        and unmounted_before = !unmounted in
        sync (fun () -> B.Expert.Var.set style (table_style ~light:true));
        frame ();
        assert (anchor middle 9.);
        assert (!mounted = mounted_before && !unmounted = unmounted_before);
        (match W.Output.selection (output ()) with
         | Cell (row, _) -> assert (D.Row_ref.equal row middle)
         | Empty | Row _ | Column _ -> failwith "style update reset table selection");
        sync (fun () -> B.Expert.Var.set style (table_style ~light:false));
        frame ();
        sync (fun () ->
          Pager.set
            pager
            ~key:(id 50_000)
            ~data:{ (row 50_000) with message = "streamed 日本語 👨‍👩‍👧‍👦" }
          |> ok);
        let rec has_copy view =
          let description = Gpuio.View.Expert.describe view in
          Option.exists description.table_cell ~f:(fun cell ->
            String.equal (T.Cell.copy_text cell) "streamed 日本語 👨‍👩‍👧‍👦")
          || List.exists description.children ~f:has_copy
        in
        wait "streamed retained cell" (fun () -> has_copy (W.Output.view (output ())));
        frame ();
        let removed_context = open_context middle in
        perform
          (App.scope app)
          (Option.bind !observed_actions ~f:(action_named "Close event details")
           |> Option.value_exn);
        wait "context closes before reopening" (fun () ->
          Option.is_none (context_action ()));
        ignore (open_context middle : unit E.t);
        perform (App.scope app) removed_context;
        frame ();
        assert (Option.is_some (context_action ()));
        sync (fun () ->
          let snapshot = Pager.snapshot pager in
          let index = D.index snapshot.data (id 50_000) |> Option.value_exn in
          let source = D.splice snapshot.data ~at:index ~remove:1 [] |> ok in
          Pager.reset pager ~query:"removed" source ~before:End ~after:End |> ok);
        wait "removed selection" (fun () ->
          match W.Output.selection (output ()) with
          | Empty -> true
          | Row _ | Column _ | Cell _ -> false);
        wait "removed row retires context" (fun () -> Option.is_none (context_action ()));
        perform (App.scope app) removed_context;
        frame ();
        assert (T.Selection.equal D.Row_ref.equal (W.Output.selection (output ())) Empty);
        sync (fun () ->
          Pager.reset pager ~query:"paged" (data 0) ~before:End ~after:(More None) |> ok);
        wait "native empty demand loads failure" (fun () ->
          match (Pager.snapshot pager).after with
          | Failed _ -> true
          | Ready | Loading | End -> false);
        frame ();
        assert (!started = 1 && !active = 0);
        sync (fun () -> load_behavior := Sample_rows);
        perform
          (App.scope app)
          (W.Paging.retry
             (Pager.controls pager)
             ~generation:(Pager.snapshot pager).generation
             After);
        wait "public retry paints rows" (fun () ->
          D.length (Pager.snapshot pager).data = 16
          && W.Output.active_cells (output ()) > 0);
        let page, deliver_page = Eio.Promise.create () in
        sync (fun () ->
          load_behavior := Pending page;
          Pager.reset
            pager
            ~query:"columns-while-loading"
            (Pager.snapshot pager).data
            ~before:End
            ~after:(More None)
          |> ok;
          Pager.request pager After |> ok);
        wait "page held during column changes" (fun () -> !active = 1);
        frame ();
        let row_four = target 4 in
        command (fun c ->
          W.Controller.batch
            c
            [ Set_selection (Cell (row_four, col "message")); Scroll_to (row_four, 7.) ]
          |> ok);
        wait "anchor before column updates" (fun () -> anchor row_four 7.);
        let generation = (Pager.snapshot pager).generation in
        (* Exercise the application request handler and actual native updates.
           Physical pointer/keyboard dispatch has separate native acceptance. *)
        let accept request =
          perform (App.scope app) ((Option.value_exn !on_request_ref) request)
        in
        accept (Resize [ col "tool", 240. ]);
        accept (Move (col "message", Some (col "tool")));
        frame ();
        wait "anchor observed after column updates" (fun () -> anchor row_four 7.);
        assert (!active = 1);
        assert (Int64.equal (Pager.snapshot pager).generation generation);
        Eio.Promise.resolve
          deliver_page
          (Ok
             { Pager.Page.rows = List.init 16 ~f:(fun n -> id (n + 16), row (n + 16))
             ; next = End
             });
        wait "page delivered after resize and reorder" (fun () ->
          !active = 0 && D.length (Pager.snapshot pager).data = 32);
        frame ();
        wait "anchor observed after page delivery" (fun () -> anchor row_four 7.);
        assert (W.Output.active_cells (output ()) <= 96);
        (match W.Output.selection (output ()) with
         | Cell (row, column) ->
           assert (D.Row_ref.equal row row_four && C.Id.equal column (col "message"))
         | Empty | Row _ | Column _ -> failwith "page delivery reset selection");
        let row_sixteen = target 16 in
        command (fun c ->
          W.Controller.batch
            c
            [ Set_selection (Cell (row_sixteen, col "message"))
            ; Scroll_to (row_sixteen, 9.)
            ]
          |> ok);
        wait "anchor before pending sort" (fun () -> anchor row_sixteen 9.);
        let obsolete, deliver_obsolete = Eio.Promise.create () in
        sync (fun () ->
          load_behavior := Protected_pending obsolete;
          Pager.reset
            pager
            ~query:"sort-while-loading"
            (Pager.snapshot pager).data
            ~before:End
            ~after:(More None)
          |> ok;
          Pager.request pager After |> ok);
        wait "producer pending before sort" (fun () -> !active = 1);
        accept (Sort (col "number", Some Descending));
        frame ();
        wait "sort follows keyed anchor" (fun () ->
          String.equal (Pager.snapshot pager).query "sorted"
          && anchor row_sixteen 9.
          && Option.exists
               (W.Output.viewport (output ()))
               ~f:(fun v -> v.visible_first = 15));
        assert (!active = 1);
        Eio.Promise.resolve
          deliver_obsolete
          (Ok { Pager.Page.rows = [ id 32, row 32 ]; next = End });
        wait "cancelled producer finishes after new sort" (fun () -> !active = 0);
        frame ();
        assert (D.length (Pager.snapshot pager).data = 32);
        assert (anchor row_sixteen 9.);
        (match W.Output.selection (output ()) with
         | Cell (row, _) -> assert (D.Row_ref.equal row row_sixteen)
         | Empty | Row _ | Column _ -> failwith "sorting reset stable selection");
        sync (fun () ->
          load_behavior := Until_cancelled;
          Pager.reset pager ~query:"closing" (data 0) ~before:End ~after:(More None) |> ok);
        wait "window-scoped producer" (fun () -> !active = 1);
        sync (fun () -> App.Window.close window);
        wait "window closes cells and cancels producer" (fun () ->
          App.Window.is_closed window && !active = 0 && !mounted = !unmounted);
        assert (!started = 5 && !finished = 5);
        Eio.Flow.copy_string
          "GPUIO_TABLE_PUBLIC_OK: 100k rows, bounded cells, keyed commands, anchors, \
           query retirement, streaming, selection repair, paging retry, column changes \
           during loading, late sort results and window cleanup\n"
          (Eio.Stdenv.stdout env);
        App.shutdown app)
    else App.on_reopen app (fun () -> E.Ignore))
;;

let () =
  let flag name = Array.exists (Sys.get_argv ()) ~f:(String.equal name) in
  run ~self_test:(flag "--self-test") ~background:(flag "--background")
;;
