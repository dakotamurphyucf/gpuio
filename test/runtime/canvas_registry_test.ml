open Core
module Registry = Gpuio_eio.Canvas_registry
module Registration = Registry.Registration
module Scope = Gpuio_eio.Scope
module Inbox = Gpuio_eio.Inbox
module Wire = Gpuio_protocol.Wire.Canvas
module Scene = Gpuio.Canvas_scene
module Asset = Gpuio.Asset
module Geometry = Gpuio.Canvas_geometry
module Resource = Gpuio.Canvas_resource

let ok = Or_error.ok_exn
let id = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok
let empty description = Scene.create ~description [] |> ok

let with_registry f =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:8 () in
      let root = Scope.Expert.create ~sw ~inbox ~max_tasks:8 in
      let asset_owner = Asset.Expert.Owner.create () in
      let registry = Registry.create ~scope:root ~asset_owner ~wake:(fun () -> ()) in
      Exn.protect
        ~f:(fun () -> f root registry asset_owner)
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

let%expect_test "coalescing, stable handle and charged retention after publication" =
  with_registry (fun scope registry owner ->
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
      String.equal
        (String.concat (List.rev !bytes))
        (Scene.Expert.encode !desired ~asset_owner:owner |> ok));
    assert (Registration.is_published registration);
    assert (Scene.Handle.equal handle (Registration.handle registration));
    assert (snd (Registry.Expert.counts registry) = Scene.Expert.retained_bytes !desired);
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
    assert (Scene.Handle.equal handle (Registration.handle registration)));
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
    Registry.complete registry (Failed Stale_resource);
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
    assert (Scene.Handle.equal handle (Registration.handle registration)));
  [%expect
    {|
    ((Native Stale_resource))
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
      let scope = Scope.child root ~name:"canvas" |> ok in
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

let image_scene owner =
  let asset = Asset.Expert.handle ~owner ~id ~format:Png in
  let resource = Resource.image ~id:(Resource.Id.of_int64 1L |> ok) asset |> ok in
  let bounds = Geometry.Rect.create ~x:0. ~y:0. ~width:10. ~height:10. |> ok in
  let item =
    Scene.Item.create
      ~id:(Scene.Item_id.of_int64 1L |> ok)
      (Scene.Drawing.image resource ~bounds)
    |> ok
  in
  Scene.create ~description:"image" [ item ] |> ok
;;

let%expect_test "foreign asset and scope are rejected before native allocation" =
  with_registry (fun scope registry _ ->
    let print result = print_s [%sexp (Result.error result : Registry.Error.t option)] in
    Registry.register
      registry
      ~scope
      (image_scene (Asset.Expert.Owner.create ()))
      ~on_result:print;
    let registration = registered registry scope (empty "valid") in
    print (Registration.set registration (image_scene (Asset.Expert.Owner.create ())));
    with_registry (fun foreign_scope _ _ ->
      Registry.register
        registry
        ~scope:foreign_scope
        (empty "wrong scope")
        ~on_result:print);
    assert (Option.is_none (Registry.next_request registry));
    assert (Registration.is_published registration));
  [%expect
    {|
    (Wrong_application)
    (Wrong_application)
    (Wrong_scope)
    |}]
;;

let large_scene description =
  let bounds = Geometry.Rect.create ~x:0. ~y:0. ~width:10. ~height:10. |> ok in
  let paint = Scene.Paint.create ~fill:(Gpuio.Color.rgb_exn 0x102030) () |> ok in
  Scene.create
    ~description
    (List.init 20_000 ~f:(fun index ->
       Scene.Item.create
         ~id:(Scene.Item_id.of_int64 (Int64.of_int (index + 1)) |> ok)
         (Scene.Drawing.rectangle bounds ~paint)
       |> ok))
  |> ok
;;

let%expect_test
    "20k items cross transport chunks; retention admission and setter rollback"
  =
  with_registry (fun scope registry owner ->
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
      String.equal
        (String.concat (List.rev !chunks))
        (Scene.Expert.encode next_scene ~asset_owner:owner |> ok));
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
    assert (Scene.equal next_scene (Registration.scene registration |> Option.value_exn));
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
  with_registry (fun scope registry owner ->
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
      String.equal
        (String.concat (List.rev !bytes))
        (Scene.Expert.encode original ~asset_owner:owner |> ok));
    print_s [%sexp (List.rev !revisions : int64 list)]);
  [%expect {| (3) |}]
;;
