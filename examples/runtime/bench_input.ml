open Core
open Gpuio_protocol
module B = Bonsai.Cont
module Driver = Gpuio_runtime_core.Window_driver

let component graph =
  let state, set_state = B.state 0 graph in
  let open B.Let_syntax in
  let%arr state = state
  and set_state = set_state in
  Gpuio_bonsai.View.button ~on_click:(set_state (state + 1)) (Int.to_string state)
;;

let () =
  Eio_main.run (fun env ->
    let now () =
      Time_ns.of_span_since_epoch
        (Time_ns.Span.of_sec (Eio.Time.now (Eio.Stdenv.clock env)))
    in
    let mono () = Eio.Time.Mono.now (Eio.Stdenv.mono_clock env) in
    List.iter [ 1; 4 ] ~f:(fun windows ->
      let drivers =
        List.init windows ~f:(fun slot ->
          let id =
            Window_id.create ~slot:(Int64.of_int slot) ~generation:1L |> Or_error.ok_exn
          in
          id, Driver.create id ~start:(now ()) ~theme:Gpuio.Theme.default component)
      in
      let apply driver =
        match Driver.next_message driver with
        | Some (Wire.Message.Apply tx) ->
          Driver.submitted driver;
          Driver.acknowledge driver ~revision:tx.revision |> Or_error.ok_exn;
          Some tx
        | Some _ -> assert false
        | None -> None
      in
      let bindings =
        List.map drivers ~f:(fun (id, driver) ->
          Driver.cycle driver ~now:(now ()) |> Or_error.ok_exn;
          let tx = apply driver |> Option.value_exn in
          let node, handler =
            List.find_map_exn tx.operations ~f:(function
              | Create (node, Button, _, Some handler) -> Some (node, handler)
              | _ -> None)
          in
          id, driver, node, handler)
      in
      let id, target, node, handler = List.hd_exn bindings in
      let samples =
        List.init 100 ~f:(fun _ ->
          let started = mono () in
          Driver.advance_clock target ~now:(now ());
          Driver.dispatch target (Press (id, node, handler, Driver.revision target));
          let messages =
            List.count drivers ~f:(fun (_, driver) ->
              Driver.cycle driver ~now:(now ()) |> Or_error.ok_exn;
              Option.is_some (apply driver))
          in
          assert (messages = 1);
          Mtime.Span.to_float_ns (Mtime.span started (mono ())) /. 1e6)
        |> List.sort ~compare:Float.compare
      in
      List.iter drivers ~f:(fun (_, driver) -> Driver.close driver);
      Eio.Flow.copy_string
        (sprintf
           "INPUT_CORE_MEASURE windows=%d samples=100 median_ms=%.3f p95_ms=%.3f \
            max_ms=%.3f\n"
           windows
           (List.nth_exn samples 50)
           (List.nth_exn samples 95)
           (List.last_exn samples))
        (Eio.Stdenv.stdout env)))
;;
