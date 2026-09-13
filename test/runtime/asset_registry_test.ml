open Core
module Registry = Gpuio_eio.Asset_registry
module Scope = Gpuio_eio.Scope
module Inbox = Gpuio_eio.Inbox
module Wire = Gpuio_protocol.Wire.Asset
module Id = Gpuio_protocol.Resource_id
module Source = Gpuio.Asset.Source

let with_registry f =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:8 () in
      let root = Scope.Expert.create ~sw ~inbox ~max_tasks:8 in
      let registry = Registry.create ~scope:root ~wake:(fun () -> ()) in
      Exn.protect
        ~f:(fun () -> f ~sw ~inbox ~root registry)
        ~finally:(fun () ->
          Registry.close registry;
          Scope.cancel root;
          Inbox.close inbox)))
;;

let id = Id.create ~slot:7L ~generation:42L |> Or_error.ok_exn

let source =
  Source.of_bytes ~format:Png (String.make (Wire.max_chunk_bytes + 17) '\255')
  |> Or_error.ok_exn
;;

let name : Wire.Request.t -> string = function
  | Begin _ -> "begin"
  | Append _ -> "append"
  | Finish _ -> "finish"
  | Release _ -> "release"
;;

let response : Wire.Request.t -> Wire.Response.t = function
  | Begin _ -> Begun id
  | Append _ | Finish _ | Release _ -> Ack
;;

let%expect_test
    "cancellation at every upload boundary accounts for late native allocation"
  =
  for cancel_at = 0 to 5 do
    with_registry (fun ~sw:_ ~inbox:_ ~root registry ->
      let scope = Scope.child root ~name:"asset" |> Or_error.ok_exn in
      let completed = ref [] in
      Registry.register registry ~scope source ~on_result:(fun result ->
        completed := result :: !completed);
      let requests = ref [] in
      let step ?(cancel = false) () =
        let request = Registry.next_request registry |> Option.value_exn in
        assert (Option.is_none (Registry.next_request registry));
        requests := name request :: !requests;
        if cancel then Scope.cancel scope;
        Registry.complete registry (response request)
      in
      if cancel_at = 0
      then Scope.cancel scope
      else (
        for index = 1 to Int.min cancel_at 4 do
          step ~cancel:(index = cancel_at) ()
        done;
        if cancel_at = 5 then Scope.cancel scope);
      let rec drain_cleanup () =
        match Registry.next_request registry with
        | None -> ()
        | Some request ->
          requests := name request :: !requests;
          (match request with
           | Release released -> assert (Id.equal released id)
           | _ -> failwith "cancelled upload issued more data");
          Registry.complete registry (response request);
          drain_cleanup ()
      in
      drain_cleanup ();
      assert (Registry.Expert.counts registry |> [%equal: int * int * int] (0, 0, 0));
      assert (List.length !completed = if cancel_at = 5 then 1 else 0);
      List.iter !completed ~f:(function
        | Ok asset -> assert (Registry.Registration.is_released asset)
        | Error _ -> failwith "unexpected failure");
      print_s [%sexp (cancel_at : int), (List.rev !requests : string list)])
  done;
  [%expect
    {|
    (0 ())
    (1 (begin release))
    (2 (begin append release))
    (3 (begin append append release))
    (4 (begin append append finish release))
    (5 (begin append append finish release))
    |}]
;;

let%expect_test
    "finish frees OCaml source; idempotent release survives transient backpressure"
  =
  with_registry (fun ~sw:_ ~inbox:_ ~root registry ->
    let completed = ref None in
    Registry.register registry ~scope:root source ~on_result:(fun result ->
      completed := Some result);
    let offset = ref 0 in
    for _ = 1 to 4 do
      let request = Registry.next_request registry |> Option.value_exn in
      (match request with
       | Append (received_id, position, bytes) ->
         assert (Id.equal received_id id && Int64.equal position (Int64.of_int !offset));
         assert (
           String.equal
             bytes
             (String.sub (Source.bytes source) ~pos:!offset ~len:(String.length bytes)));
         offset := !offset + String.length bytes
       | Finish _ ->
         assert ([%equal: int * int * int] (Registry.Expert.counts registry) (1, 1, 0))
       | Begin _ -> ()
       | Release _ -> failwith "premature release");
      Registry.complete registry (response request)
    done;
    let asset =
      match !completed with
      | Some (Ok asset) -> asset
      | _ -> failwith "missing registration"
    in
    assert (Option.value_exn (Registry.Registration.Expert.native_id asset) |> Id.equal id);
    assert ([%equal: int * int * int] (Registry.Expert.counts registry) (1, 0, 0));
    Registry.Registration.release asset;
    Registry.Registration.release asset;
    assert (Option.is_none (Registry.Registration.Expert.native_id asset));
    let release = Registry.next_request registry |> Option.value_exn in
    Registry.complete registry (Failed Resource_limit);
    let retry = Registry.next_request registry |> Option.value_exn in
    assert (Wire.Request.equal release retry);
    Registry.complete registry Ack;
    assert (Option.is_none (Registry.next_request registry));
    assert ([%equal: int * int * int] (Registry.Expert.counts registry) (0, 0, 0));
    print_endline "exact bytes; source freed; one retirement; retry acknowledged");
  [%expect {| exact bytes; source freed; one retirement; retry acknowledged |}]
;;

let%expect_test
    "bounded admission, cancellation frees queued data, and foreign scopes reject"
  =
  with_registry (fun ~sw ~inbox ~root registry ->
    let scope = Scope.child root ~name:"uploads" |> Or_error.ok_exn in
    let errors = ref [] in
    let on_result = function
      | Error error -> errors := error :: !errors
      | Ok _ -> failwith "not driven"
    in
    for _ = 1 to 8 do
      Registry.register registry ~scope source ~on_result
    done;
    Registry.register registry ~scope source ~on_result;
    let foreign = Scope.Expert.create ~sw ~inbox ~max_tasks:1 in
    Registry.register registry ~scope:foreign source ~on_result;
    Scope.cancel foreign;
    Scope.cancel scope;
    assert ([%equal: int * int * int] (Registry.Expert.counts registry) (0, 0, 0));
    assert (Option.is_none (Registry.next_request registry));
    let large =
      Source.of_bytes ~format:Png (String.make Source.max_bytes 'x') |> Or_error.ok_exn
    in
    for _ = 1 to 4 do
      Registry.register registry ~scope:root large ~on_result
    done;
    Registry.register registry ~scope:root source ~on_result;
    Registry.close registry;
    assert ([%equal: int * int * int] (Registry.Expert.counts registry) (0, 0, 0));
    Registry.register registry ~scope:root source ~on_result;
    print_s [%sexp (List.rev !errors : Registry.Error.t list)]);
  [%expect {| (Resource_limit Invalid_scope Resource_limit Closed) |}]
;;

let%expect_test
    "native errors clean up known IDs and shutdown suppresses pending completion"
  =
  with_registry (fun ~sw:_ ~inbox:_ ~root registry ->
    let results = ref [] in
    let on_result result = results := result :: !results in
    Registry.register registry ~scope:root source ~on_result;
    ignore (Registry.next_request registry : Wire.Request.t option);
    Registry.complete registry (Failed Resource_limit);
    assert ([%equal: int * int * int] (Registry.Expert.counts registry) (0, 0, 0));
    Registry.register registry ~scope:root source ~on_result;
    ignore (Registry.next_request registry : Wire.Request.t option);
    Registry.complete registry (Begun id);
    ignore (Registry.next_request registry : Wire.Request.t option);
    Registry.complete registry (Failed Invalid_chunk);
    (match Registry.next_request registry with
     | Some (Release found) -> assert (Id.equal found id)
     | _ -> failwith "missing cleanup");
    Registry.complete registry Ack;
    Registry.register registry ~scope:root source ~on_result;
    ignore (Registry.next_request registry : Wire.Request.t option);
    Registry.close registry;
    Registry.complete registry (Begun id);
    assert (Option.is_none (Registry.next_request registry));
    assert ([%equal: int * int * int] (Registry.Expert.counts registry) (0, 0, 0));
    print_s
      [%sexp
        (List.rev_map !results ~f:(function
           | Error error -> error
           | Ok _ -> failwith "unexpected ready")
         : Registry.Error.t list)]);
  [%expect {| (Resource_limit Native_failure) |}]
;;

let%expect_test
    "live registration metadata is bounded and scope end retires the entire set"
  =
  with_registry (fun ~sw:_ ~inbox:_ ~root registry ->
    let source = Source.of_bytes ~format:Png "x" |> Or_error.ok_exn in
    let ready = ref 0 in
    for slot = 0 to 1023 do
      Registry.register registry ~scope:root source ~on_result:(function
        | Ok _ -> incr ready
        | Error _ -> failwith "unexpected rejection");
      let id = Id.create ~slot:(Int64.of_int slot) ~generation:1L |> Or_error.ok_exn in
      for _ = 1 to 3 do
        let request = Registry.next_request registry |> Option.value_exn in
        Registry.complete
          registry
          (match request with
           | Begin _ -> Begun id
           | Append _ | Finish _ -> Ack
           | Release _ -> failwith "early retirement")
      done
    done;
    assert (!ready = 1024);
    Registry.register registry ~scope:root source ~on_result:(function
      | Error Resource_limit -> ()
      | _ -> failwith "unbounded registrations");
    Scope.cancel root;
    for slot = 0 to 1023 do
      (match Registry.next_request registry with
       | Some (Release id) -> assert (Int64.equal (Id.slot id) (Int64.of_int slot))
       | _ -> failwith "missing retirement");
      Registry.complete registry Ack
    done;
    assert ([%equal: int * int * int] (Registry.Expert.counts registry) (0, 0, 0));
    assert (Option.is_none (Registry.next_request registry));
    print_endline
      "1024 ready registrations; bound enforced; 1024 acknowledged retirements");
  [%expect {| 1024 ready registrations; bound enforced; 1024 acknowledged retirements |}]
;;
