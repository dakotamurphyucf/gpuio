open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Chart = Gpuio_eio.Chart
module Data = Gpuio.Chart_data
module Wire = Gpuio_protocol.Wire.Chart
module Effect = Bonsai.Effect

let ok = Or_error.ok_exn

let chart_ok = function
  | Ok value -> value
  | Error error -> raise_s [%sexp (error : Chart.Error.t)]
;;

let dataset count name =
  Data.line
    [ Data.Series.create
        ~id:(Data.Series_id.of_int64 1L |> ok)
        ~name
        (List.init count ~f:(fun index ->
           Data.Point.create
             ~id:(Data.Datum_id.of_int64 (Int64.of_int (index + 1)) |> ok)
             ~x:(Float.of_int index)
             ~y:(if index mod 23 = 0 then None else Some (Float.sin (Float.of_int index)))
             ()
           |> ok))
      |> ok
    ]
  |> ok
;;

let () =
  let completed = ref false in
  let shutdown_completions = ref 0 in
  App.run ~exit_on_last_window:false (fun env app ->
    let root = App.scope app in
    Scope.start
      root
      ~f:(fun () ->
        Eio.Time.with_timeout_exn (Eio.Stdenv.clock env) 60. (fun () ->
          let on_ui ui_effect =
            let promise, resolver = Eio.Promise.create () in
            Scope.Expert.enqueue root (fun () ->
              Effect.Expert.handle
                (Effect.map ui_effect ~f:(Eio.Promise.resolve resolver)));
            Eio.Promise.await promise
          in
          let ui f = on_ui (Effect.of_thunk f) in
          let request message = on_ui (App.Expert.chart app message) in
          let ack message =
            let response = request message in
            if not (Wire.Response.equal response Ack)
            then raise_s [%sexp "expected chart Ack", (response : Wire.Response.t)]
          in
          let scope = ui (fun () -> Scope.child root ~name:"chart data" |> ok) in
          let large = dataset 100_000 "Streaming" in
          let registration = on_ui (Chart.create app ~scope large) |> chart_ok in
          let source =
            ui (fun () ->
              assert (Chart.is_published registration);
              Gpuio.Chart_resource.Expert.native_id (Chart.handle registration))
          in
          let wait_published registration =
            let rec loop () =
              if not (ui (fun () -> Chart.is_published registration))
              then (
                ui (fun () ->
                  Option.iter (Chart.error registration) ~f:(fun error ->
                    raise_s [%sexp (error : Chart.Error.t)]));
                Eio.Time.sleep (Eio.Stdenv.clock env) 0.005;
                loop ())
            in
            loop ()
          in
          ui (fun () ->
            for index = 1 to 1000 do
              Chart.set registration (dataset 4 (Int.to_string index)) |> chart_ok
            done;
            Chart.reset registration large |> chart_ok);
          wait_published registration;
          ui (fun () ->
            assert (Data.equal large (Chart.data registration |> Option.value_exn));
            assert ((App.diagnostics app).charts = 1));
          Eio.traceln
            "CHART_UPLOAD_STAGE: 100k points published; coalesced reset accepted";
          (* Exercise decode failure through the actual worker and response path.
             First failed successor is revision 3 after the coalesced reset. *)
          ack
            (Begin { id = source; base = 2L; revision = 3L; generation = 2L; bytes = 1L });
          ack (Chunk (source, 3L, 0L, "\255"));
          assert (
            Wire.Response.equal (request (Publish (source, 3L))) (Failed Invalid_data));
          ack (Abort (source, 3L));
          (* Returning the same expected base proves rejection did not publish. *)
          ack
            (Begin { id = source; base = 2L; revision = 3L; generation = 2L; bytes = 1L });
          ack (Abort (source, 3L));
          ui (fun () ->
            Scope.cancel scope;
            assert (Chart.is_released registration));
          let rec await_cleanup () =
            if ui (fun () -> (App.diagnostics app).charts <> 0)
            then (
              Eio.Time.sleep (Eio.Stdenv.clock env) 0.005;
              await_cleanup ())
          in
          await_cleanup ();
          assert (Wire.Response.equal (request (Release source)) (Failed Stale_handle));
          for _ = 1 to 270 do
            let registration =
              on_ui (Chart.create app ~scope:root (dataset 2 "Reuse")) |> chart_ok
            in
            ui (fun () -> Chart.release registration)
          done;
          await_cleanup ();
          let pending, reserved =
            ui (fun () ->
              let pending =
                List.init 63 ~f:(fun _ ->
                  let promise, resolver = Eio.Promise.create () in
                  Effect.Expert.handle
                    (Effect.map
                       (App.Expert.chart app Create)
                       ~f:(Eio.Promise.resolve resolver));
                  promise)
              in
              Effect.Expert.handle
                (Effect.map (App.Expert.chart app Create) ~f:(fun response ->
                   assert (Wire.Response.equal response (Failed Resource_limit))));
              let reserved, resolver = Eio.Promise.create () in
              Effect.Expert.handle
                (Effect.map
                   (Chart.create app ~scope:root (dataset 2 "Reserved"))
                   ~f:(Eio.Promise.resolve resolver));
              pending, reserved)
          in
          List.iter pending ~f:(fun promise ->
            match Eio.Promise.await promise with
            | Created id -> ack (Release id)
            | response -> raise_s [%sexp (response : Wire.Response.t)]);
          let reserved = Eio.Promise.await reserved |> chart_ok in
          ui (fun () -> Chart.release reserved);
          await_cleanup ();
          ui (fun () ->
            let diagnostics = App.diagnostics app in
            assert (diagnostics.chart_data_bytes = 0);
            assert (diagnostics.runtime.commits = 0 && diagnostics.runtime.rendered = 0))))
      ~on_result:(fun result ->
        Effect.of_thunk (fun () ->
          Or_error.ok_exn result;
          let pending () =
            Effect.Expert.handle
              (Effect.map (App.Expert.chart app Create) ~f:(fun response ->
                 assert (Wire.Response.equal response (Failed Closed));
                 incr shutdown_completions))
          in
          for _ = 1 to 8 do
            pending ()
          done;
          completed := true;
          App.shutdown app;
          pending ()))
    |> ok
    |> fun (_ : Scope.Task.t) -> ());
  assert !completed;
  assert (!shutdown_completions = 9);
  Eio.traceln
    "GPUIO_CHART_UPLOAD_OK: 100k data, rejection, coalescing/reset, scope cleanup, 270 \
     releases, bounded request lanes, shutdown"
;;
