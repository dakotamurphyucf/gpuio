open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Effect = Bonsai.Effect
module Wire = Gpuio_protocol.Wire.Canvas
module Scene = Gpuio_protocol.Canvas_scene_wire
module Geometry = Gpuio_protocol.Canvas_wire
module Id = Gpuio_protocol.Resource_id

let scene () : Scene.t =
  let rect : Geometry.Rect.t = { x = 0.; y = 0.; width = 10.; height = 10. } in
  { version = 1L
  ; description = "Large retained scene"
  ; resources = []
  ; items =
      List.init 20_000 ~f:(fun index ->
        { Scene.Item.id = Int64.of_int (index + 1)
        ; transform = Geometry.Transform.identity
        ; clips = []
        ; drawing = Shape (Rectangle rect, { fill = Some 0x102030ffL; stroke = None })
        ; interaction = None
        })
  }
;;

let () =
  let completed = ref false in
  let shutdown_completions = ref 0 in
  App.run ~exit_on_last_window:false (fun env app ->
    let scope = App.scope app in
    Scope.start
      scope
      ~f:(fun () ->
        Eio.Time.with_timeout_exn (Eio.Stdenv.clock env) 30. (fun () ->
          let on_ui ui_effect =
            let promise, resolver = Eio.Promise.create () in
            Scope.Expert.enqueue scope (fun () ->
              Effect.Expert.handle
                (Effect.map ui_effect ~f:(Eio.Promise.resolve resolver)));
            Eio.Promise.await promise
          in
          let request message = on_ui (App.Expert.canvas app message) in
          let ack message =
            let response = request message in
            if not (Wire.Response.equal response Ack)
            then raise_s [%sexp "expected canvas Ack", (response : Wire.Response.t)]
          in
          let rec create () =
            match request Create with
            | Created id -> id
            | Failed Not_ready ->
              Eio.Time.sleep (Eio.Stdenv.clock env) 0.005;
              create ()
            | response ->
              raise_s [%sexp "expected scene handle", (response : Wire.Response.t)]
          in
          let id = create () in
          let bytes =
            Bin_prot.Utils.bin_dump Scene.bin_writer_t (scene ()) |> Bigstring.to_string
          in
          assert (String.length bytes > Gpuio_protocol.Wire.max_message_bytes);
          let begin_ ~base ~revision ~generation bytes =
            ack
              (Begin
                 { id
                 ; base
                 ; revision
                 ; generation
                 ; bytes = Int64.of_int (String.length bytes)
                 })
          in
          begin_ ~base:0L ~revision:1L ~generation:1L bytes;
          let first = String.sub bytes ~pos:0 ~len:Wire.max_chunk_bytes in
          ack (Chunk (id, 1L, 0L, first));
          assert (Wire.Response.equal (request (Publish (id, 1L))) (Failed Incomplete));
          let rec upload offset =
            if offset < String.length bytes
            then (
              let length = Int.min Wire.max_chunk_bytes (String.length bytes - offset) in
              ack
                (Chunk
                   (id, 1L, Int64.of_int offset, String.sub bytes ~pos:offset ~len:length));
              upload (offset + length))
          in
          upload (String.length first);
          ack (Publish (id, 1L));
          assert (
            Wire.Response.equal
              (request
                 (Begin { id; base = 0L; revision = 2L; generation = 1L; bytes = 1L }))
              (Failed Invalid_revision));
          begin_ ~base:1L ~revision:2L ~generation:1L "\255";
          ack (Chunk (id, 2L, 0L, "\255"));
          assert (Wire.Response.equal (request (Publish (id, 2L))) (Failed Invalid_scene));
          ack (Abort (id, 2L));
          let empty =
            Bin_prot.Utils.bin_dump
              Scene.bin_writer_t
              { version = 1L; resources = []; items = []; description = "Reset" }
            |> Bigstring.to_string
          in
          begin_ ~base:1L ~revision:2L ~generation:2L empty;
          ack (Chunk (id, 2L, 0L, empty));
          ack (Publish (id, 2L));
          ack (Release id);
          let replacement = create () in
          assert (Int64.equal (Id.slot id) (Id.slot replacement));
          assert (Int64.(Id.generation replacement > Id.generation id));
          assert (Wire.Response.equal (request (Release id)) (Failed Stale_handle));
          ack (Release replacement);
          let pending =
            on_ui
              (Effect.of_thunk (fun () ->
                 let pending =
                   List.init 63 ~f:(fun _ ->
                     let promise, resolver = Eio.Promise.create () in
                     Effect.Expert.handle
                       (Effect.map
                          (App.Expert.canvas app Create)
                          ~f:(Eio.Promise.resolve resolver));
                     promise)
                 in
                 Effect.Expert.handle
                   (Effect.map (App.Expert.canvas app Create) ~f:(fun response ->
                      assert (Wire.Response.equal response (Failed Resource_limit))));
                 pending))
          in
          List.iter pending ~f:(fun promise ->
            match Eio.Promise.await promise with
            | Created id -> ack (Release id)
            | response ->
              raise_s [%sexp "unexpected lane response", (response : Wire.Response.t)])))
      ~on_result:(fun result ->
        Effect.of_thunk (fun () ->
          Or_error.ok_exn result;
          let stats = App.stats app in
          assert (stats.commits = 0 && stats.rendered = 0);
          let pending_create () =
            Effect.Expert.handle
              (Effect.map (App.Expert.canvas app Create) ~f:(fun response ->
                 assert (Wire.Response.equal response (Failed Closed));
                 incr shutdown_completions))
          in
          for _ = 1 to 8 do
            pending_create ()
          done;
          completed := true;
          App.shutdown app;
          pending_create ()))
    |> Or_error.ok_exn
    |> fun (_ : Scope.Task.t) -> ());
  assert !completed;
  assert (!shutdown_completions = 9);
  Eio.traceln
    "GPUIO_CANVAS_UPLOAD_OK: chunked 20k-item scene, atomic rejection/reset, stale IDs, \
     bounded request lanes and shutdown"
;;
