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

let run ~self_test ~background =
  App.run ~exit_on_last_window:(not self_test) (fun env app ->
    let cfg = B.Expert.Var.create initial_config in
    let observed = ref None
    and pager_ref = ref None in
    let mounted = ref 0
    and unmounted = ref 0 in
    let fail = ref true
    and wait_load = ref false in
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
                     if !wait_load
                     then Eio.Fiber.await_cancel ()
                     else if !fail
                     then
                       Or_error.error_string
                         "Demonstration: page unavailable; retry to continue"
                     else
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
             E.of_thunk (fun () ->
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
           in
           fun graph ->
             let snapshot = Pager.value pager in
             let output =
               W.paged
                 snapshot
                 ~paging:(B.return controls)
                 ~config:(B.Expert.Var.value cfg)
                 ~style:(B.return fill)
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
             let open B.Let_syntax in
             B.Edge.after_display
               (let%arr output = output in
                E.of_thunk (fun () -> observed := Some (ok output)))
               graph;
             let%arr output = output
             and snapshot = snapshot in
             let output = ok output in
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
        frame ();
        assert (anchor middle 9.);
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
        sync (fun () ->
          let snapshot = Pager.snapshot pager in
          let index = D.index snapshot.data (id 50_000) |> Option.value_exn in
          let source = D.splice snapshot.data ~at:index ~remove:1 [] |> ok in
          Pager.reset pager ~query:"removed" source ~before:End ~after:End |> ok);
        wait "removed selection" (fun () ->
          match W.Output.selection (output ()) with
          | Empty -> true
          | Row _ | Column _ | Cell _ -> false);
        sync (fun () ->
          Pager.reset pager ~query:"paged" (data 0) ~before:End ~after:(More None) |> ok);
        wait "native empty demand loads failure" (fun () ->
          match (Pager.snapshot pager).after with
          | Failed _ -> true
          | Ready | Loading | End -> false);
        frame ();
        assert (!started = 1 && !active = 0);
        sync (fun () -> fail := false);
        perform
          (App.scope app)
          (W.Paging.retry
             (Pager.controls pager)
             ~generation:(Pager.snapshot pager).generation
             After);
        wait "public retry paints rows" (fun () ->
          D.length (Pager.snapshot pager).data = 16
          && W.Output.active_cells (output ()) > 0);
        sync (fun () ->
          wait_load := true;
          Pager.reset pager ~query:"closing" (data 0) ~before:End ~after:(More None) |> ok);
        wait "window-scoped producer" (fun () -> !active = 1);
        sync (fun () -> App.Window.close window);
        wait "window closes cells and cancels producer" (fun () ->
          App.Window.is_closed window && !active = 0 && !mounted = !unmounted);
        assert (!started = 3 && !finished = 3);
        Eio.Flow.copy_string
          "GPUIO_TABLE_PUBLIC_OK: 100k rows, bounded cells, keyed commands, anchors, \
           query retirement, streaming, selection repair, paging retry and window cleanup\n"
          (Eio.Stdenv.stdout env);
        App.shutdown app)
    else App.on_reopen app (fun () -> E.Ignore))
;;

let () =
  let flag name = Array.exists (Sys.get_argv ()) ~f:(String.equal name) in
  run ~self_test:(flag "--self-test") ~background:(flag "--background")
;;
