open Core
module Registry = Gpuio_eio.Document_registry
module Scope = Gpuio_eio.Scope
module Inbox = Gpuio_eio.Inbox
module Wire = Gpuio_protocol.Wire.Document
module Id = Gpuio_protocol.Resource_id
module Source = Gpuio.Text_source

let with_registry f =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:8 () in
      let root = Scope.Expert.create ~sw ~inbox ~max_tasks:8 in
      let registry = Registry.create ~scope:root ~wake:(fun () -> ()) in
      Exn.protect
        ~f:(fun () -> f root registry)
        ~finally:(fun () ->
          Registry.close registry;
          Scope.cancel root;
          Inbox.close inbox)))
;;

let id = Id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let set registration source =
  match Registry.Registration.set registration source with
  | Ok () -> ()
  | Error error -> raise_s [%sexp (error : Wire.Error.t)]
;;

let response : Wire.Request.t -> Wire.Response.t = function
  | Create -> Created id
  | Begin _ | Chunk _ | Publish _ | Abort _ | Release _ -> Ack
;;

let drain registry ~f =
  let rec loop remaining =
    assert (remaining > 0);
    match Registry.next_request registry with
    | None -> ()
    | Some request ->
      assert (Option.is_none (Registry.next_request registry));
      f request;
      Registry.complete registry (response request);
      loop (remaining - 1)
  in
  loop 100
;;

let%expect_test "coalescing sends only the suffix and preserves terminal content" =
  with_registry (fun scope registry ->
    let initial =
      Source.of_string ~status:Streaming (String.make 100_000 'a') |> Or_error.ok_exn
    in
    let registration = ref None in
    Registry.register registry ~scope initial ~on_result:(function
      | Ok value -> registration := Some value
      | Error _ -> failwith "registration failed");
    drain registry ~f:(fun _ -> ());
    let registration = Option.value_exn !registration in
    let desired = ref initial in
    for _ = 1 to 1000 do
      desired := Source.append !desired "λ" |> Or_error.ok_exn;
      set registration !desired
    done;
    desired := Source.finish !desired |> Or_error.ok_exn;
    set registration !desired;
    let starts = ref [] in
    let bytes = ref [] in
    drain registry ~f:(function
      | Begin update ->
        starts := (update.from_byte, update.suffix_bytes, update.status) :: !starts
      | Chunk (_, _, _, data) -> bytes := data :: !bytes
      | _ -> ());
    print_s [%sexp (List.rev !starts : (int64 * int64 * Wire.Status.t) list)];
    print_s [%sexp (String.length (String.concat (List.rev !bytes)) : int)];
    let _, held = Registry.Expert.counts registry in
    print_s [%sexp (held = Source.byte_length !desired : bool)];
    Registry.Registration.release registration;
    drain registry ~f:(function
      | Release _ -> ()
      | _ -> failwith "unexpected data after release");
    print_s [%sexp (Registry.Expert.counts registry : int * int)]);
  [%expect
    {|
    ((100000 2000 Complete))
    2000
    true
    (0 0)
    |}]
;;

let%expect_test "cancellation at every boundary suppresses publication callbacks" =
  for cancel_at = 0 to 4 do
    with_registry (fun root registry ->
      let scope = Scope.child root ~name:"document" |> Or_error.ok_exn in
      let callbacks = ref 0 in
      let source = Source.of_string "hello" |> Or_error.ok_exn in
      Registry.register registry ~scope source ~on_result:(fun _ -> Int.incr callbacks);
      if cancel_at = 0
      then Scope.cancel scope
      else
        for index = 1 to cancel_at do
          let request = Registry.next_request registry |> Option.value_exn in
          if index = cancel_at then Scope.cancel scope;
          Registry.complete registry (response request)
        done;
      drain registry ~f:(function
        | Release _ -> ()
        | _ -> failwith "cancelled source uploaded more data");
      assert (!callbacks = 0);
      assert ([%equal: int * int] (Registry.Expert.counts registry) (0, 0)))
  done;
  print_s [%sexp "all upload boundaries cleaned"];
  [%expect {| "all upload boundaries cleaned" |}]
;;

let%expect_test "new desired snapshots survive a pending publication and generation reset"
  =
  with_registry (fun scope registry ->
    let initial = Source.empty_stream () in
    let registration = ref None in
    Registry.register registry ~scope initial ~on_result:(function
      | Ok value -> registration := Some value
      | Error _ -> failwith "failed");
    drain registry ~f:(fun _ -> ());
    let registration = Option.value_exn !registration in
    let next = Source.append initial "old" |> Or_error.ok_exn in
    set registration next;
    let pending = Registry.next_request registry |> Option.value_exn in
    let reset = Source.reset next "new" |> Or_error.ok_exn in
    set registration reset;
    Registry.complete registry (response pending);
    let commits = ref [] in
    drain registry ~f:(function
      | Begin update ->
        commits
        := (update.base, update.revision, update.generation, update.from_byte) :: !commits
      | _ -> ());
    print_s [%sexp (List.rev !commits : (int64 * int64 * int64 * int64) list)];
    print_s
      [%sexp
        (Option.value_exn (Registry.Registration.source registration) |> Source.to_string
         : string)]);
  [%expect
    {|
    ((2 3 2 0))
    new
    |}]
;;

let%expect_test "navigation is rejected immediately on reset and scope release" =
  with_registry (fun scope registry ->
    let source = Source.empty_stream () in
    let registration = ref None in
    Registry.register registry ~scope source ~on_result:(function
      | Ok value -> registration := Some value
      | Error _ -> failwith "failed");
    drain registry ~f:(fun _ -> ());
    let registration = Option.value_exn !registration in
    let id = Registry.Registration.handle registration |> Source.Expert.native_id in
    assert (Registry.accepts_navigation registry id ~generation:1L);
    set registration (Source.reset source "new" |> Or_error.ok_exn);
    assert (not (Registry.accepts_navigation registry id ~generation:1L));
    drain registry ~f:(fun _ -> ());
    assert (Registry.accepts_navigation registry id ~generation:2L);
    Registry.Registration.release registration;
    assert (not (Registry.accepts_navigation registry id ~generation:2L)));
  [%expect {| |}]
;;
