open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module W = Gpuio.Window
module E = Bonsai.Effect
module B = Bonsai.Cont
module View = Gpuio_bonsai.View

let component label window _graph =
  B.return
    (View.column
       [ View.text label
       ; View.button
           "Close this window"
           ~on_click:(E.of_sync_fun App.Window.request_close window)
       ])
;;

let run_decisions () =
  let passed = ref false in
  App.run ~exit_on_last_window:false (fun env app ->
    let scope = App.scope app in
    let open_window title =
      App.open_window app ~focus:false ~title ~width:600. ~height:360. (component title)
      |> Or_error.ok_exn
    in
    let first = open_window "Workspace one" in
    let second = open_window "Workspace two" in
    let cancelled = ref 0 in
    let (_ : unit -> unit) =
      Scope.Expert.on_cancel (App.Window.scope first) (fun () -> incr cancelled)
      |> Or_error.ok_exn
    in
    Scope.start
      scope
      ~f:(fun () ->
        let clock = Eio.Stdenv.clock env in
        let wait f =
          while not (f ()) do
            Eio.Time.sleep clock 0.005
          done
        in
        let on_ui ui_effect =
          let promise, resolver = Eio.Promise.create () in
          Scope.Expert.enqueue scope (fun () ->
            E.Expert.handle (E.map ui_effect ~f:(Eio.Promise.resolve resolver)));
          Eio.Promise.await promise
        in
        let sync f = on_ui (E.of_sync_fun f ()) in
        let command window cmd = on_ui (App.Window.command window cmd) in
        Eio.Time.with_timeout_exn clock 20. (fun () ->
          wait (fun () ->
            Option.is_some (App.Window.snapshot first)
            && Option.is_some (App.Window.snapshot second));
          let observe window =
            let rec loop () =
              match command window Observe with
              | Error Not_ready ->
                Eio.Time.sleep clock 0.005;
                loop ()
              | Ok snapshot -> snapshot
              | Error error -> raise_s [%sexp (error : W.Error.t)]
            in
            loop ()
          in
          ignore (observe first : W.Snapshot.t);
          ignore (observe second : W.Snapshot.t);
          let changed = command first (Set_title "Renamed λ") in
          (match changed with
           | Ok snapshot -> assert (String.equal snapshot.title "Renamed λ")
           | Error _ -> failwith "title command");
          assert (String.equal (observe second).title "Workspace two");
          ignore
            (command first (Resize (640., 400.)) : (W.Snapshot.t, W.Error.t) Result.t);
          wait (fun () ->
            Option.exists (App.Window.snapshot first) ~f:(fun snapshot ->
              Float.(
                abs (snapshot.content_width -. 640.) < 1.
                && abs (snapshot.content_height -. 400.) < 1.)));
          let calls = ref 0
          and answer = ref None in
          let handler _reason =
            E.Expert.of_fun ~f:(fun ~callback ->
              incr calls;
              answer := Some callback)
          in
          sync (fun () ->
            App.Window.set_close_handler first handler;
            App.Window.request_close first;
            App.Window.request_close first);
          wait (fun () -> !calls = 1);
          assert (not (App.Window.is_closed first));
          let stale = Option.value_exn !answer in
          sync (fun () -> stale W.Close_decision.Keep_open);
          (* A subsequent queued UI turn observes the completed decision. *)
          Eio.Time.sleep clock 0.02;
          assert (!cancelled = 0);
          sync (fun () -> App.Window.request_close first);
          wait (fun () -> !calls = 2);
          let allow = Option.value_exn !answer in
          sync (fun () -> allow W.Close_decision.Allow);
          wait (fun () -> App.Window.is_closed first);
          assert (!cancelled = 1);
          assert (not (App.Window.is_closed second));
          (match command first Observe with
           | Error Closed -> ()
           | _ -> failwith "closed generation command accepted");
          let replacement = sync (fun () -> open_window "Replacement") in
          ignore (observe replacement : W.Snapshot.t);
          sync (fun () ->
            stale W.Close_decision.Allow;
            allow W.Close_decision.Allow);
          assert (not (App.Window.is_closed replacement));
          let second_calls = ref 0
          and second_answer = ref None in
          let replacement_calls = ref 0
          and replacement_answer = ref None in
          let guard calls answer reason =
            E.Expert.of_fun ~f:(fun ~callback ->
              assert (W.Close_reason.equal reason Application_quit);
              incr calls;
              answer := Some callback)
          in
          sync (fun () ->
            App.Window.set_close_handler second (guard second_calls second_answer);
            App.Window.set_close_handler
              replacement
              (guard replacement_calls replacement_answer);
            App.request_quit app;
            App.request_quit app);
          wait (fun () -> !second_calls = 1 && !replacement_calls = 1);
          assert (not (App.Window.is_closed second || App.Window.is_closed replacement));
          assert (
            Result.is_error
              (App.open_window
                 app
                 ~focus:false
                 ~title:"Blocked"
                 ~width:400.
                 ~height:300.
                 (component "Blocked")));
          sync (fun () -> (Option.value_exn !second_answer) Keep_open);
          Eio.Time.sleep clock 0.02;
          (* Repeated denied attempts cannot accumulate waiters on the other
             window's still-pending decision. *)
          for attempt = 2 to 20 do
            sync (fun () -> App.request_quit app);
            wait (fun () -> !second_calls = attempt);
            sync (fun () -> (Option.value_exn !second_answer) Keep_open);
            Eio.Time.sleep clock 0.005
          done;
          assert (!replacement_calls = 1);
          sync (fun () -> App.request_quit app);
          wait (fun () -> !second_calls = 21);
          sync (fun () -> App.Window.close replacement);
          assert (App.Window.is_closed replacement);
          assert (not (App.Window.is_closed second));
          passed := true;
          sync (fun () -> (Option.value_exn !second_answer) Allow)))
      ~on_result:(fun result ->
        match result with
        | Ok () -> E.Ignore
        | Error error ->
          E.of_sync_fun
            (fun () ->
               App.shutdown app;
               Error.raise error)
            ())
    |> Or_error.ok_exn
    |> ignore);
  assert !passed;
  Eio.traceln
    "GPUIO_WINDOW_LIFECYCLE_OK: two windows, title/size, delayed deny/allow, close \
     coalescing, scope cancellation, stale generation, bounded repeated quit decisions, \
     atomic quit"
;;

let run_last_window () =
  let passed = ref false in
  App.run (fun env app ->
    let open_window title =
      App.open_window app ~focus:false ~title ~width:500. ~height:300. (component title)
      |> Or_error.ok_exn
    in
    let first = open_window "First"
    and last = open_window "Last" in
    Scope.start
      (App.scope app)
      ~f:(fun () ->
        let clock = Eio.Stdenv.clock env in
        Eio.Time.with_timeout_exn clock 10. (fun () ->
          while
            Option.is_none (App.Window.snapshot first)
            || Option.is_none (App.Window.snapshot last)
          do
            Eio.Time.sleep clock 0.005
          done;
          App.Window.request_close first;
          while not (App.Window.is_closed first) do
            Eio.Time.sleep clock 0.005
          done;
          assert (not (App.Window.is_closed last));
          passed := true;
          App.Window.request_close last))
      ~on_result:(function
        | Ok () -> E.Ignore
        | Error error ->
          E.of_sync_fun
            (fun () ->
               App.shutdown app;
               Error.raise error)
            ())
    |> Or_error.ok_exn
    |> ignore);
  assert !passed;
  Eio.traceln
    "GPUIO_LAST_WINDOW_OK: closing the first window preserves the second; closing the \
     last returns from App.run"
;;

let () =
  if Array.exists (Sys.get_argv ()) ~f:(String.equal "--last-window")
  then run_last_window ()
  else run_decisions ()
;;
