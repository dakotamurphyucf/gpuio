open Core
module Registry = Gpuio_eio.Chart_registry
module Registration = Registry.Registration
module Scope = Gpuio_eio.Scope
module Inbox = Gpuio_eio.Inbox
module Wire = Gpuio_protocol.Chart_resource_wire
module Data = Gpuio.Chart_data
module Resource = Gpuio.Chart_resource

let ok = Or_error.ok_exn
let id = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok

let empty description =
  Data.pie
    [ Data.Slice.create ~id:(Data.Datum_id.of_int64 1L |> ok) ~label:description ~value:1.
      |> ok
    ]
  |> ok
;;

let with_registry f =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:8 () in
      let root = Scope.Expert.create ~sw ~inbox ~max_tasks:8 in
      let registry = Registry.create ~scope:root ~wake:(fun () -> ()) in
      Exn.protect
        ~f:(fun () -> f root registry ())
        ~finally:(fun () ->
          Registry.close registry;
          Scope.cancel root;
          Inbox.close inbox)))
;;

let response : Wire.Request.t -> Wire.Response.t = function
  | Create -> Created id
  | Begin _ | Chunk _ | Publish _ | Abort _ | Release _ -> Ack
;;

let next registry = Registry.next_request registry |> Option.value_exn

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
  loop 2048
;;

let registered registry scope scene =
  let result = ref None in
  Registry.register registry ~scope scene ~on_result:(fun value -> result := Some value);
  assert (Option.is_none !result);
  drain registry ~f:(fun _ -> assert (Option.is_none !result));
  match Option.value_exn !result with
  | Ok registration -> registration
  | Error error -> raise_s [%sexp (error : Registry.Error.t)]
;;

let change result =
  match result with
  | Ok () -> ()
  | Error error -> raise_s [%sexp (error : Registry.Error.t)]
;;

let begin_info (update : Wire.Update.t) = update.base, update.revision, update.generation

let%expect_test "selection revisions fence resets, in-flight publication and release" =
  with_registry (fun scope registry _ ->
    let registration = registered registry scope (empty "initial") in
    let accepts revision generation =
      Registry.accepts_revision registry id ~revision ~generation
    in
    assert (accepts 1L 1L);
    assert (not (accepts 0L 0L));
    change (Registration.set registration (empty "ordinary update"));
    for _ = 1 to 2 do
      let request = next registry in
      Registry.complete registry (response request)
    done;
    assert (not (accepts 2L 1L));
    (match next registry with
     | Publish (_, 2L) -> ()
     | _ -> assert false);
    assert (accepts 1L 1L && accepts 2L 1L);
    Registry.complete registry Ack;
    assert (not (accepts 1L 1L));
    change (Registration.reset registration (empty "reset"));
    assert (not (accepts 2L 1L));
    for _ = 1 to 2 do
      let request = next registry in
      Registry.complete registry (response request)
    done;
    (match next registry with
     | Publish (_, 3L) -> ()
     | _ -> assert false);
    assert (accepts 3L 2L);
    change (Registration.reset registration (empty "reset during publish"));
    assert (not (accepts 3L 2L));
    Registry.complete registry Ack;
    drain registry ~f:(fun _ -> ());
    assert (accepts 4L 3L);
    let handle = Registration.handle registration in
    assert (Resource.Expert.belongs_to handle ~owner:(Registry.Expert.owner registry));
    assert (
      not (Resource.Expert.belongs_to handle ~owner:(Resource.Expert.Owner.create ())));
    Registration.release registration;
    assert (not (accepts 4L 3L));
    drain registry ~f:(fun _ -> ()));
  print_endline "only live resource revisions from the current reset epoch are accepted";
  [%expect {| only live resource revisions from the current reset epoch are accepted |}]
;;

let%expect_test "coalescing, stable handle and charged retention after publication" =
  with_registry (fun scope registry _ ->
    let registration = registered registry scope (empty "initial") in
    let handle = Registration.handle registration in
    let desired = ref (empty "next") in
    for index = 1 to 1000 do
      desired := empty (Int.to_string index);
      change (Registration.set registration !desired)
    done;
    assert (not (Registration.is_published registration));
    let uploads = ref [] in
    let bytes = ref [] in
    drain registry ~f:(function
      | Begin update -> uploads := begin_info update :: !uploads
      | Chunk (_, _, _, chunk) -> bytes := chunk :: !bytes
      | _ -> ());
    assert (
      String.equal (String.concat (List.rev !bytes)) (Data.Expert.encode !desired |> ok));
    assert (Registration.is_published registration);
    assert (Resource.equal handle (Registration.handle registration));
    assert (snd (Registry.Expert.counts registry) = Data.Expert.retained_bytes !desired);
    print_s [%sexp (List.rev !uploads : (int64 * int64 * int64) list)];
    change (Registration.set registration !desired);
    assert (Option.is_none (Registry.next_request registry));
    Registration.release registration;
    drain registry ~f:(function
      | Release _ -> ()
      | _ -> failwith "unexpected upload");
    print_s [%sexp (Registry.Expert.counts registry : int * int)]);
  [%expect
    {|
    ((1 2 1))
    (0 0)
    |}]
;;

let%expect_test "coalesced resets never skip generations, including in-flight resets" =
  with_registry (fun scope registry _ ->
    let registration = registered registry scope (empty "initial") in
    let handle = Registration.handle registration in
    change (Registration.set registration (empty "in flight"));
    let pending = next registry in
    let starts =
      ref
        (match pending with
         | Begin u -> [ begin_info u ]
         | _ -> assert false)
    in
    for _ = 1 to 100 do
      change (Registration.reset registration (empty "latest reset"))
    done;
    Registry.complete registry (response pending);
    let reset_again = ref false in
    drain registry ~f:(function
      | Begin update ->
        starts := begin_info update :: !starts;
        if not !reset_again
        then (
          reset_again := true;
          change (Registration.reset registration (empty "reset while reset is pending")))
      | _ -> ());
    print_s [%sexp (List.rev !starts : (int64 * int64 * int64) list)];
    assert (Registration.is_published registration);
    assert (Resource.equal handle (Registration.handle registration)));
  [%expect {| ((1 2 1) (2 3 2) (3 4 3)) |}]
;;

let%expect_test
    "rejected update preserves registration, aborts, and permits explicit retry"
  =
  with_registry (fun scope registry _ ->
    let registration = registered registry scope (empty "initial") in
    let handle = Registration.handle registration in
    let desired = empty "replacement" in
    change (Registration.set registration desired);
    let begin_ = next registry in
    Registry.complete registry (response begin_);
    let chunk = next registry in
    Registry.complete registry (response chunk);
    (match next registry with
     | Publish _ -> ()
     | _ -> assert false);
    Registry.complete registry (Failed Invalid_data);
    assert (not (Registration.is_released registration));
    assert (not (Registration.is_published registration));
    print_s [%sexp (Registration.error registration : Registry.Error.t option)];
    (match next registry with
     | Abort _ -> ()
     | _ -> assert false);
    Registry.complete registry Ack;
    assert (Option.is_none (Registry.next_request registry));
    change (Registration.reset registration desired);
    drain registry ~f:(function
      | Begin update -> print_s [%sexp (begin_info update : int64 * int64 * int64)]
      | _ -> ());
    assert (Registration.is_published registration);
    assert (Option.is_none (Registration.error registration));
    assert (Resource.equal handle (Registration.handle registration)));
  [%expect
    {|
    ((Native Invalid_data))
    (1 2 2)
    |}]
;;

let%expect_test "begin failure does not abort an unstarted stage and retry is explicit" =
  with_registry (fun scope registry _ ->
    let registration = registered registry scope (empty "initial") in
    let desired = empty "next" in
    change (Registration.set registration desired);
    (match next registry with
     | Begin _ -> ()
     | _ -> assert false);
    Registry.complete registry (Failed Resource_limit);
    assert (Option.is_none (Registry.next_request registry));
    change (Registration.set registration desired);
    drain registry ~f:(function
      | Begin update -> print_s [%sexp (begin_info update : int64 * int64 * int64)]
      | _ -> ());
    assert (Registration.is_published registration));
  [%expect {| (1 2 1) |}]
;;

let%expect_test
    "cancellation at all upload boundaries suppresses callbacks and retires late IDs"
  =
  for cancel_at = 0 to 4 do
    with_registry (fun root registry _ ->
      let scope = Scope.child root ~name:"chart" |> ok in
      let callbacks = ref 0 in
      Registry.register registry ~scope (empty "cancel") ~on_result:(fun _ ->
        incr callbacks);
      if cancel_at = 0
      then Scope.cancel scope
      else
        for index = 1 to cancel_at do
          let request = next registry in
          if index = cancel_at then Scope.cancel scope;
          Registry.complete registry (response request)
        done;
      drain registry ~f:(function
        | Release _ -> ()
        | _ -> failwith "upload after release");
      assert (!callbacks = 0);
      assert ([%equal: int * int] (Registry.Expert.counts registry) (0, 0)))
  done;
  print_endline "cancelled at every boundary";
  [%expect {| cancelled at every boundary |}]
;;

let%expect_test
    "initial failure completes once, released setters and late shutdown are safe"
  =
  with_registry (fun scope registry _ ->
    let results = ref [] in
    Registry.register registry ~scope (empty "fails") ~on_result:(fun r ->
      results := r :: !results);
    let create = next registry in
    Registry.complete registry (response create);
    ignore (next registry : Wire.Request.t);
    Registry.complete registry (Failed Resource_limit);
    drain registry ~f:(function
      | Release _ -> ()
      | _ -> assert false);
    print_s [%sexp (List.map !results ~f:Result.error : Registry.Error.t option list)];
    let registration = registered registry scope (empty "works") in
    change (Registration.set registration (empty "pending shutdown"));
    ignore (next registry : Wire.Request.t);
    Registry.close registry;
    Registry.complete registry Ack;
    print_s
      [%sexp
        (Registration.set registration (empty "closed")
         : (unit, Registry.Error.t) Result.t)];
    assert (Registration.is_released registration);
    assert ([%equal: int * int] (Registry.Expert.counts registry) (0, 0)));
  [%expect
    {|
    (((Native Resource_limit)))
    (Error (Native Closed))
    |}]
;;

let%expect_test "foreign scope is rejected before native allocation" =
  with_registry (fun _ registry _ ->
    with_registry (fun foreign _ _ ->
      Registry.register
        registry
        ~scope:foreign
        (empty "foreign")
        ~on_result:(fun result ->
          print_s [%sexp (Result.error result : Registry.Error.t option)])));
  [%expect {| (Wrong_scope) |}]
;;

let large_scene description =
  let series =
    Data.Series.create
      ~id:(Data.Series_id.of_int64 1L |> ok)
      ~name:description
      (List.init 100_000 ~f:(fun index ->
         Data.Point.create
           ~id:(Data.Datum_id.of_int64 (Int64.of_int (index + 1)) |> ok)
           ~x:(Float.of_int index)
           ~y:(Some (Float.of_int index))
           ()
         |> ok))
    |> ok
  in
  Data.line [ series ] |> ok
;;

let%expect_test
    "100k points cross transport chunks; retention admission and setter rollback"
  =
  with_registry (fun scope registry _ ->
    let large = large_scene "large" in
    let registration = registered registry scope large in
    let next_scene = large_scene "updated" in
    change (Registration.set registration next_scene);
    let chunks = ref [] in
    drain registry ~f:(function
      | Chunk (_, _, offset, bytes) ->
        assert (String.length bytes <= Wire.max_chunk_bytes);
        assert (Int64.to_int_exn offset = List.sum (module Int) !chunks ~f:String.length);
        chunks := bytes :: !chunks
      | _ -> ());
    assert (List.length !chunks > 1);
    assert (
      String.equal (String.concat (List.rev !chunks)) (Data.Expert.encode next_scene |> ok));
    let results = ref [] in
    for _ = 1 to 8 do
      Registry.register registry ~scope large ~on_result:(fun r ->
        results := r :: !results)
    done;
    assert (
      List.exists !results ~f:(function
        | Error (Native Resource_limit) -> true
        | _ -> false));
    let before = Registry.Expert.counts registry in
    print_s
      [%sexp
        (Registration.set registration (large_scene "over quota")
         : (unit, Registry.Error.t) Result.t)];
    assert ([%equal: int * int] before (Registry.Expert.counts registry));
    assert (Data.equal next_scene (Registration.data registration |> Option.value_exn));
    Scope.cancel scope;
    drain registry ~f:(function
      | Release _ -> ()
      | _ -> assert false);
    print_s [%sexp (Registry.Expert.counts registry : int * int)]);
  [%expect
    {|
    (Error (Native Resource_limit))
    (0 0)
    |}]
;;

let%expect_test "concurrent registrations share four staging slots and all make progress" =
  with_registry (fun scope registry _ ->
    let results = ref [] in
    for index = 1 to 12 do
      Registry.register
        registry
        ~scope
        (empty (Int.to_string index))
        ~on_result:(fun r -> results := r :: !results)
    done;
    let stages = ref 0 in
    let maximum = ref 0 in
    drain registry ~f:(function
      | Begin _ ->
        incr stages;
        maximum := Int.max !maximum !stages;
        assert (!stages <= 4)
      | Publish _ -> decr stages
      | _ -> ());
    assert (
      !stages = 0 && List.length !results = 12 && List.for_all !results ~f:Result.is_ok);
    print_s [%sexp (!maximum : int)]);
  [%expect {| 4 |}]
;;

let%expect_test "registration count limit and cleanup priority" =
  with_registry (fun scope registry _ ->
    let registration = registered registry scope (empty "first") in
    for _ = 2 to 256 do
      Registry.register registry ~scope (empty "queued") ~on_result:(fun _ -> ())
    done;
    Registry.register registry ~scope (empty "overflow") ~on_result:(fun r ->
      print_s [%sexp (Result.error r : Registry.Error.t option)]);
    Registration.release registration;
    (match next registry with
     | Release _ -> ()
     | _ -> failwith "cleanup must precede queued allocations");
    Registry.complete registry Ack;
    Scope.cancel scope;
    drain registry ~f:(fun _ -> assert false);
    print_s [%sexp (Registry.Expert.counts registry : int * int)]);
  [%expect
    {|
    ((Native Resource_limit))
    (0 0)
    |}]
;;

let%expect_test "reverting to the accepted snapshot waits for an older in-flight upload" =
  with_registry (fun scope registry _ ->
    let original = empty "original" in
    let registration = registered registry scope original in
    change (Registration.set registration (empty "temporary"));
    let pending = next registry in
    change (Registration.set registration original);
    assert (not (Registration.is_published registration));
    Registry.complete registry (response pending);
    let revisions = ref [] in
    let bytes = ref [] in
    drain registry ~f:(function
      | Begin update ->
        revisions := update.revision :: !revisions;
        bytes := []
      | Chunk (_, _, _, chunk) -> bytes := chunk :: !bytes
      | _ -> ());
    assert (Registration.is_published registration);
    assert (
      String.equal (String.concat (List.rev !bytes)) (Data.Expert.encode original |> ok));
    print_s [%sexp (List.rev !revisions : int64 list)]);
  [%expect {| (3) |}]
;;

let%expect_test "view observations reject retired resources and forged pre-data success" =
  with_registry (fun scope registry _ ->
    let registration = registered registry scope (empty "initial") in
    let accepts ?(source = Some id) ?(revision = 0L) ?(generation = 0L) observation =
      Registry.accepts_event
        registry
        source
        ~data_revision:revision
        ~data_generation:generation
        observation
    in
    assert (accepts (Failed Unavailable_data));
    assert (accepts ~source:None (Failed Wrong_application));
    assert (not (accepts ~source:None (Failed Unavailable_data)));
    let metrics : Gpuio_protocol.Chart_view_wire.Metrics.t =
      { source_values = 1L
      ; retained_values = 1L
      ; mesh_vertices = 6L
      ; quads = 0L
      ; bytes = 500L
      }
    in
    let selected =
      Gpuio_protocol.Chart_view_wire.Observation.Selection_changed
        (Some (Gpuio_protocol.Chart_selection_wire.Slice 7L))
    in
    assert (not (accepts selected));
    assert (not (accepts ~source:None ~revision:1L ~generation:1L selected));
    assert (accepts ~revision:1L ~generation:1L selected);
    assert (
      not (accepts ~revision:1L ~generation:1L (Selection_changed (Some (Slice 0L)))));
    assert (not (accepts (Ready metrics)));
    assert (accepts ~revision:1L ~generation:1L (Ready metrics));
    assert (
      not
        (accepts
           ~revision:1L
           ~generation:1L
           (Ready { metrics with bytes = Int64.max_value })));
    change (Registration.reset registration (empty "new identity"));
    assert (not (accepts ~revision:1L ~generation:1L (Ready metrics)));
    assert (not (accepts ~revision:1L ~generation:1L selected));
    Registration.release registration;
    assert (not (accepts (Failed Unavailable_data)));
    Registry.close registry;
    assert (not (accepts ~source:None (Failed Wrong_application))));
  print_endline
    "pre-data failure, valid publication, reset, release and shutdown fences pass";
  [%expect
    {| pre-data failure, valid publication, reset, release and shutdown fences pass |}]
;;

let%expect_test "selection and clear observations each retire on reset, release or close" =
  let verify retire =
    with_registry (fun scope registry _ ->
      let registration = registered registry scope (empty "selection publication") in
      let accepts observation =
        Registry.accepts_event
          registry
          (Some id)
          ~data_revision:1L
          ~data_generation:1L
          observation
      in
      let observations =
        [ Gpuio_protocol.Chart_view_wire.Observation.Selection_changed None
        ; Selection_changed (Some (Gpuio_protocol.Chart_selection_wire.Slice 7L))
        ]
      in
      assert (List.for_all observations ~f:accepts);
      retire registration registry;
      assert (List.for_all observations ~f:(fun event -> not (accepts event))))
  in
  verify (fun registration _ ->
    change (Registration.reset registration (empty "replacement")));
  verify (fun registration _ -> Registration.release registration);
  verify (fun _ registry -> Registry.close registry);
  print_endline "fresh publication: both select and clear fenced by each retirement path";
  [%expect {| fresh publication: both select and clear fenced by each retirement path |}]
;;

let%expect_test "fatal update errors and failed aborts require a new registration" =
  let retired registry registration =
    assert (Registration.is_released registration);
    assert (not (Registration.is_published registration));
    assert (Option.is_some (Registration.error registration));
    assert (
      [%equal: (unit, Registry.Error.t) Result.t]
        (Registration.reset registration (empty "retry"))
        (Error (Native Closed)));
    drain registry ~f:(function
      | Release _ -> ()
      | _ -> failwith "retired resource attempted publication");
    assert ([%equal: int * int] (Registry.Expert.counts registry) (0, 0))
  in
  List.iter [ Wire.Error.Closed; Stale_handle; Native_failure ] ~f:(fun error ->
    with_registry (fun scope registry _ ->
      let registration = registered registry scope (empty "published") in
      change (Registration.set registration (empty "next"));
      (match next registry with
       | Begin _ -> ()
       | _ -> assert false);
      Registry.complete registry (Failed error);
      retired registry registration));
  with_registry (fun scope registry _ ->
    let registration = registered registry scope (empty "published") in
    change (Registration.set registration (empty "next"));
    let begin_ = next registry in
    Registry.complete registry (response begin_);
    (match next registry with
     | Chunk _ -> ()
     | _ -> assert false);
    Registry.complete registry (Failed Resource_limit);
    assert (not (Registration.is_released registration));
    (match next registry with
     | Abort _ -> ()
     | _ -> assert false);
    Registry.complete registry (Failed Resource_limit);
    retired registry registration);
  print_endline "fatal responses and failed abort retire the registration";
  [%expect {| fatal responses and failed abort retire the registration |}]
;;
