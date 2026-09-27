open Core
open Gpuio
module N = Navigation_stack

let ok = Or_error.ok_exn
let id name = N.Id.of_string name |> ok
let entry ?(draft = "") name = N.Entry.create ~id:(id name) ~label:name draft |> ok
let names entries = List.map entries ~f:(fun entry -> N.Id.to_string (N.Entry.id entry))

let show t =
  print_s
    [%sexp
      (names (N.back_entries t) : string list)
    , (Option.map (N.current t) ~f:(fun entry -> N.Id.to_string (N.Entry.id entry))
       : string option)
    , (names (N.forward_entries t) : string list)]
;;

let%expect_test "back, replacement and branching keep entry instances explicit" =
  let root = N.singleton (entry "home") in
  let chat = N.push root (entry ~draft:"pending prompt" "chat-1") |> ok in
  let settings = N.push chat (entry "settings") |> ok in
  show settings;
  let back = N.pop settings in
  show back;
  let edited = N.update back (id "chat-1") ~f:(fun draft -> draft ^ "!") |> ok in
  let replaced = N.replace edited (entry ~draft:"new conversation" "chat-2") |> ok in
  show replaced;
  let restored = N.forward replaced in
  show restored;
  let branched = N.push (N.pop restored) (entry "help") |> ok in
  show branched;
  print_s
    [%sexp
      (N.find edited (id "chat-1") |> Option.map ~f:N.Entry.data : string option)
    , (N.find branched (id "chat-1") |> Option.is_none : bool)
    , (N.find branched (id "settings") |> Option.is_none : bool)];
  show (N.pop_to_root branched);
  show (N.pop (N.pop_to_root branched));
  show (N.clear branched);
  [%expect
    {|
    ((home chat-1) (settings) ())
    ((home) (chat-1) (settings))
    ((home) (chat-2) (settings))
    ((home chat-2) (settings) ())
    ((home chat-2) (help) ())
    (("pending prompt!") true true)
    (() (home) (chat-2 help))
    (() (home) (chat-2 help))
    (() () ())
    |}]
;;

let%expect_test "restore history, stale breadcrumbs and same-ID updates" =
  let t = N.create ~current:(id "b") [ entry "a"; entry "b"; entry "c" ] |> ok in
  let t = N.replace t (entry ~draft:"retained" "b") |> ok in
  show t;
  assert (Result.is_error (N.pop_to t (id "c")));
  assert (Result.is_error (N.pop_to t (id "missing")));
  assert (Result.is_error (N.replace t (entry "a")));
  assert (Result.is_error (N.replace t (entry "c")));
  assert (Result.is_error (N.push t (entry "c")));
  assert (Result.is_error (N.update t (id "missing") ~f:Fn.id));
  let home = N.pop_to t (id "a") |> ok in
  show home;
  show (N.forward (N.forward home));
  assert (Result.is_error (N.create [ entry "a"; entry "a" ]));
  assert (Result.is_error (N.create ~current:(id "a") []));
  show (N.replace N.empty (entry "first") |> ok);
  show (N.forward N.empty);
  [%expect
    {|
    ((a) (b) (c))
    (() (a) (b c))
    ((a b) (c) ())
    (() (first) ())
    (() () ())
    |}]
;;

let%expect_test "bounded history and labels; traversal preserves payload identity" =
  List.iter
    [ ""; "\255"; "a\000b"; String.make 257 'x' ]
    ~f:(fun name -> assert (Result.is_error (N.Id.of_string name)));
  List.iter
    [ ""; "\255"; "a\000b"; String.make 4097 'x' ]
    ~f:(fun label -> assert (Result.is_error (N.Entry.create ~id:(id "valid") ~label ())));
  let payload = ref 7 in
  let entries =
    List.init N.max_entries ~f:(fun i ->
      N.Entry.create ~id:(id (Int.to_string i)) ~label:"Visit" payload |> ok)
  in
  let t = N.create entries |> ok in
  assert (
    Result.is_error
      (N.push t (N.Entry.create ~id:(id "overflow") ~label:"Visit" payload |> ok)));
  let first = N.pop_to_root t in
  let rec traverse current visits =
    let item = N.current current |> Option.value_exn in
    assert (phys_equal (N.Entry.data item) payload);
    if N.can_forward current then traverse (N.forward current) (visits + 1) else visits
  in
  assert (traverse first 1 = N.max_entries);
  let branch = N.Entry.create ~id:(id "new") ~label:"Visit" payload |> ok in
  let shorter = N.push first branch |> ok in
  assert (List.length (N.entries shorter) = 2);
  assert (List.length (N.entries t) = N.max_entries);
  print_endline
    "128-entry full traversal and branch pruning preserve caller-owned payloads";
  [%expect
    {| 128-entry full traversal and branch pruning preserve caller-owned payloads |}]
;;
