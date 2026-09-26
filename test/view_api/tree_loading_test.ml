open Core
open Gpuio
module T = Tree
module P = Tree_loading

let ok = Or_error.ok_exn
let id text = T.Id.of_string text |> ok
let leaf name = T.Node.create ~label:name ~children:Leaf name |> ok

let branch ?(children = []) ?(next = List_paging.Boundary.More None) name =
  T.Node.create ~label:name ~children:(Branch { ids = children; next }) name |> ok
;;

let forest count =
  let nodes =
    List.init count ~f:(fun i ->
      let name = sprintf "p%03d" i in
      id name, branch name)
  in
  T.create ~roots:(List.map nodes ~f:fst) nodes |> ok
;;

let snapshot = P.snapshot
let tree t = P.Snapshot.tree (snapshot t)
let status t name = P.Snapshot.status (snapshot t) (id name)
let take t = P.take t |> Option.value_exn
let enqueue t name = assert (P.request t (id name) |> ok)

let page ?(next = List_paging.Boundary.End) name =
  { P.Page.roots = [ id name ]; nodes = [ id name, leaf name ]; next }
;;

let empty next = { P.Page.roots = []; nodes = []; next }

let show_counts t =
  let s = snapshot t in
  print_s
    [%sexp
      (P.Snapshot.queued_count s : int)
    , (P.Snapshot.running_count s : int)
    , (P.Snapshot.failed_count s : int)
    , (P.Snapshot.error_detail_count s : int)]
;;

let%expect_test
    "FIFO admission bounds queued and running work and cancellation frees capacity"
  =
  let source = forest 70 in
  let t = P.create source in
  for i = 0 to P.max_queued - 1 do
    enqueue t (sprintf "p%03d" i)
  done;
  assert (not (P.request t (id "p000") |> ok));
  assert (Result.is_error (P.request t (id "p064")));
  assert (Result.is_error (P.request t (id "absent")));
  let requests =
    List.fold (List.range 0 P.max_running) ~init:[] ~f:(fun reversed _ ->
      take t :: reversed)
    |> List.rev
  in
  assert (Option.is_none (P.take t));
  print_s [%sexp (List.map requests ~f:P.Request.parent : T.Id.t list)];
  show_counts t;
  enqueue t "p064";
  let obsolete = List.nth_exn requests 2 in
  P.cancel t (P.Request.parent obsolete);
  P.cancel t (P.Request.parent obsolete);
  assert (not (P.is_current t obsolete));
  print_s [%sexp (P.Request.parent (take t) : T.Id.t)];
  let invalid = { P.Page.roots = [ id "missing" ]; nodes = []; next = More None } in
  print_s [%sexp (P.complete t obsolete invalid |> ok : P.Completion.t)];
  let other = P.create source in
  enqueue other "p000";
  let foreign = take other in
  assert (not (P.is_current t foreign));
  assert (P.Completion.equal (P.fail t foreign (Error.of_string "foreign")) Obsolete);
  show_counts t;
  for _ = 1 to 10_000 do
    P.cancel t (id "p069");
    enqueue t "p069";
    P.cancel t (id "p069")
  done;
  assert (P.Snapshot.queued_count (snapshot t) = 60);
  P.close t;
  P.close t;
  show_counts t;
  assert (Option.is_none (P.take t));
  assert (Result.is_error (P.request t (id "p000")));
  assert (phys_equal (tree t) source);
  [%expect
    {|
    (p000 p001 p002 p003)
    (60 4 0 0)
    p004
    Obsolete
    (60 4 0 0)
    (0 0 0 0)
    |}]
;;

let%expect_test
    "pages append atomically while unrelated requests and historical snapshots stay valid"
  =
  let t = P.create (forest 2) in
  enqueue t "p000";
  enqueue t "p001";
  let a = take t in
  let b = take t in
  let before = snapshot t in
  assert (
    P.Completion.equal
      (P.complete t a (page ~next:(More (Some "second")) "a") |> ok)
      Applied);
  assert (P.is_current t b);
  assert (P.Completion.equal (P.complete t a (page "duplicate") |> ok) Obsolete);
  assert (Option.is_none (T.find (P.Snapshot.tree before) (id "a")));
  enqueue t "p000";
  let a2 = take t in
  assert (not (P.Request.same a a2));
  print_s [%sexp (P.Request.cursor a2 : string option)];
  ignore (P.complete t a2 (empty End) |> ok : P.Completion.t);
  let nested =
    { P.Page.roots = [ id "b" ]
    ; nodes =
        [ id "b", branch ~children:[ id "b-child" ] ~next:End "b"
        ; id "b-child", leaf "b-child"
        ]
    ; next = End
    }
  in
  ignore (P.complete t b nested |> ok : P.Completion.t);
  print_s [%sexp (T.preorder (tree t) : T.Id.t list)];
  print_s [%sexp (T.ancestors (tree t) (id "b-child") : T.Id.t list option)];
  print_s
    [%sexp (status t "p000" : P.Status.t option), (status t "p001" : P.Status.t option)];
  assert (not (P.request t (id "p000") |> ok));
  assert (Result.is_error (P.request t (id "a")));
  show_counts t;
  [%expect
    {|
    (second)
    (p000 a p001 b b-child)
    ((p001 b))
    ((End) (End))
    (0 0 0 0)
    |}]
;;

let%expect_test
    "invalid current pages fail without changing data and require explicit retry"
  =
  let invalid_pages =
    [ "no cursor progress", empty (More None)
    ; "missing node", { P.Page.roots = [ id "missing" ]; nodes = []; next = End }
    ; "existing ID", page "p000"
    ; ( "duplicate ID"
      , { P.Page.roots = [ id "a"; id "a" ]
        ; nodes = [ id "a", leaf "a"; id "a", leaf "a" ]
        ; next = End
        } )
    ; ( "cycle"
      , { P.Page.roots = []
        ; nodes =
            [ id "a", branch ~children:[ id "b" ] "a"
            ; id "b", branch ~children:[ id "a" ] "b"
            ]
        ; next = End
        } )
    ; "oversized cursor", empty (More (Some (String.make 4097 'x')))
    ]
  in
  List.iter invalid_pages ~f:(fun (name, page) ->
    let t = P.create (forest 1) in
    let before = tree t in
    enqueue t "p000";
    let request = take t in
    assert (Result.is_error (P.complete t request page));
    assert (phys_equal (tree t) before);
    assert (not (P.request t (id "p000") |> ok));
    assert (
      match status t "p000" with
      | Some (Failed _) -> true
      | None | Some (Ready | Queued | Loading | End) -> false);
    assert (P.retry t (id "p000") |> ok);
    assert (P.Completion.equal (P.complete t request page |> ok) Obsolete);
    ignore (P.complete t (take t) (empty (More (Some "advanced"))) |> ok : P.Completion.t);
    assert (
      match status t "p000" with
      | Some Ready -> true
      | None | Some (Queued | Loading | End | Failed _) -> false);
    print_endline name);
  let t = P.create (forest 1) in
  enqueue t "p000";
  let request = take t in
  let nodes =
    List.init (P.max_page_nodes + 1) ~f:(fun i ->
      let name = Int.to_string i in
      id name, leaf name)
  in
  assert (
    Result.is_error
      (P.complete t request { roots = List.map nodes ~f:fst; nodes; next = End }));
  let nodes =
    List.init T.max_depth ~f:(fun i ->
      let name = Int.to_string i in
      let children = if i + 1 < T.max_depth then [ id (Int.to_string (i + 1)) ] else [] in
      id name, branch ~children name)
  in
  let t = P.create (T.create ~roots:[ id "0" ] nodes |> ok) in
  enqueue t "127";
  let before = tree t in
  assert (Result.is_error (P.complete t (take t) (page "too-deep")));
  assert (phys_equal (tree t) before);
  print_endline "page count and combined depth";
  [%expect
    {|
    no cursor progress
    missing node
    existing ID
    duplicate ID
    cycle
    oversized cursor
    page count and combined depth
    |}]
;;

let%expect_test
    "collapse, source revisions, incarnation, reset and explicit invalidation reject \
     late work"
  =
  let source =
    T.create
      ~roots:[ id "root"; id "other" ]
      [ id "root", branch ~children:[ id "child" ] "root"
      ; id "child", branch "child"
      ; id "other", branch "other"
      ]
    |> ok
  in
  let t = P.create source in
  enqueue t "child";
  enqueue t "other";
  let child = take t in
  let other = take t in
  let updated = T.set_data source ~id:(id "child") "changed payload" |> ok in
  P.update t updated |> ok;
  assert (P.is_current t child && P.is_current t other);
  let reordered =
    T.replace updated ~roots:[ id "other"; id "root" ] (T.to_alist updated) |> ok
  in
  P.update t reordered |> ok;
  assert (P.is_current t child && P.is_current t other);
  let ui =
    Tree_state.create reordered ~expanded:[ id "root"; id "child"; id "other" ] () |> ok
  in
  P.cancel_hidden t ui;
  assert (P.is_current t child && P.is_current t other);
  let ui = Tree_state.toggle_expanded ui reordered (id "root") in
  P.cancel_hidden t ui;
  assert ((not (P.is_current t child)) && P.is_current t other);
  enqueue t "root";
  enqueue t "child";
  P.cancel_subtree t (id "root");
  assert (P.Snapshot.queued_count (snapshot t) = 0);
  P.invalidate t (id "other");
  assert (not (P.is_current t other));
  enqueue t "child";
  let old = take t in
  let removed = T.replace reordered ~roots:[] [] |> ok in
  let restored =
    T.replace removed ~roots:(T.roots reordered) (T.to_alist reordered) |> ok
  in
  P.update t restored |> ok;
  assert (not (P.is_current t old));
  enqueue t "child";
  let old = take t in
  let changed =
    T.replace
      restored
      ~roots:(T.roots restored)
      (T.to_alist restored
       |> List.map ~f:(fun (key, node) ->
         ( key
         , if T.Id.equal key (id "child")
           then branch ~next:(More (Some "new source")) "child"
           else node )))
    |> ok
  in
  P.update t changed |> ok;
  assert (not (P.is_current t old));
  assert (Result.is_error (P.update t source));
  enqueue t "child";
  let old = take t in
  P.reset t source |> ok;
  print_s
    [%sexp
      (P.Request.generation old : int64), (P.Snapshot.generation (snapshot t) : int64)];
  assert (P.Completion.equal (P.complete t old (page "late") |> ok) Obsolete);
  assert (T.length (tree t) = 3);
  print_endline
    "unrelated edits preserve work; collapse/delete/reuse/child revision/reset cancel it";
  [%expect
    {|
    (0 1)
    unrelated edits preserve work; collapse/delete/reuse/child revision/reset cancel it
    |}]
;;

let%expect_test
    "bounded error details do not turn evicted failures into automatic retries"
  =
  let t = P.create (forest 70) in
  for i = 0 to 69 do
    let name = sprintf "p%03d" i in
    enqueue t name;
    let error = Error.of_string (String.make 4095 'x' ^ "💚") in
    ignore (P.fail t (take t) error : P.Completion.t)
  done;
  show_counts t;
  P.cancel t (id "p000");
  assert (not (P.request t (id "p000") |> ok));
  assert (
    match status t "p000" with
    | Some (Failed _) -> true
    | None | Some (Ready | Queued | Loading | End) -> false);
  let error =
    match status t "p069" with
    | Some (Failed error) -> error
    | None | Some (Ready | Queued | Loading | End) -> assert false
  in
  assert (String.length (Error.to_string_hum error) = 4095);
  assert (Stdlib.String.is_valid_utf_8 (Error.to_string_hum error));
  let before = snapshot t in
  assert (P.retry t (id "p000") |> ok);
  ignore (P.complete t (take t) (empty End) |> ok : P.Completion.t);
  assert (P.Snapshot.failed_count before = 70);
  show_counts t;
  P.invalidate t (id "p001");
  enqueue t "p001";
  ignore (P.fail t (take t) (Error.of_string "\255") : P.Completion.t);
  print_s [%sexp (status t "p001" : P.Status.t option)];
  let removed = T.replace (tree t) ~roots:[] [] |> ok in
  P.update t removed |> ok;
  show_counts t;
  [%expect
    {|
    (0 0 70 64)
    (0 0 69 64)
    ((Failed "Tree load failed (invalid UTF-8 error message)"))
    (0 0 0 0)
    |}]
;;
