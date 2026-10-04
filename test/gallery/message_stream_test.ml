open Core
open Gpuio
module State = Gpuio_gallery_model.Message_stream

let apply t actions = List.fold actions ~init:t ~f:State.apply

let changed_values before after =
  List_collection.fold_changed_values
    (State.rows after)
    ~previous:(State.rows before)
    ~init:[]
    ~f:(fun keys key -> key :: keys)
  |> List.sort ~compare:Int.compare
;;

let%expect_test "history splices preserve response identities and streamed content" =
  let streaming = apply State.initial [ Stream_latest; Stream_latest ] in
  assert (
    phys_equal
      (List_collection.keys (State.rows State.initial))
      (List_collection.keys (State.rows streaming)));
  print_s [%sexp (changed_values State.initial streaming : int list)];
  let history = apply streaming [ Prepend; Append; Prepend ] in
  print_s [%sexp (changed_values streaming history : int list)];
  assert (
    Option.equal
      String.equal
      (List_collection.find (State.rows streaming) 999)
      (List_collection.find (State.rows history) 999));
  assert (State.streamed_lines history = 0);
  let latest = State.apply history Stream_latest in
  print_s [%sexp (changed_values history latest : int list)];
  let reset = State.apply latest Reset_latest in
  assert (
    List.equal
      [%equal: int * string]
      (List_collection.to_alist (State.rows history))
      (List_collection.to_alist (State.rows reset)));
  print_s
    [%sexp
      (State.first reset : int)
    , (State.last reset : int)
    , (List_collection.length (State.rows reset) : int)];
  [%expect
    {|
    (999)
    (-2 -1 1000)
    (1000)
    (-2 1000 1003)
    |}]
;;

let%expect_test "repeated queued gallery actions stay bounded" =
  let t =
    List.fold (List.init 100 ~f:Fn.id) ~init:State.initial ~f:(fun t _ ->
      apply t [ Prepend; Append; Stream_latest ])
  in
  print_s
    [%sexp
      (State.first t : int)
    , (State.last t : int)
    , (List_collection.length (State.rows t) : int)
    , (State.streamed_lines t : int)
    , (State.can_prepend t : bool)
    , (State.can_append t : bool)];
  assert (phys_equal t (apply t [ Prepend; Append; Stream_latest ]));
  let reset = State.apply t Reset_latest in
  print_s [%sexp (changed_values t reset : int list)];
  [%expect
    {|
    (-32 1031 1064 8 false false)
    (1031)
    |}]
;;
