open Core
open Gpuio
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Probe = Gpuio_performance_probe

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

module W = Gpuio_bonsai.Table
module T = Gpuio.Table
module D = Gpuio.Table_data
module Column = Gpuio.Table_column
module Cache = Page_cache

let column_id i = Column.Id.of_string (Int.to_string i) |> ok

let config =
  let columns =
    List.init Cache.columns ~f:(fun i ->
      Column.create
        ~id:(column_id i)
        ~label:(sprintf "Column %02d" i)
        ~width:128.
        ~pin:(if i = 0 then Left else Unpinned)
        ()
      |> ok)
    |> Column.Collection.create
    |> ok
  in
  T.Config.create
    ~columns
    ~label:"Logical paged qualification table"
    ~row_height:32.
    ~overscan:64.
    ~max_active_rows:32
    ~max_active_cells:2048
    ()
  |> ok
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

let component ~wall_clock ~source ~command ~events ~observed ~materialized _window graph =
  let open B.Let_syntax in
  let output =
    W.component
      (B.Expert.Var.value source)
      ~config:(B.return config)
      ~style:(B.return (style [ Width (px 1168.); Height (px 720.); Shrink 0. ]))
      ~render_row_presentation:(fun ~row:_ ~data ~lifetime:_ graph ->
        B.Edge.after_display
          (let%arr data = data in
           E.of_thunk (fun () ->
             Option.iter data ~f:(fun row ->
               materialized := Set.add !materialized (Cache.Row.number row))))
          graph;
        B.return (Ok Table_presentation.Row.empty))
      ~render_cell:(fun ~row:_ ~data ~column ~lifetime:_ _graph ->
        let%arr data = data
        and column = column in
        let index = Column.id column |> Column.Id.to_string |> Int.of_string in
        W.Cell.text
          ~column:(Column.id column)
          (Option.value_map data ~default:"Loading" ~f:(fun row ->
             Cache.Row.cell row ~column:index)))
      graph
  in
  B.Edge.after_display
    (let%arr output = output in
     E.of_thunk (fun () -> observed := Some (ok output)))
    graph;
  let%arr output = output
  and command = B.Expert.Var.value command in
  let sequence, request = command in
  let instance = Probe.instance ~sequence request |> ok in
  let on_event = function
    | Extension.Event.Data event ->
      E.of_thunk (fun () ->
        assert (Queue.is_empty events);
        Queue.enqueue events event)
    | Failed error ->
      E.of_thunk (fun () ->
        failwith (Sexp.to_string_hum (Extension.Error.sexp_of_t error)))
    | Mounted | Command_completed _ -> E.Ignore
  in
  V.column
    ~style:
      (style
         [ Padding (px 16.)
         ; Gap (px 8.)
         ; Width (px 1200.)
         ; Height (px 800.)
         ; Background (Background.solid (Color.rgb_exn 0xf5f6f8))
         ; Foreground (Color.rgb_exn 0x182030)
         ])
    [ V.text "GPUIO qualification · 100k logical rows / 64 columns"
    ; W.Output.view (ok output)
    ; (if wall_clock
       then V.row ~style:(style [ Width (px 1.); Height (px 1.) ]) []
       else V.extension ~on_event instance)
    ]
;;

let run ~smoke ~background ~wall_clock =
  let rows = if smoke then 1030 else 100_000 in
  let idle_seconds = if smoke then 2. else 60. in
  App.run ~exit_on_last_window:false (fun env app ->
    let clock = Eio.Stdenv.clock env in
    let scope = App.scope app in
    let cache =
      ref (Cache.create ~rows |> ok |> fun t -> Cache.prepare t ~target:0 |> ok)
    in
    let source = B.Expert.Var.create (Cache.data !cache) in
    let command = B.Expert.Var.create (1L, Probe.Command.Begin) in
    let events = Queue.create () in
    let observed = ref None in
    let materialized = ref Int.Set.empty in
    let window =
      App.open_window
        app
        ~focus:(not background)
        ~title:"GPUIO · Paged table qualification"
        ~width:1200.
        ~height:800.
        (component ~wall_clock ~source ~command ~events ~observed ~materialized)
      |> ok
    in
    let active_state () =
      Option.map (App.Window.snapshot window) ~f:(fun s -> s.active)
    in
    let previous_active = ref (active_state ()) in
    let active_changes = ref 0 in
    App.Window.on_change window (fun snapshot ->
      E.of_thunk (fun () ->
        let active = Some snapshot.active in
        if not (Option.equal Bool.equal active !previous_active)
        then Int.incr active_changes;
        previous_active := active));
    let work () =
      let emit name sexp =
        Eio.Flow.copy_string
          ("GPUIO_TABLE_PERF " ^ name ^ " " ^ Sexp.to_string sexp ^ "\n")
          (Eio.Stdenv.stdout env)
      in
      let wait ?(label = "condition") f =
        try
          Eio.Time.with_timeout_exn clock 15. (fun () ->
            while not (f ()) do
              Eio.Time.sleep clock 0.005
            done)
        with
        | Eio.Time.Timeout -> failwith ("Timed out: " ^ label)
      in
      let next_event () =
        wait ~label:"native measurement event" (fun () -> not (Queue.is_empty events));
        Queue.dequeue_exn events
      in
      let sequence = ref 1L in
      let send request =
        sequence := Int64.succ !sequence;
        perform
          scope
          (E.of_thunk (fun () -> B.Expert.Var.set command (!sequence, request)))
      in
      let begin_native_phase name =
        match next_event () with
        | Probe.Event.Begun capture_ns ->
          emit "begin" [%sexp (name : string), (capture_ns : int64)]
        | event -> failwith (Sexp.to_string_hum (Probe.Event.sexp_of_t event))
      in
      let finish_native_phase name =
        send Probe.Command.Finish;
        let finished = next_event () in
        let counts =
          match finished with
          | Probe.Event.Finished { counts; _ } -> counts
          | event -> failwith (Sexp.to_string_hum (Probe.Event.sexp_of_t event))
        in
        assert (List.length counts = 5);
        emit "finish" [%sexp (name : string), (finished : Probe.Event.t)];
        List.iteri counts ~f:(fun metric expected_count ->
          let offset = ref 0 in
          let total = ref None in
          let count = ref 0L in
          let previous = ref None in
          let more = ref true in
          while !more do
            send (Probe.Command.Buckets { metric; offset = !offset });
            let response = next_event () in
            (match response with
             | Probe.Event.Buckets page ->
               assert (page.metric = metric && page.offset = !offset);
               assert (page.total >= !offset && List.length page.values <= 128);
               Option.iter !total ~f:(fun old -> assert (old = page.total));
               total := Some page.total;
               List.iter page.values ~f:(fun (value, samples) ->
                 assert (Int64.(value >= 0L && samples > 0L));
                 Option.iter !previous ~f:(fun old -> assert (Int64.(old < value)));
                 previous := Some value;
                 count := Int64.(!count + samples));
               let length = List.length page.values in
               assert (length > 0 || !offset = page.total);
               offset := !offset + length;
               assert (!offset <= page.total);
               more := !offset < page.total
             | event -> failwith (Sexp.to_string_hum (Probe.Event.sexp_of_t event)));
            emit "buckets" [%sexp (name : string), (response : Probe.Event.t)]
          done;
          assert (Int64.equal !count expected_count));
        finished
      in
      let phase_start = ref None in
      let begin_phase name =
        if wall_clock
        then (
          Eio.Time.sleep clock 2.;
          phase_start := Some (Eio.Time.Mono.now (Eio.Stdenv.mono_clock env));
          emit "wall-begin" [%sexp (name : string)])
        else begin_native_phase name
      in
      let finish_phase name =
        if wall_clock
        then (
          let elapsed =
            Mtime.span
              (Option.value_exn !phase_start)
              (Eio.Time.Mono.now (Eio.Stdenv.mono_clock env))
          in
          let elapsed_ns = Mtime.Span.to_uint64_ns elapsed in
          phase_start := None;
          emit "wall-finish" [%sexp (name : string), (elapsed_ns : int64)];
          None)
        else Some (finish_native_phase name)
      in
      let output () = Option.value_exn !observed in
      let current_viewport () = W.Output.viewport (output ()) in
      let viewport () = Option.value_exn (current_viewport ()) in
      let stage = ref "startup" in
      let trace ?(always = false) name =
        if smoke || always
        then
          Eio.Flow.copy_string
            ("GPUIO_TABLE_TRACE "
             ^ name
             ^ " "
             ^ Sexp.to_string
                 [%sexp
                   (!stage : string)
                 , (App.Window.snapshot window : Window.Snapshot.t option)
                 , (W.Output.column_viewport (output ()) : T.Column_viewport.t option)]
             ^ "\n")
            (Eio.Stdenv.stdout env)
      in
      let frame () =
        let promise, resolver = Eio.Promise.create () in
        App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
          E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
        |> ok;
        try Eio.Time.with_timeout_exn clock 10. (fun () -> Eio.Promise.await promise) with
        | exn ->
          trace ~always:true "frame-failed";
          raise_s
            [%sexp "Native frame acknowledgement failed", (!stage : string), (exn : exn)]
      in
      let peak_rows = ref 0 in
      let peak_cells = ref 0 in
      let check_budgets () =
        let o = output () in
        let v = viewport () in
        assert ((not v.budget_exhausted) && not (W.Output.budget_exhausted o));
        let active = W.Output.active_rows o in
        let cells = W.Output.active_cells o in
        assert (active <= 32 && cells <= 2048);
        peak_rows := Int.max !peak_rows active;
        peak_cells := Int.max !peak_cells cells
      in
      let target n = W.Output.target (output ()) (Cache.id n) |> ok in
      let preload n =
        (* Scoped Eio worker; all model mutation is delivered back as an effect.
           Payload generation is deterministic, not a disk/network measurement. *)
        let next = Cache.prepare !cache ~target:n |> ok in
        perform
          scope
          (E.of_thunk (fun () ->
             cache := next;
             B.Expert.Var.set source (Cache.data next)))
      in
      let move n =
        stage := sprintf "row %d" n;
        preload n;
        let row = target n in
        perform scope (W.Controller.scroll_to (W.Output.controller (output ())) row |> ok);
        wait ~label:(sprintf "table row %d" n) (fun () ->
          Option.exists (current_viewport ()) ~f:(fun v ->
            (Option.exists v.anchor ~f:(fun (anchor, _) ->
               Key.equal anchor (D.Expert.row_key row))
             || (v.at_end && v.visible_first <= n && v.visible_last > n))
            && Set.mem !materialized n));
        frame ();
        wait ~label:"table viewport after frame" (fun () ->
          Option.is_some (current_viewport ()));
        check_budgets ();
        let v = viewport () in
        for i = v.visible_first to v.visible_last - 1 do
          assert (Option.is_some (D.find (Cache.data !cache) (Cache.id i) |> Option.join));
          assert (Set.mem !materialized i)
        done;
        v
      in
      emit
        "config"
        [%sexp
          (rows : int)
        , (Cache.columns : int)
        , (Cache.page_size : int)
        , (Cache.max_pages : int)
        , (smoke : bool)
        , (background : bool)
        , (idle_seconds : float)];
      wait ~label:"initial table viewport" (fun () ->
        Option.exists !observed ~f:(fun o -> Option.is_some (W.Output.viewport o)));
      begin_phase "history";
      trace ~always:true "history-start";
      let selected = target 0 in
      let selection = T.Selection.Cell (selected, column_id 0) in
      perform scope (W.Controller.select (W.Output.controller (output ())) selection);
      let checkpoint = ref 10_000 in
      let progress direction count =
        if count >= !checkpoint
        then (
          trace ~always:true direction;
          checkpoint := !checkpoint + 10_000)
      in
      let forward = ref Int.Set.empty in
      let next = ref 0 in
      while !next < rows do
        let v = move !next in
        for i = v.visible_first to v.visible_last - 1 do
          forward := Set.add !forward i
        done;
        progress "forward" (Set.length !forward);
        if v.at_end then next := rows else next := Int.max (!next + 1) (v.visible_last - 1)
      done;
      checkpoint := 10_000;
      let backward = ref Int.Set.empty in
      next := rows - 1;
      while !next >= 0 do
        let v = move !next in
        for i = v.visible_first to v.visible_last - 1 do
          backward := Set.add !backward i
        done;
        progress "backward" (Set.length !backward);
        if v.visible_first = 0
        then next := -1
        else
          next
          := Int.max 0 (v.visible_first - Int.max 1 (v.visible_last - v.visible_first - 1))
      done;
      assert (Set.length !forward = rows && Set.length !backward = rows);
      assert (Set.length !materialized = rows);
      assert (T.Selection.equal D.Row_ref.equal (W.Output.selection (output ())) selection);
      let seen_columns = ref Int.Set.empty in
      for i = 0 to Cache.columns - 1 do
        stage := sprintf "column %d" i;
        trace "column-before";
        perform
          scope
          (W.Controller.scroll_to_column (W.Output.controller (output ())) (column_id i));
        (try
           wait ~label:(sprintf "column %d" i) (fun () ->
             Option.exists
               (W.Output.column_viewport (output ()))
               ~f:(fun viewport ->
                 List.exists (T.Column_viewport.columns viewport) ~f:(fun c ->
                   Column.Id.equal (T.Column_viewport.Column.id c) (column_id i)
                   && T.Column_viewport.Column.fully_visible c)))
         with
         | exn ->
           emit
             "column-failed"
             [%sexp
               (i : int)
             , (W.Output.column_viewport (output ()) : T.Column_viewport.t option)];
           raise exn);
        frame ();
        let columns = W.Output.column_viewport (output ()) |> Option.value_exn in
        List.iter (T.Column_viewport.columns columns) ~f:(fun c ->
          seen_columns
          := Set.add
               !seen_columns
               (T.Column_viewport.Column.id c |> Column.Id.to_string |> Int.of_string));
        check_budgets ()
      done;
      assert (Set.length !seen_columns = Cache.columns);
      let reveal_row = rows / 2 in
      preload reveal_row;
      perform
        scope
        (W.Controller.reveal
           (W.Output.controller (output ()))
           ~column:(column_id 63)
           (target reveal_row));
      wait ~label:"middle cell reveal" (fun () ->
        Option.exists (current_viewport ()) ~f:(fun v ->
          v.visible_first <= reveal_row && v.visible_last > reveal_row)
        && Set.mem !materialized reveal_row);
      frame ();
      check_budgets ();
      assert (T.Selection.equal D.Row_ref.equal (W.Output.selection (output ())) selection);
      wait ~label:"revealed horizontal cell" (fun () ->
        Option.exists
          (W.Output.column_viewport (output ()))
          ~f:(fun viewport ->
            List.exists (T.Column_viewport.columns viewport) ~f:(fun c ->
              Column.Id.equal (T.Column_viewport.Column.id c) (column_id 63)
              && T.Column_viewport.Column.fully_visible c)));
      emit "interactions" [%sexp (0 : int), (0 : int), (reveal_row : int), (63 : int)];
      ignore (finish_phase "history" : Probe.Event.t option);
      emit
        "history"
        [%sexp
          (Set.length !forward : int)
        , (Set.length !backward : int)
        , (Set.length !materialized : int)
        , (Set.length !seen_columns : int)
        , (!peak_rows : int)
        , (!peak_cells : int)
        , (Cache.peak_loaded_rows !cache : int)
        , (Cache.loads !cache : int)
        , (Cache.evictions !cache : int)
        , (Cache.loaded_rows !cache : int)];
      (* Give final selection/reveal and observer work a named settling period
         before the measured idle interval, as required by qualification. *)
      Eio.Time.sleep clock 2.;
      if not wall_clock then send Probe.Command.Begin;
      begin_phase "idle";
      let idle_active_before = active_state () in
      let idle_changes_before = !active_changes in
      Eio.Time.sleep clock idle_seconds;
      let idle = finish_phase "idle" in
      emit
        "idle-activation"
        [%sexp
          (idle_active_before : bool option)
        , (active_state () : bool option)
        , (!active_changes - idle_changes_before : int)];
      (match idle with
       | Some (Probe.Event.Finished { counts = draws :: _; _ }) ->
         assert (Int64.equal draws 0L)
       | None -> assert wall_clock
       | _ -> assert false);
      App.Window.close window;
      wait (fun () -> (App.diagnostics app).windows = 0);
      emit "cleanup" (App.Diagnostics.sexp_of_t (App.diagnostics app));
      emit "complete" [%sexp (rows : int)];
      App.shutdown app
    in
    ignore
      (Scope.start scope ~f:work ~on_result:(fun result ->
         E.of_thunk (fun () -> ok result))
       |> ok
       : Scope.Task.t))
;;

let () =
  let flag name = Array.exists (Sys.get_argv ()) ~f:(String.equal name) in
  run
    ~smoke:(flag "--smoke")
    ~background:(flag "--background")
    ~wall_clock:(flag "--wall-clock")
;;
