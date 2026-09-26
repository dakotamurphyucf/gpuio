open Core
open Gpuio
module T = Tree

let ok = Or_error.ok_exn
let id s = T.Id.of_string s |> ok
let ids = List.map ~f:id
let leaf ?(label = "File") data = T.Node.create ~label ~children:Leaf data |> ok

let branch ?(next = List_paging.Boundary.End) children data =
  T.Node.create ~label:"Folder" ~children:(Branch { ids = ids children; next }) data |> ok
;;

let forest () =
  T.create
    ~roots:(ids [ "root"; "other" ])
    [ id "root", branch [ "a"; "folder" ] 0
    ; id "a", leaf 1
    ; id "folder", branch ~next:(More (Some "page-2")) [ "b" ] 2
    ; id "b", leaf 3
    ; id "other", leaf 4
    ]
  |> ok
;;

let%expect_test
    "ordered forest has stable typed identity and honest lazy hierarchy metadata"
  =
  let tree = forest () in
  print_s [%sexp (T.preorder tree : T.Id.t list)];
  List.iter [ "root"; "folder"; "b"; "other"; "absent" ] ~f:(fun name ->
    print_s [%sexp (name : string), (T.position tree (id name) : T.Position.t option)]);
  print_s [%sexp (T.ancestors tree (id "b") : T.Id.t list option)];
  assert (Option.is_none (T.ancestors tree (id "absent")));
  let empty_folder =
    T.Node.create ~label:"Empty" ~children:(Branch { ids = []; next = End }) () |> ok
  in
  assert (not (T.Children.equal (T.Node.children empty_folder) Leaf));
  assert (T.length (T.create ~roots:[] [] |> ok) = 0);
  [%expect
    {|
    (root a folder b other)
    (root (((parent ()) (depth 1) (index 0) (sibling_count (2)))))
    (folder (((parent (root)) (depth 2) (index 1) (sibling_count (2)))))
    (b (((parent (folder)) (depth 3) (index 0) (sibling_count ()))))
    (other (((parent ()) (depth 1) (index 1) (sibling_count (2)))))
    (absent ())
    ((root folder))
    |}]
;;

let%expect_test
    "complete proposed topology rejects cycles, duplicate ownership and missing nodes"
  =
  let cases =
    [ "duplicate ID", [ "a"; "a" ], [ "a", leaf (); "a", leaf () ]
    ; "duplicate root", [ "a"; "a" ], [ "a", leaf (); "b", leaf () ]
    ; "missing root", [ "b" ], [ "a", leaf () ]
    ; "missing child", [ "a" ], [ "a", branch [ "missing" ] (); "b", leaf () ]
    ; ( "duplicate child"
      , [ "a" ]
      , [ "a", branch [ "b"; "b" ] (); "b", leaf (); "c", leaf () ] )
    ; ( "multiple parents"
      , [ "a"; "b" ]
      , [ "a", branch [ "c" ] (); "b", branch [ "c" ] (); "c", leaf (); "d", leaf () ] )
    ; "root is child", [ "a" ], [ "a", branch [ "a" ] (); "b", leaf () ]
    ; "unreachable", [ "a" ], [ "a", leaf (); "b", leaf () ]
    ; "rootless cycle", [], [ "a", branch [ "b" ] (); "b", branch [ "a" ] () ]
    ; ( "disconnected cycle"
      , [ "root" ]
      , [ "root", leaf (); "a", branch [ "b" ] (); "b", branch [ "a" ] () ] )
    ]
  in
  List.iter cases ~f:(fun (name, roots, nodes) ->
    let result =
      T.create ~roots:(ids roots) (List.map nodes ~f:(fun (key, node) -> id key, node))
    in
    assert (Result.is_error result);
    print_endline name);
  [%expect
    {|
    duplicate ID
    duplicate root
    missing root
    missing child
    duplicate child
    multiple parents
    root is child
    unreachable
    rootless cycle
    disconnected cycle
    |}]
;;

let%expect_test
    "payload updates, reorder, relocation and reincarnation have separate versions"
  =
  let tree = forest () in
  let version tree name = T.Expert.incarnation tree (id name) |> Option.value_exn in
  let children_version tree name =
    T.Expert.children_revision tree (id name) |> Option.value_exn
  in
  let updated = T.set_data tree ~id:(id "b") 99 |> ok in
  assert (phys_equal (T.preorder tree) (T.preorder updated));
  assert (
    phys_equal
      (T.position tree (id "b") |> Option.value_exn)
      (T.position updated (id "b") |> Option.value_exn));
  assert (
    phys_equal
      (T.find tree (id "a") |> Option.value_exn)
      (T.find updated (id "a") |> Option.value_exn));
  assert (Int64.equal (children_version tree "folder") (children_version updated "folder"));
  let replaced =
    T.replace
      updated
      ~roots:(ids [ "other"; "root" ])
      [ id "root", branch [ "folder"; "a" ] 0
      ; id "a", leaf 1
      ; id "folder", branch ~next:(More (Some "page-2")) [ "b" ] 2
      ; id "b", leaf 99
      ; id "other", leaf 4
      ]
    |> ok
  in
  assert (Int64.equal (version tree "b") (version replaced "b"));
  assert (
    Int64.equal (children_version tree "folder") (children_version replaced "folder"));
  assert (
    not (Int64.equal (children_version tree "root") (children_version replaced "root")));
  print_s [%sexp (T.preorder replaced : T.Id.t list)];
  let moved =
    T.replace
      replaced
      ~roots:(ids [ "other"; "root" ])
      [ id "root", branch [ "folder"; "a" ] 0
      ; id "a", leaf 1
      ; id "folder", branch [] 2
      ; id "b", leaf 99
      ; id "other", branch [ "b" ] 4
      ]
    |> ok
  in
  assert (Int64.equal (version tree "b") (version moved "b"));
  print_s [%sexp (T.ancestors moved (id "b") : T.Id.t list option)];
  let removed = T.replace moved ~roots:[] [] |> ok in
  let restored = T.replace removed ~roots:(T.roots tree) (T.to_alist tree) |> ok in
  assert (not (Int64.equal (version tree "b") (version restored "b")));
  assert (Option.is_none (T.Expert.incarnation removed (id "b")));
  assert (Result.is_error (T.replace restored ~roots:(ids [ "b" ]) []));
  assert (Result.is_error (T.set_data restored ~id:(id "absent") 2));
  assert (T.find restored (id "b") |> Option.value_exn |> T.Node.data = 3);
  print_s
    [%sexp
      (List.map [ tree; updated; replaced; moved; removed; restored ] ~f:T.revision
       : int64 list)];
  [%expect
    {|
    (other root folder b a)
    ((other))
    (0 1 2 3 4 5)
    |}]
;;

let%expect_test "text, depth and aggregate metadata bounds reject without normalization" =
  List.iter
    [ ""; "\000"; "\255"; String.make 257 'x' ]
    ~f:(fun text -> assert (Result.is_error (T.Id.of_string text)));
  List.iter
    [ ""; "\000"; "\255"; String.make 4097 'x' ]
    ~f:(fun label -> assert (Result.is_error (T.Node.create ~label ~children:Leaf ())));
  let opaque =
    T.Node.create
      ~label:"Folder"
      ~children:(Branch { ids = []; next = More (Some "\000\255") })
      ()
    |> ok
  in
  assert (T.metadata_bytes (T.create ~roots:[ id "a" ] [ id "a", opaque ] |> ok) = 10);
  assert (
    Result.is_error
      (T.Node.create
         ~label:"Folder"
         ~children:(Branch { ids = []; next = More (Some (String.make 4097 'x')) })
         ()));
  let deep count =
    let nodes =
      List.init count ~f:(fun i ->
        ( Int.to_string i |> id
        , if i + 1 = count then leaf () else branch [ Int.to_string (i + 1) ] () ))
    in
    T.create ~roots:[ id "0" ] nodes
  in
  let accepted = deep T.max_depth |> ok in
  assert (List.length (T.ancestors accepted (id "127") |> Option.value_exn) = 127);
  assert (Result.is_error (deep (T.max_depth + 1)));
  let node = leaf ~label:(String.make 4096 'x') () in
  let nodes = List.init 2048 ~f:(fun i -> id (Int.to_string i), node) in
  assert (Result.is_error (T.create ~roots:(List.map nodes ~f:fst) nodes));
  let long_ids =
    List.init 20_000 ~f:(fun i -> id (String.make 250 'x' ^ sprintf "%06d" i))
  in
  let nodes = List.map long_ids ~f:(fun id -> id, leaf ()) in
  (* Declarations alone fit; counting the root/child reference copies does not. *)
  assert (Result.is_error (T.create ~roots:long_ids nodes));
  let root =
    T.Node.create ~label:"Folder" ~children:(Branch { ids = long_ids; next = End }) ()
    |> ok
  in
  assert (Result.is_error (T.create ~roots:[ id "root" ] ((id "root", root) :: nodes)));
  print_endline "UTF-8 labels/IDs, opaque cursor, depth 128 and 8 MiB metadata enforced";
  [%expect {| UTF-8 labels/IDs, opaque cursor, depth 128 and 8 MiB metadata enforced |}]
;;

let%expect_test
    "100000 loaded nodes traverse, reorder and preserve topology on payload update"
  =
  let nodes = List.init T.max_nodes ~f:(fun i -> id (Int.to_string i), leaf i) in
  let roots = List.map nodes ~f:fst in
  let tree = T.create ~roots nodes |> ok in
  assert (T.length tree = T.max_nodes);
  let sum =
    List.fold (T.preorder tree) ~init:0 ~f:(fun sum id ->
      sum + (T.find tree id |> Option.value_exn |> T.Node.data))
  in
  assert (sum = 4_999_950_000);
  let reordered = T.replace tree ~roots:(List.rev roots) nodes |> ok in
  let updated = T.set_data reordered ~id:(id "50000") (-1) |> ok in
  assert (phys_equal (T.preorder reordered) (T.preorder updated));
  assert (List.equal T.Id.equal (List.rev (T.preorder tree)) (T.preorder updated));
  assert (T.metadata_bytes tree = T.metadata_bytes updated);
  assert (
    Result.is_error
      (T.create ~roots:(id "extra" :: roots) ((id "extra", leaf 0) :: nodes)));
  printf
    "%d nodes; sum %d; reversed order and O(log n) payload update\n"
    (T.length updated)
    sum;
  [%expect {| 100000 nodes; sum 4999950000; reversed order and O(log n) payload update |}]
;;
