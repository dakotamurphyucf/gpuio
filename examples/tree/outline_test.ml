open Core
module D = Tree_example_model.Outline_data
module T = Gpuio.Tree
module L = Gpuio.Tree_loading
module S = Gpuio.Tree_state
module I = Gpuio.Tree_interaction

let ok = Or_error.ok_exn
let snapshot () = L.snapshot (L.create (D.initial ()))

let state snapshot =
  S.create (L.Snapshot.tree snapshot) ~expanded:[ D.id "inbox"; D.id "archive" ] () |> ok
;;

let propose snapshot source destination placement =
  let target name = I.Target.capture snapshot (D.id name) |> ok in
  match
    I.apply
      (state snapshot)
      snapshot
      (I.Request.move ~source:(target source) ~destination:(target destination) placement)
  with
  | Some outcome ->
    (match I.Outcome.action outcome with
     | Move move -> move
     | None | Activate _ -> assert false)
  | None -> failwith "invalid test proposal"
;;

let children tree name =
  match T.Node.children (T.find tree (D.id name) |> Option.value_exn) with
  | Leaf -> []
  | Branch { ids; _ } -> List.map ids ~f:T.Id.to_string
;;

let%expect_test
    "approved outline moves preserve payload and identity across all placements"
  =
  List.iter
    I.Placement.[ Before; After; Inside ]
    ~f:(fun placement ->
      let snapshot = snapshot () in
      let destination =
        match placement with
        | Inside -> "archive"
        | Before | After -> "release"
      in
      let proposal = propose snapshot "plan" destination placement in
      let changed = D.approve snapshot ~state:(state snapshot) proposal |> ok in
      assert (
        Option.equal
          Int64.equal
          (T.Expert.incarnation changed (D.id "plan"))
          (T.Expert.incarnation (L.Snapshot.tree snapshot) (D.id "plan")));
      assert (
        String.equal
          (T.Node.data (T.find changed (D.id "plan") |> Option.value_exn))
          "A native workspace for the next idea.");
      print_s
        [%sexp
          (placement : I.Placement.t)
        , (children changed "inbox" : string list)
        , (children changed "archive" : string list)]);
  [%expect
    {|
    (Before (notes) (plan release))
    (After (notes) (release plan))
    (Inside (notes) (release plan))
    |}]
;;

let%expect_test
    "same-parent reorder and promoting a child adjust the ordered lists atomically"
  =
  let snapshot = snapshot () in
  let changed =
    D.approve snapshot ~state:(state snapshot) (propose snapshot "plan" "notes" After)
    |> ok
  in
  print_s [%sexp (children changed "inbox" : string list)];
  let promoted =
    D.approve snapshot ~state:(state snapshot) (propose snapshot "plan" "inbox" Before)
    |> ok
  in
  print_s
    [%sexp
      (List.map (T.roots promoted) ~f:T.Id.to_string : string list)
    , (children promoted "inbox" : string list)];
  [%expect
    {|
    (notes plan)
    ((plan inbox archive) (notes))
    |}]
;;

let%expect_test "approval rejects a stale generation and a collapsed source" =
  let snapshot = snapshot () in
  let proposal = propose snapshot "plan" "archive" Inside in
  assert (
    Or_error.is_error
      (D.approve (L.snapshot (L.create (D.initial ()))) ~state:(state snapshot) proposal));
  let collapsed =
    S.toggle_expanded (state snapshot) (L.Snapshot.tree snapshot) (D.id "inbox")
  in
  assert (Or_error.is_error (D.approve snapshot ~state:collapsed proposal));
  assert (
    List.equal
      String.equal
      (children (L.Snapshot.tree snapshot) "inbox")
      [ "plan"; "notes" ]);
  [%expect {| |}]
;;
