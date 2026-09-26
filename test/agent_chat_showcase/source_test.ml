open Core
module D = Gpuio_agent_chat_runtime.Source_data
module T = Gpuio.Tree
module L = Gpuio.Tree_loading
module I = Gpuio.Tree_interaction

let ok = Or_error.ok_exn

let load loader name attempt =
  (if attempt = 1 then L.request else L.retry) loader (D.id name) |> ok |> ignore;
  let request = L.take loader |> Option.value_exn in
  match D.load ~attempt request with
  | Ok page -> L.complete loader request page |> ok |> ignore
  | Error error -> L.fail loader request error |> ignore
;;

let%expect_test "lazy sample failure requires retry and then appends stable children" =
  let loader = L.create (D.initial ()) in
  load loader "project" 1;
  load loader "notes" 1;
  let snapshot = L.snapshot loader in
  print_s
    [%sexp
      (T.length (L.Snapshot.tree snapshot) : int)
    , (L.Snapshot.failed_count snapshot : int)];
  print_s [%sexp (L.request loader (D.id "notes") |> ok : bool)];
  load loader "notes" 2;
  let snapshot = L.snapshot loader in
  print_s
    [%sexp
      (T.length (L.Snapshot.tree snapshot) : int)
    , (L.Snapshot.failed_count snapshot : int)];
  [%expect
    {|
    (6 1)
    false
    (8 0)
    |}]
;;

let%expect_test
    "approval preserves source identity but rejects reset and incomplete destinations"
  =
  let loader = L.create (D.initial ()) in
  load loader "project" 1;
  let snapshot = L.snapshot loader in
  let tree = L.Snapshot.tree snapshot in
  let state =
    Gpuio.Tree_state.create tree ~expanded:[ D.id "project"; D.id "archive" ] () |> ok
  in
  let proposal destination =
    let source = I.Target.capture snapshot (D.id "app") |> ok in
    let destination = I.Target.capture snapshot (D.id destination) |> ok in
    let outcome =
      I.apply state snapshot (I.Request.move ~source ~destination Inside)
      |> Option.value_exn
    in
    match I.Outcome.action outcome with
    | Move proposal -> proposal
    | None | Activate _ -> assert false
  in
  let move = proposal "archive" in
  let updated = D.approve snapshot ~state move |> ok in
  print_s
    [%sexp ((T.position updated (D.id "app") |> Option.value_exn).parent : T.Id.t option)];
  print_s
    [%sexp
      (Option.equal
         Int64.equal
         (T.Expert.incarnation tree (D.id "app"))
         (T.Expert.incarnation updated (D.id "app"))
       : bool)];
  print_s
    [%sexp (D.approve snapshot ~state (proposal "notes") |> Or_error.is_error : bool)];
  let fresh = L.snapshot (L.create tree) in
  print_s [%sexp (D.approve fresh ~state move |> Or_error.is_error : bool)];
  [%expect
    {|
    (archive)
    true
    true
    true
    |}]
;;

let%expect_test
    "normal startup stays small and the explicit fixture contains 100000 nodes"
  =
  print_s
    [%sexp
      (T.length (D.initial ()) : int)
    , (T.length (D.empty ()) : int)
    , (T.length (D.large ()) : int)];
  [%expect {| (3 0 100000) |}]
;;
