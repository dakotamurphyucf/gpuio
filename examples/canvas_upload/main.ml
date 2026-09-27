open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Effect = Bonsai.Effect
module Wire = Gpuio_protocol.Wire.Canvas
module Scene = Gpuio_protocol.Canvas_scene_wire
module Geometry = Gpuio_protocol.Canvas_wire
module Id = Gpuio_protocol.Resource_id
module Canvas = Gpuio_eio.Canvas
module Typed_scene = Gpuio.Canvas_scene
module Resource = Gpuio.Canvas_resource
module Asset = Gpuio_eio.Asset

let canvas_ok = function
  | Ok value -> value
  | Error error -> raise_s [%sexp (error : Canvas.Error.t)]
;;

let typed_text text =
  let resource =
    Resource.text ~id:(Resource.Id.of_int64 1L |> Or_error.ok_exn) text |> Or_error.ok_exn
  in
  let origin = Gpuio.Canvas_geometry.Point.create ~x:0. ~y:0. |> Or_error.ok_exn in
  let item =
    Typed_scene.Item.create
      ~id:(Typed_scene.Item_id.of_int64 1L |> Or_error.ok_exn)
      (Typed_scene.Drawing.text resource ~origin ~color:(Gpuio.Color.rgb_exn 0x102030))
    |> Or_error.ok_exn
  in
  Typed_scene.create ~description:"Scoped text scene" [ item ] |> Or_error.ok_exn
;;

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
        Eio.Time.with_timeout_exn (Eio.Stdenv.clock env) 60. (fun () ->
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
          let ui f = on_ui (Effect.of_thunk f) in
          let rec wait_for predicate =
            if not (ui predicate)
            then (
              Eio.Time.sleep (Eio.Stdenv.clock env) 0.001;
              wait_for predicate)
          in
          Eio.traceln "CANVAS_STAGE: raw transport passed; scoped create";
          let scoped =
            on_ui (Canvas.create app ~scope (typed_text "first")) |> canvas_ok
          in
          Eio.traceln "CANVAS_STAGE: scoped create published";
          let scoped_handle = ui (fun () -> Canvas.handle scoped) in
          ui (fun () ->
            Canvas.set scoped (typed_text "changed without resource generation")
            |> canvas_ok);
          wait_for (fun () -> Option.is_some (Canvas.error scoped));
          ui (fun () ->
            assert (
              Option.equal
                Canvas.Error.equal
                (Canvas.error scoped)
                (Some (Native Stale_resource)));
            assert (not (Canvas.is_released scoped));
            Canvas.reset scoped (typed_text "reset permits changed content") |> canvas_ok);
          wait_for (fun () -> Canvas.is_published scoped);
          ui (fun () ->
            assert (Option.is_none (Canvas.error scoped));
            assert (Typed_scene.Handle.equal scoped_handle (Canvas.handle scoped));
            for index = 1 to 1000 do
              Canvas.reset scoped (typed_text (Int.to_string index)) |> canvas_ok
            done);
          wait_for (fun () -> Canvas.is_published scoped);
          let scoped_id = Typed_scene.Expert.native_id scoped_handle in
          (* Only the final coalesced reset published: revisions 1,2,3 and
             generations 1,2,3, despite 1000 requested resets. *)
          ack
            (Begin
               { id = scoped_id; base = 3L; revision = 4L; generation = 3L; bytes = 1L });
          ack (Abort (scoped_id, 4L));
          ui (fun () -> Canvas.release scoped);
          Eio.traceln "CANVAS_STAGE: rejection/reset/coalescing passed";
          let child =
            ui (fun () -> Scope.child scope ~name:"canvas-assets" |> Or_error.ok_exn)
          in
          let source =
            Asset.Source.of_bytes ~format:Pnm "P6\n1 1\n255\n\020\100\240"
            |> Or_error.ok_exn
          in
          let asset =
            match on_ui (Asset.register app ~scope:child source) with
            | Ok asset -> asset
            | Error error -> raise_s [%sexp (error : Asset.Error.t)]
          in
          Eio.traceln "CANVAS_STAGE: asset published";
          let image_scene =
            ui (fun () ->
              let resource =
                Resource.image
                  ~id:(Resource.Id.of_int64 1L |> Or_error.ok_exn)
                  (Asset.handle asset)
                |> Or_error.ok_exn
              in
              let bounds =
                Gpuio.Canvas_geometry.Rect.create ~x:0. ~y:0. ~width:10. ~height:10.
                |> Or_error.ok_exn
              in
              let item =
                Typed_scene.Item.create
                  ~id:(Typed_scene.Item_id.of_int64 1L |> Or_error.ok_exn)
                  (Typed_scene.Drawing.image resource ~bounds)
                |> Or_error.ok_exn
              in
              Typed_scene.create ~description:"Leased image" [ item ] |> Or_error.ok_exn)
          in
          let image_canvas =
            on_ui (Canvas.create app ~scope:child image_scene) |> canvas_ok
          in
          Eio.traceln "CANVAS_STAGE: image scene published";
          let asset_id =
            ui (fun () -> Asset.handle asset |> Gpuio.Asset.Expert.native_id)
          in
          ui (fun () -> Asset.release asset);
          (* Asset retirement is idempotent and keeps its generation alive while
             leases exist. A second acknowledged Release is a transport barrier,
             not a Stale_handle probe (retired leases deliberately retain IDs). *)
          assert (
            Gpuio_protocol.Wire.Asset.Response.equal
              (on_ui (App.Expert.asset app (Release asset_id)))
              Ack);
          Eio.traceln "CANVAS_STAGE: asset release observed";
          ui (fun () -> Canvas.reset image_canvas image_scene |> canvas_ok);
          wait_for (fun () -> Canvas.is_published image_canvas);
          Eio.traceln "CANVAS_STAGE: image scene republished after asset release";
          (match on_ui (Canvas.create app ~scope:child image_scene) with
           | Error (Native Unavailable_image) -> ()
           | Error error -> raise_s [%sexp (error : Canvas.Error.t)]
           | Ok _ -> failwith "retired asset was acquired by a new canvas");
          ui (fun () ->
            Scope.cancel child;
            assert (Canvas.is_released image_canvas));
          (* Repeated scoped allocation/release reuses slots without consuming
             the 256 native registrations. No window is involved. *)
          Eio.traceln "CANVAS_STAGE: image leases passed; 270 registrations";
          for index = 1 to 270 do
            let registered =
              on_ui (Canvas.create app ~scope (typed_text "churn")) |> canvas_ok
            in
            ui (fun () -> Canvas.release registered);
            if index mod 90 = 0
            then Eio.traceln "CANVAS_STAGE: released %d registrations" index
          done;
          (* Drain the last asynchronous cleanup before measuring an exactly
             empty raw lane; the old scene ID has been retired/reused. *)
          assert (Wire.Response.equal (request (Release scoped_id)) (Failed Stale_handle));
          let pending, reserved =
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
                 let reserved, resolver = Eio.Promise.create () in
                 Effect.Expert.handle
                   (Effect.map
                      (Canvas.create app ~scope (typed_text "reserved lane"))
                      ~f:(Eio.Promise.resolve resolver));
                 pending, reserved))
          in
          List.iter pending ~f:(fun promise ->
            match Eio.Promise.await promise with
            | Created id -> ack (Release id)
            | response ->
              raise_s [%sexp "unexpected lane response", (response : Wire.Response.t)]);
          let reserved = Eio.Promise.await reserved |> canvas_ok in
          ui (fun () ->
            assert (Canvas.is_published reserved);
            Canvas.release reserved)))
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
     bounded request lanes, scoped recovery/coalescing/image leases/270 releases and \
     shutdown"
;;
