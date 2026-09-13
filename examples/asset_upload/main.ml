open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module E = Bonsai.Effect
module Asset = Gpuio_protocol.Wire.Asset
module Id = Gpuio_protocol.Resource_id

let () =
  let complete = ref false in
  App.run ~exit_on_last_window:false (fun env app ->
    let scope = App.scope app in
    let clock = Eio.Stdenv.clock env in
    Scope.start
      scope
      ~f:(fun () ->
        Eio.Time.with_timeout_exn clock 20. (fun () ->
          let request message =
            let promise, resolver = Eio.Promise.create () in
            Scope.Expert.enqueue scope (fun () ->
              E.Expert.handle
                (E.map (App.Expert.asset app message) ~f:(fun response ->
                   Eio.Promise.resolve resolver response)));
            Eio.Promise.await promise
          in
          let ack message =
            match request message with
            | Asset.Response.Ack -> ()
            | response ->
              raise_s
                [%sexp "expected asset acknowledgement", (response : Asset.Response.t)]
          in
          let begin_ size =
            match request (Begin (Png, Int64.of_int size)) with
            | Begun id -> id
            | response ->
              raise_s [%sexp "expected asset upload", (response : Asset.Response.t)]
          in
          let rec wait_ready () =
            match request (Begin (Png, 0L)) with
            | Failed Not_ready ->
              Eio.Time.sleep clock 0.005;
              wait_ready ()
            | Failed Invalid_size -> ()
            | response ->
              raise_s
                [%sexp "unexpected negotiation response", (response : Asset.Response.t)]
          in
          wait_ready ();
          let bytes =
            String.init ((2 * 1024 * 1024) + 13) ~f:(fun i -> Char.of_int_exn (i mod 256))
          in
          let id = begin_ (String.length bytes) in
          let rec upload offset =
            if offset < String.length bytes
            then (
              let length = Int.min Asset.max_chunk_bytes (String.length bytes - offset) in
              ack
                (Append (id, Int64.of_int offset, String.sub bytes ~pos:offset ~len:length));
              upload (offset + length))
          in
          upload 0;
          ack (Finish id);
          ack (Release id);
          ack (Release id);
          let next = begin_ 4 in
          assert (Int64.equal (Id.slot id) (Id.slot next));
          assert (Int64.(Id.generation next > Id.generation id));
          assert (Asset.Response.equal (request (Release id)) (Failed Stale_handle));
          ack (Append (next, 0L, "\000\255"));
          assert (Asset.Response.equal (request (Finish next)) (Failed Incomplete));
          let invalid = begin_ 1 in
          assert (
            Asset.Response.equal
              (request (Append (invalid, -1L, "a")))
              (Failed Invalid_chunk));
          assert (Asset.Response.equal (request (Finish invalid)) (Failed Not_uploading));
          let reserved = List.init 4 ~f:(fun _ -> begin_ Gpuio.Asset.Source.max_bytes) in
          assert (Asset.Response.equal (request (Begin (Png, 1L))) (Failed Resource_limit));
          List.iter reserved ~f:(fun id -> ack (Release id));
          let recovered = begin_ 1 in
          ack (Append (recovered, 0L, "x"));
          ack (Finish recovered);
          ack (Release recovered)))
      ~on_result:(fun result ->
        E.of_thunk (fun () ->
          Or_error.ok_exn result;
          let stats = App.stats app in
          assert (stats.commits = 0 && stats.rendered = 0);
          complete := true;
          App.shutdown app))
    |> Or_error.ok_exn
    |> fun (_ : Scope.Task.t) -> ());
  assert !complete;
  Eio.traceln
    "GPUIO_ASSET_UPLOAD_OK: >2 MiB chunked FFI upload, stale release, prefix/offset \
     rejection, quota recovery and shutdown without windows"
;;
