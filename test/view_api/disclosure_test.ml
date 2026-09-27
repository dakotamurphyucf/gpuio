open Core
open Gpuio
module D = Disclosure

let ok = Or_error.ok_exn
let id name = Choice.Id.of_string name |> ok

let item ?(disabled = false) name =
  Choice.create ~id:(id name) ~label:name ~disabled () |> ok
;;

let collection items = Choice.Collection.create items |> ok
let expanded t = List.map (D.expanded t) ~f:Choice.Id.to_string
let show t = print_s [%sexp (expanded t : string list)]

let%expect_test "ordered toggles and single-mode replacement use current expansion" =
  let items = collection [ item "a"; item "b"; item "c" ] in
  let t = D.create ~items ~mode:Multiple ~expanded:[] () |> ok in
  let t =
    List.fold
      [ D.Request.Toggle (id "a"); Toggle (id "b"); Toggle (id "a"); Expand (id "c") ]
      ~init:t
      ~f:(fun t request ->
        let t = D.apply_request t request in
        show t;
        t)
  in
  let single = D.with_mode t (Single { allow_empty = false }) in
  show single;
  show (D.apply_request single (Collapse (id "b")));
  let next = D.apply_request single (Toggle (id "a")) in
  show next;
  show
    (D.apply_request (D.with_mode next (Single { allow_empty = true })) (Toggle (id "a")));
  [%expect
    {|
    (a)
    (a b)
    (b)
    (b c)
    (b)
    (b)
    (a)
    ()
    |}]
;;

let%expect_test "replacement normalizes history but disabled requests never act" =
  let items = collection [ item ~disabled:true "a"; item "b"; item "c" ] in
  let t =
    D.create ~items ~mode:(Single { allow_empty = false }) ~expanded:[ id "a" ] () |> ok
  in
  show (D.apply_request t (Toggle (id "a")));
  let t = D.apply_request t (Expand (id "b")) in
  show t;
  List.iter
    [ D.Request.Toggle (id "b"); Expand (id "c"); Collapse (id "b") ]
    ~f:(fun request ->
      let disabled = D.with_disabled t true in
      assert (D.equal disabled (D.apply_request disabled request)));
  assert (D.equal t (D.apply_request t (Toggle (id "missing"))));
  show (D.with_items t (collection [ item ~disabled:true "a"; item "c" ]));
  show (D.with_items t (collection [ item ~disabled:true "a" ]));
  let empty = D.with_items t (collection []) in
  show empty;
  show (D.with_items empty items);
  let multiple = D.create ~items ~mode:Multiple ~expanded:[ id "c"; id "a" ] () |> ok in
  show multiple;
  show (D.with_items multiple (collection [ item "c"; item "a" ]));
  [%expect
    {|
    (a)
    (b)
    (c)
    (a)
    ()
    (b)
    (a c)
    (c a)
    |}]
;;

let%expect_test "invalid expansion is rejected; nested models are independent" =
  let items = collection [ item "parent"; item "sibling" ] in
  List.iter
    [ D.create ~items ~mode:Multiple ~expanded:[ id "parent"; id "parent" ] ()
    ; D.create ~items ~mode:Multiple ~expanded:[ id "missing" ] ()
    ; D.create
        ~items
        ~mode:(Single { allow_empty = true })
        ~expanded:[ id "parent"; id "sibling" ]
        ()
    ; D.create ~items ~mode:(Single { allow_empty = false }) ~expanded:[] ()
    ]
    ~f:(fun result -> assert (Result.is_error result));
  let outer = D.create ~items ~mode:Multiple ~expanded:[ id "parent" ] () |> ok in
  let inner =
    D.create
      ~items:(collection [ item "detail" ])
      ~mode:Multiple
      ~expanded:[ id "detail" ]
      ()
    |> ok
  in
  let closed = D.apply_request outer (Collapse (id "parent")) in
  show closed;
  show inner;
  show (D.apply_request closed (Expand (id "parent")));
  print_endline "pure expansion changes do not own nested models or task lifetimes";
  [%expect
    {|
    ()
    (detail)
    (parent)
    pure expansion changes do not own nested models or task lifetimes
    |}]
;;

let%expect_test "maximum collection traverses and contracts without leftover expansion" =
  let items =
    List.init Choice.Collection.max_choices ~f:(fun index -> item (Int.to_string index))
  in
  let all =
    D.create
      ~items:(collection items)
      ~mode:Multiple
      ~expanded:(List.map items ~f:Choice.id)
      ()
    |> ok
  in
  assert (List.length (D.expanded all) = Choice.Collection.max_choices);
  let small = D.with_items all (collection (List.take items 2)) in
  show small;
  show (D.with_mode small (Single { allow_empty = false }));
  show (D.with_items small (collection []));
  assert (List.length (D.expanded all) = Choice.Collection.max_choices);
  [%expect
    {|
    (0 1)
    (0)
    ()
    |}]
;;
