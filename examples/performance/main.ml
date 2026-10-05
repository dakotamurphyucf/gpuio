open Core
open Gpuio
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module L = Gpuio_bonsai.Virtual_list
module C = List_collection
module Probe = Gpuio_performance_probe

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let key i = Key.of_string (Int.to_string i) |> ok

let body i =
  let bytes = [| 64; 256; 2048 |].(i % 3) in
  let prefix = sprintf "Row %05d · 世界 " i in
  prefix
  ^ String.init
      (bytes - String.length prefix)
      ~f:(fun n -> if n % 6 = 5 then ' ' else 'x')
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
    L.component
      (module Int)
      (B.Expert.Var.value source)
      ~row_key:key
      ~config:
        (L.Config.create ~height:(Estimated 72.) ~overscan:200. ~max_active:32 () |> ok)
      ~style:(B.return (style [ Width (px 1168.); Height (px 720.); Shrink 0. ]))
      ~render_row:(fun ~key:id ~data ~lifetime:_ graph ->
        B.Edge.after_display
          (let%arr id = id in
           E.of_thunk (fun () -> materialized := Set.add !materialized id))
          graph;
        let%arr data = data in
        V.column
          ~style:
            (style
               [ Padding (px 8.)
               ; Min_height (px 28.)
               ; Shrink 0.
               ; Font_size 14.
               ; Line_height (px 20.)
               ])
          [ V.text data ])
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
    [ V.text "GPUIO qualification · variable-height history"
    ; L.Output.view (ok output)
    ; (if wall_clock
       then V.row ~style:(style [ Width (px 1.); Height (px 1.) ]) []
       else V.extension ~on_event instance)
    ]
;;

let run ~smoke ~background ~wall_clock =
  let rows = if smoke then 96 else 10_000 in
  let idle_seconds = if smoke then 2. else 60. in
  App.run ~exit_on_last_window:false (fun env app ->
    let clock = Eio.Stdenv.clock env in
    let scope = App.scope app in
    let source =
      B.Expert.Var.create
        (C.of_alist (module Int) (List.init rows ~f:(fun i -> i, body i)) |> ok)
    in
    let command = B.Expert.Var.create (1L, Probe.Command.Begin) in
    let events = Queue.create () in
    let observed = ref None in
    let materialized = ref Int.Set.empty in
    let window =
      App.open_window
        app
        ~focus:(not background)
        ~title:"GPUIO · Performance qualification"
        ~width:1200.
        ~height:800.
        (component ~wall_clock ~source ~command ~events ~observed ~materialized)
      |> ok
    in
    let work () =
      let emit name sexp =
        Eio.Flow.copy_string
          ("GPUIO_PERF " ^ name ^ " " ^ Sexp.to_string sexp ^ "\n")
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
      let current_viewport () = L.Output.viewport (output ()) in
      let viewport () = Option.value_exn (current_viewport ()) in
      let frame () =
        let promise, resolver = Eio.Promise.create () in
        App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
          E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
        |> ok;
        Eio.Time.with_timeout_exn clock 10. (fun () -> Eio.Promise.await promise)
      in
      let peak_active = ref 0 in
      let move target =
        perform
          scope
          (L.Controller.scroll_to (L.Output.controller (output ())) target |> ok);
        wait ~label:(sprintf "row %d viewport" target) (fun () ->
          Option.exists (current_viewport ()) ~f:(fun v ->
            (Option.exists v.anchor ~f:(fun (anchor, _) -> Key.equal anchor (key target))
             || (v.at_end && v.visible_first <= target && v.visible_last > target))
            && Set.mem !materialized target));
        frame ();
        wait ~label:"painted viewport" (fun () -> Option.is_some (current_viewport ()));
        let v = viewport () in
        assert ((not v.budget_exhausted) && not (L.Output.budget_exhausted (output ())));
        let active = L.Output.active_rows (output ()) in
        assert (active <= 32);
        peak_active := Int.max !peak_active active;
        v
      in
      emit
        "config"
        [%sexp (rows : int), (smoke : bool), (background : bool), (idle_seconds : float)];
      wait ~label:"initial list viewport" (fun () ->
        Option.exists !observed ~f:(fun o -> Option.is_some (L.Output.viewport o)));
      begin_phase "history";
      let forward = ref Int.Set.empty in
      let target = ref 0 in
      while !target < rows do
        let v = move !target in
        for i = v.visible_first to v.visible_last - 1 do
          forward := Set.add !forward i
        done;
        if v.at_end
        then target := rows
        else target := Int.max (!target + 1) (v.visible_last - 1)
      done;
      let backward = ref Int.Set.empty in
      for target = rows - 1 downto 0 do
        let v = move target in
        for i = v.visible_first to v.visible_last - 1 do
          backward := Set.add !backward i
        done
      done;
      assert (Set.length !forward = rows && Set.length !backward = rows);
      assert (Set.length !materialized = rows);
      let before_growth = (viewport ()).visible_last in
      emit "growth-start" (Virtual_list.Viewport.sexp_of_t (viewport ()));
      perform
        scope
        (E.of_thunk (fun () ->
           let collection = B.Expert.Var.get source in
           (* Forty explicit lines make row zero taller than the viewport. A smaller
           change can legitimately leave the same partially visible final row. *)
           B.Expert.Var.set
             source
             (C.set
                collection
                ~key:0
                ~data:
                  (body 0
                   ^ String.concat (List.init 40 ~f:(fun _ -> "\nA growing response.")))
              |> ok)));
      (try
         wait ~label:"grown row layout" (fun () ->
           Option.exists (current_viewport ()) ~f:(fun v ->
             v.visible_last < before_growth))
       with
       | exn ->
         emit
           "growth-failed"
           [%sexp (current_viewport () : Virtual_list.Viewport.t option)];
         raise exn);
      frame ();
      assert ((viewport ()).visible_first = 0);
      assert ((viewport ()).visible_last = 1);
      emit "growth-complete" (Virtual_list.Viewport.sexp_of_t (viewport ()));
      ignore (finish_phase "history" : Probe.Event.t option);
      emit
        "history"
        [%sexp
          (Set.length !forward : int), (Set.length !backward : int), (!peak_active : int)];
      if not wall_clock then send Probe.Command.Begin;
      begin_phase "idle";
      Eio.Time.sleep clock idle_seconds;
      let idle = finish_phase "idle" in
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
