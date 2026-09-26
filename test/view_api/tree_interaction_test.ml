open Core
open Gpuio
module T = Tree
module S = Tree_state
module L = Tree_loading
module I = Tree_interaction

let ok = Or_error.ok_exn
let id name = T.Id.of_string name |> ok
let ids names = List.map names ~f:id

let leaf ?(disabled = false) name =
  T.Node.create ~label:name ~disabled ~children:Leaf name |> ok
;;

let branch ?(disabled = false) name children =
  T.Node.create
    ~label:name
    ~disabled
    ~children:(Branch { ids = ids children; next = End })
    name
  |> ok
;;

let tree () =
  T.create
    ~roots:(ids [ "root"; "other"; "disabled" ])
    [ id "root", branch "root" [ "a"; "folder" ]
    ; id "a", leaf "a"
    ; id "folder", branch "folder" [ "b" ]
    ; id "b", leaf "b"
    ; id "other", branch "other" []
    ; id "disabled", leaf ~disabled:true "disabled"
    ]
  |> ok
;;

let target snapshot name = I.Target.capture snapshot (id name) |> ok
let applied state snapshot request = I.apply state snapshot request |> Option.value_exn
let apply state snapshot request = applied state snapshot request |> I.Outcome.state

let show state =
  print_s
    [%sexp
      (S.active state : T.Id.t option)
    , (S.selected state : T.Id.t list)
    , (S.expanded state : T.Id.t list)]
;;

let%expect_test "ordered relative input, idempotent expansion and separate activation" =
  let tree = tree () in
  let snapshot = L.create tree |> L.snapshot in
  let state = S.create tree ~mode:Multiple () |> ok in
  let next = I.Request.navigate snapshot ~selection:(Some Replace) Next in
  let state = apply state snapshot next in
  let open_root = I.Request.set_expanded (target snapshot "root") true in
  let state = apply state snapshot open_root in
  let repeated = apply state snapshot open_root in
  assert (phys_equal state repeated);
  let state = apply state snapshot next |> fun state -> apply state snapshot next in
  show state;
  let outcome = applied state snapshot (I.Request.activate (target snapshot "a")) in
  assert (phys_equal state (I.Outcome.state outcome));
  assert (Option.is_none (I.Outcome.reveal outcome) && not (I.Outcome.focus outcome));
  (match I.Outcome.action outcome with
   | Activate item -> print_s [%sexp (item : T.Id.t)]
   | None | Move _ -> assert false);
  assert (
    Option.is_none
      (I.apply state snapshot (I.Request.set_expanded (target snapshot "a") true)));
  assert (
    Option.is_none
      (I.apply state snapshot (I.Request.select (target snapshot "disabled") Replace)));
  assert (
    Option.is_none (I.apply state snapshot (I.Request.activate (target snapshot "b"))));
  [%expect
    {|
    ((folder) (folder) (root))
    a
    |}]
;;

let%expect_test "exact selection setters preserve ordering, cursor and range anchor" =
  let tree = tree () in
  let snapshot = L.create tree |> L.snapshot in
  List.iter [ S.Mode.Single; Multiple ] ~f:(fun mode ->
    let state =
      S.create tree ~mode () |> ok |> fun state -> S.select state tree (id "root") Replace
    in
    let set state name selected =
      let outcome =
        applied state snapshot (I.Request.set_selected (target snapshot name) selected)
      in
      assert (Option.is_none (I.Outcome.reveal outcome));
      assert (not (I.Outcome.focus outcome));
      (match I.Outcome.action outcome with
       | None -> ()
       | Activate _ | Move _ -> assert false);
      let next = I.Outcome.state outcome in
      assert (Option.equal T.Id.equal (S.active state) (S.active next));
      assert (Option.equal T.Id.equal (S.anchor state) (S.anchor next));
      next
    in
    let selected = set state "other" true in
    let repeated = set selected "other" true in
    assert (List.equal T.Id.equal (S.selected selected) (S.selected repeated));
    show repeated;
    let cleared = set repeated "other" false |> fun state -> set state "other" false in
    show cleared;
    List.iter [ "disabled"; "b" ] ~f:(fun name ->
      assert (
        Option.is_none
          (I.apply cleared snapshot (I.Request.set_selected (target snapshot name) true))));
    let foreign = L.snapshot (L.create tree) in
    assert (
      Option.is_none
        (I.apply cleared foreign (I.Request.set_selected (target snapshot "root") false))));
  [%expect
    {|
    ((root) (other) ())
    ((root) () ())
    ((root) (other root) ())
    ((root) (root) ())
    |}]
;;

let%expect_test
    "reveal opens ancestors without changing selection or claiming native focus"
  =
  let tree = tree () in
  let snapshot = L.create tree |> L.snapshot in
  let state =
    S.create tree ~mode:Multiple ()
    |> ok
    |> fun state -> S.select state tree (id "other") Replace
  in
  let request = I.Request.reveal (target snapshot "b") ~focus:false in
  let outcome = applied state snapshot request in
  let state = I.Outcome.state outcome in
  show state;
  assert (Option.equal T.Id.equal (S.anchor state) (Some (id "other")));
  assert (not (I.Outcome.focus outcome));
  assert (T.Id.equal (I.Target.id (I.Outcome.reveal outcome |> Option.value_exn)) (id "b"));
  let outcome =
    applied state snapshot (I.Request.reveal (target snapshot "b") ~focus:true)
  in
  assert (I.Outcome.focus outcome);
  show (I.Outcome.state outcome);
  let collapsed =
    applied
      (I.Outcome.state outcome)
      snapshot
      (I.Request.set_expanded (target snapshot "root") false)
  in
  assert (I.Outcome.focus collapsed);
  assert (
    Option.equal T.Id.equal (S.active (I.Outcome.state collapsed)) (Some (id "root")));
  assert (
    T.Id.equal (I.Target.id (I.Outcome.reveal collapsed |> Option.value_exn)) (id "root"));
  assert (
    Option.is_none
      (I.apply state snapshot (I.Request.reveal (target snapshot "disabled") ~focus:true)));
  let tree =
    T.replace
      tree
      ~roots:(T.roots tree)
      (List.map (T.to_alist tree) ~f:(fun (key, node) ->
         ( key
         , if T.Id.equal key (id "root")
           then branch ~disabled:true "root" [ "a"; "folder" ]
           else node )))
    |> ok
  in
  let state =
    S.create tree () |> ok |> fun state -> S.reveal state tree (id "b") ~focus:true
  in
  show state;
  [%expect
    {|
    ((other) (other) (folder root))
    ((b) (other) (folder root))
    ((b) () (folder root))
    |}]
;;

let%expect_test
    "commands survive reorder and data edits but reject reset and reincarnation"
  =
  let tree = tree () in
  let loader = L.create tree in
  let snapshot = L.snapshot loader in
  let state = S.create tree () |> ok in
  let captured = target snapshot "other" in
  let request = I.Request.reveal captured ~focus:true in
  let tree = T.set_data tree ~id:(id "other") "updated" |> ok in
  let tree =
    T.replace tree ~roots:(ids [ "disabled"; "other"; "root" ]) (T.to_alist tree) |> ok
  in
  L.update loader tree |> ok;
  let snapshot = L.snapshot loader in
  assert (I.Target.equal captured (target snapshot "other"));
  let state = apply state snapshot request in
  assert (Option.equal T.Id.equal (S.active state) (Some (id "other")));
  let deleted =
    T.replace
      tree
      ~roots:(ids [ "disabled"; "root" ])
      (List.filter (T.to_alist tree) ~f:(fun (key, _) ->
         not (T.Id.equal key (id "other"))))
    |> ok
  in
  let restored = T.replace deleted ~roots:(T.roots tree) (T.to_alist tree) |> ok in
  L.update loader restored |> ok;
  assert (Option.is_none (I.apply state (L.snapshot loader) request));
  let fresh_request = I.Request.reveal (target (L.snapshot loader) "other") ~focus:true in
  let relative = I.Request.navigate (L.snapshot loader) ~selection:None Next in
  L.reset loader restored |> ok;
  List.iter [ fresh_request; relative ] ~f:(fun request ->
    assert (Option.is_none (I.apply state (L.snapshot loader) request)));
  let other_loader = L.create restored in
  assert (Option.is_none (I.apply state (L.snapshot other_loader) fresh_request));
  assert (Result.is_error (I.Target.capture (L.snapshot loader) (id "absent")));
  print_endline
    "reorder/data retain target; coalesced delete/reinsert, reset and foreign owner \
     reject";
  [%expect
    {| reorder/data retain target; coalesced delete/reinsert, reset and foreign owner reject |}]
;;

let%expect_test
    "move proposals require approval and revalidation after hierarchy or visibility \
     changes"
  =
  let tree = tree () in
  let loader = L.create tree in
  let snapshot = L.snapshot loader in
  let state = S.create tree ~expanded:(ids [ "root"; "folder" ]) () |> ok in
  let request source destination placement =
    I.Request.move
      ~source:(target snapshot source)
      ~destination:(target snapshot destination)
      placement
  in
  let outcome = applied state snapshot (request "a" "other" Inside) in
  assert (phys_equal state (I.Outcome.state outcome));
  assert (phys_equal tree (L.Snapshot.tree (L.snapshot loader)));
  let proposal =
    match I.Outcome.action outcome with
    | Move move -> move
    | None | Activate _ -> assert false
  in
  assert (I.Move.is_current proposal snapshot ~state);
  assert (I.Placement.equal (I.Move.placement proposal) Inside);
  assert (T.Id.equal (I.Target.id (I.Move.source proposal)) (id "a"));
  List.iter
    [ request "root" "b" Before
    ; request "a" "a" After
    ; request "other" "a" Inside
    ; request "a" "disabled" Before
    ]
    ~f:(fun request -> assert (Option.is_none (I.apply state snapshot request)));
  let collapsed = S.toggle_expanded state tree (id "root") in
  assert (not (I.Move.is_current proposal snapshot ~state:collapsed));
  let nodes =
    List.map (T.to_alist tree) ~f:(fun (key, node) ->
      key, if T.Id.equal key (id "other") then leaf "other" else node)
  in
  L.update loader (T.replace tree ~roots:(T.roots tree) nodes |> ok) |> ok;
  assert (not (I.Move.is_current proposal (L.snapshot loader) ~state));
  let allowed = applied state snapshot (request "a" "other" Before) in
  (match I.Outcome.action allowed with
   | Move move -> assert (T.Id.equal (I.Target.id (I.Move.destination move)) (id "other"))
   | None | Activate _ -> assert false);
  print_endline
    "no silent mutation; cycle/self/leaf/disabled reject; delayed approval rechecks \
     collapse and topology";
  [%expect
    {| no silent mutation; cycle/self/leaf/disabled reject; delayed approval rechecks collapse and topology |}]
;;

let%expect_test
    "maximum-depth reveal is bounded and interaction targets retain no payload"
  =
  let rec nodes depth =
    if depth > 128
    then []
    else (
      let name = Int.to_string depth in
      ( id name
      , if depth = 128 then leaf name else branch name [ Int.to_string (depth + 1) ] )
      :: nodes (depth + 1))
  in
  let tree = T.create ~roots:[ id "1" ] (nodes 1) |> ok in
  let snapshot = L.create tree |> L.snapshot in
  let outcome =
    applied
      (S.create tree () |> ok)
      snapshot
      (I.Request.reveal (target snapshot "128") ~focus:true)
  in
  assert (List.length (S.expanded (I.Outcome.state outcome)) = 127);
  assert (List.length (S.visible (I.Outcome.state outcome)) = 128);
  let weak = Stdlib.Weak.create 1 in
  let[@inline never] capture () =
    let payload = Bytes.create 1_048_576 in
    Stdlib.Weak.set weak 0 (Some payload);
    let node = T.Node.create ~label:"data" ~children:Leaf payload |> ok in
    let loader = L.create (T.create ~roots:[ id "data" ] [ id "data", node ] |> ok) in
    let snapshot = L.snapshot loader in
    let captured = target snapshot "data" in
    I.Request.reveal captured ~focus:true, captured
  in
  let request, captured = capture () in
  Gc.full_major ();
  Gc.full_major ();
  assert (not (Stdlib.Weak.check weak 0));
  ignore (Sys.opaque_identity request : I.Request.t);
  assert (T.Id.equal (I.Target.id captured) (id "data"));
  print_endline "128-level reveal; captured request/target do not retain 1 MiB payload";
  [%expect {| 128-level reveal; captured request/target do not retain 1 MiB payload |}]
;;

let%expect_test
    "child paging preserves interaction identity while retiring the page target"
  =
  let node =
    T.Node.create ~label:"lazy" ~children:(Branch { ids = []; next = More None }) () |> ok
  in
  let tree = T.create ~roots:[ id "lazy" ] [ id "lazy", node ] |> ok in
  let loader = L.create tree in
  let before = L.snapshot loader in
  let interaction = target before "lazy" in
  let page_target = L.Snapshot.target before (id "lazy") |> ok in
  assert (L.request loader (id "lazy") |> ok);
  let request = L.take loader |> Option.value_exn in
  let child = T.Node.create ~label:"child" ~children:Leaf () |> ok in
  ignore
    (L.complete
       loader
       request
       { roots = [ id "child" ]; nodes = [ id "child", child ]; next = End }
     |> ok
     : L.Completion.t);
  let after = L.snapshot loader in
  assert (I.Target.is_current interaction after);
  assert (not (L.Snapshot.is_current after page_target));
  let state = S.create (L.Snapshot.tree after) () |> ok in
  let state = apply state after (I.Request.set_expanded interaction true) in
  assert (List.equal T.Id.equal (S.visible state) (ids [ "lazy"; "child" ]));
  print_endline "loaded branch identity survives; obsolete child page token does not";
  [%expect {| loaded branch identity survives; obsolete child page token does not |}]
;;
