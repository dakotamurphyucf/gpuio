open Core
open Gpuio
module T = Tree
module S = Tree_state

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
    ~roots:(ids [ "root"; "tail" ])
    [ id "root", branch "root" [ "a"; "folder"; "disabled"; "z" ]
    ; id "a", leaf "a"
    ; id "folder", branch "folder" [ "b"; "c" ]
    ; id "b", leaf "b"
    ; id "c", leaf "c"
    ; id "disabled", leaf ~disabled:true "disabled"
    ; id "z", leaf "z"
    ; id "tail", leaf "tail"
    ]
  |> ok
;;

let show t =
  print_s
    [%sexp
      (S.visible t : T.Id.t list)
    , (S.selected t : T.Id.t list)
    , (S.expanded t : T.Id.t list)
    , (S.active t : T.Id.t option)
    , (S.anchor t : T.Id.t option)]
;;

let%expect_test
    "collapse preserves descendant preferences and repairs only the logical cursor"
  =
  let tree = tree () in
  let state =
    S.create tree ~mode:Multiple ~expanded:(ids [ "root"; "folder" ]) () |> ok
  in
  let state = S.select state tree (id "b") Replace in
  let visible = S.visible state in
  let state = S.select state tree (id "c") Toggle in
  assert (phys_equal visible (S.visible state));
  let state = S.toggle_expanded state tree (id "root") in
  show state;
  assert (phys_equal state (S.select state tree (id "b") Replace));
  assert (phys_equal state (S.toggle_expanded state tree (id "folder")));
  let state = S.toggle_expanded state tree (id "root") in
  show state;
  assert (phys_equal state (S.focus state tree (id "disabled")));
  assert (phys_equal state (S.toggle_expanded state tree (id "a")));
  [%expect
    {|
    ((root tail) (b c) (folder) (root) (c))
    ((root a folder b c disabled z tail) (b c) (folder root) (root) (c))
    |}]
;;

let%expect_test
    "range selection uses the visible order, skips disabled rows and has a stable anchor"
  =
  let tree = tree () in
  let initial =
    S.create tree ~mode:Multiple ~expanded:(ids [ "root"; "folder" ]) () |> ok
  in
  let state = S.select initial tree (id "a") Replace in
  let state = S.select state tree (id "z") (Range { extend = false }) in
  show state;
  let state = S.select state tree (id "b") (Range { extend = false }) in
  show state;
  let state = S.select state tree (id "tail") Toggle in
  let state = S.select state tree (id "z") (Range { extend = true }) in
  show state;
  let state = S.select initial tree (id "b") Replace in
  let state = S.toggle_expanded state tree (id "folder") in
  let state = S.select state tree (id "z") (Range { extend = false }) in
  show state;
  [%expect
    {|
    ((root a folder b c disabled z tail) (a b c folder z) (folder root) (z) (a))
    ((root a folder b c disabled z tail) (a b folder) (folder root) (b) (a))
    ((root a folder b c disabled z tail) (a b folder tail z) (folder root)
     (z) (tail))
    ((root a folder disabled z tail) (folder z) (root) (z) (folder))
    |}]
;;

let%expect_test "programmatic policy is explicit and invalid preferences reject" =
  let tree = tree () in
  let initial = S.create tree () |> ok in
  List.iter
    [ S.create tree ~selected:(ids [ "a"; "b" ]) ()
    ; S.create tree ~mode:Multiple ~selected:(ids [ "a"; "a" ]) ()
    ; S.create tree ~expanded:(ids [ "a" ]) ()
    ; S.create tree ~expanded:(ids [ "root"; "root" ]) ()
    ; S.create tree ~selected:(ids [ "absent" ]) ()
    ]
    ~f:(fun result -> assert (Result.is_error result));
  let state = S.with_selected initial tree (ids [ "disabled" ]) |> ok in
  assert (S.is_selected state (id "disabled"));
  assert (phys_equal state (S.select state tree (id "disabled") Toggle));
  let state = S.with_mode state tree Multiple in
  let state = S.with_selected state tree (ids [ "b"; "tail"; "a" ]) |> ok in
  let single = S.with_mode state tree Single in
  print_s [%sexp (S.selected single : T.Id.t list)];
  let state = S.focus state tree (id "tail") in
  print_s [%sexp (S.selected (S.with_mode state tree Single) : T.Id.t list)];
  let single = S.select single tree (id "tail") Toggle in
  let single = S.select single tree (id "tail") Toggle in
  print_s [%sexp (S.selected single : T.Id.t list)];
  assert (Option.is_none (S.anchor (S.with_selected single tree [] |> ok)));
  assert (Result.is_error (S.with_expanded state tree [ id "tail" ]));
  [%expect
    {|
    (a)
    (tail)
    (tail)
    |}]
;;

let%expect_test
    "reorder, leaf conversion, deletion and reincarnation normalize deterministically"
  =
  let original = tree () in
  let state =
    S.create original ~mode:Multiple ~expanded:(ids [ "root"; "folder" ]) () |> ok
  in
  let state = S.select state original (id "b") Replace in
  let data_only = T.set_data original ~id:(id "b") "streamed data" |> ok in
  assert (phys_equal state (S.reconcile state data_only));
  let nodes =
    T.to_alist original
    |> List.map ~f:(fun (key, node) ->
      ( key
      , if T.Id.equal key (id "root")
        then branch "root" [ "z"; "folder"; "a"; "disabled" ]
        else node ))
  in
  let reordered = T.replace original ~roots:(ids [ "tail"; "root" ]) nodes |> ok in
  let state = S.reconcile state reordered in
  show state;
  let nodes =
    T.to_alist reordered
    |> List.filter_map ~f:(fun (key, node) ->
      if List.mem (ids [ "b"; "c" ]) key ~equal:T.Id.equal
      then None
      else Some (key, if T.Id.equal key (id "folder") then leaf "folder" else node))
  in
  let removed = T.replace reordered ~roots:(T.roots reordered) nodes |> ok in
  let repaired = S.reconcile state removed in
  show repaired;
  let restored =
    T.replace removed ~roots:(T.roots reordered) (T.to_alist reordered) |> ok
  in
  let direct = S.reconcile state restored in
  assert (not (S.is_selected direct (id "b")));
  assert (Option.is_none (S.anchor direct));
  let empty = T.replace restored ~roots:[] [] |> ok in
  show (S.reconcile direct empty);
  [%expect
    {|
    ((tail root z folder b c a disabled) (b) (folder root) (b) (b))
    ((tail root z folder a disabled) () (root) (folder) ())
    (() () () () ())
    |}]
;;

let%expect_test "deleted roots and disabled active rows choose a surviving neighbor" =
  let original =
    T.create
      ~roots:(ids [ "a"; "b"; "c"; "d" ])
      (List.map [ "a"; "b"; "c"; "d" ] ~f:(fun name -> id name, leaf name))
    |> ok
  in
  let state = S.create original () |> ok |> fun t -> S.focus t original (id "b") in
  let updated =
    T.replace
      original
      ~roots:(ids [ "a"; "c"; "d" ])
      [ id "a", leaf "a"; id "c", leaf ~disabled:true "c"; id "d", leaf "d" ]
    |> ok
  in
  print_s [%sexp (S.active (S.reconcile state updated) : T.Id.t option)];
  let state = S.focus state original (id "d") in
  let updated = T.replace original ~roots:(ids [ "a" ]) [ id "a", leaf "a" ] |> ok in
  print_s [%sexp (S.active (S.reconcile state updated) : T.Id.t option)];
  let disabled =
    T.replace updated ~roots:(ids [ "a" ]) [ id "a", leaf ~disabled:true "a" ] |> ok
  in
  print_s [%sexp (S.active (S.reconcile state disabled) : T.Id.t option)];
  [%expect
    {|
    (d)
    (a)
    ()
    |}]
;;

let%expect_test
    "logical keyboard traversal, branch motion and modifiers have separate effects"
  =
  let tree = tree () in
  let initial = S.create tree () |> ok in
  let actions =
    S.Navigation.
      [ Next
      ; Child
      ; Child
      ; Next
      ; Child
      ; Child
      ; Parent
      ; Parent
      ; Next
      ; Previous
      ; Last
      ; First
      ]
  in
  let _, cursors =
    List.fold_map actions ~init:initial ~f:(fun state direction ->
      let state = S.navigate state tree ~selection:(Some Replace) direction in
      state, S.active state)
  in
  print_s [%sexp (cursors : T.Id.t option list)];
  let state =
    S.create tree ~mode:Multiple ~expanded:(ids [ "root"; "folder" ]) () |> ok
  in
  let state = S.select state tree (id "b") Replace in
  let state = S.navigate state tree ~selection:(Some (Range { extend = false })) Next in
  let state = S.navigate state tree ~selection:(Some (Range { extend = false })) Next in
  let state = S.navigate state tree ~selection:None Previous in
  print_s
    [%sexp
      (S.selected state : T.Id.t list)
    , (S.active state : T.Id.t option)
    , (S.anchor state : T.Id.t option)];
  let empty = T.create ~roots:[] [] |> ok in
  let state = S.create empty () |> ok in
  List.iter actions ~f:(fun direction ->
    assert (phys_equal state (S.navigate state empty ~selection:None direction)));
  [%expect
    {|
    ((root) (root) (a) (folder) (folder) (b) (folder) (folder) (z) (folder)
     (tail) (root))
    ((b c z) (c) (b))
    |}]
;;

let%expect_test "persistent state does not retain application payloads" =
  let weak = Stdlib.Weak.create 1 in
  let[@inline never] make () =
    let data = Bytes.create 1_048_576 in
    Stdlib.Weak.set weak 0 (Some data);
    let node = T.Node.create ~label:"Payload" ~children:Leaf data |> ok in
    let tree = T.create ~roots:[ id "payload" ] [ id "payload", node ] |> ok in
    S.create tree ~selected:[ id "payload" ] () |> ok
  in
  let state = make () in
  Gc.full_major ();
  assert (not (Stdlib.Weak.check weak 0));
  assert (S.is_selected state (id "payload"));
  print_endline
    "selection and visible metadata survive; application payload is collectible";
  [%expect
    {| selection and visible metadata survive; application payload is collectible |}]
;;

let%expect_test
    "large selection shares one bounded visible snapshot through repeated focus changes"
  =
  let nodes =
    List.init T.max_nodes ~f:(fun i ->
      let name = sprintf "%06d" i in
      id name, leaf name)
  in
  let tree = T.create ~roots:(List.map nodes ~f:fst) nodes |> ok in
  let state = S.create tree ~mode:Multiple () |> ok in
  let visible = S.visible state in
  let state = S.select state tree (id "000000") Replace in
  let state = S.select state tree (id "099999") (Range { extend = false }) in
  assert (List.length (S.selected state) = T.max_nodes);
  let state =
    List.fold (List.range 0 1000) ~init:state ~f:(fun state i ->
      let state = S.focus state tree (id (sprintf "%06d" i)) in
      assert (phys_equal visible (S.visible state));
      state)
  in
  assert (List.length (S.selected state) = T.max_nodes);
  printf
    "%d logical selections, %d cached visible IDs; no row models created\n"
    (List.length (S.selected state))
    (List.length visible);
  [%expect
    {| 100000 logical selections, 100000 cached visible IDs; no row models created |}]
;;
