open Core
module Source = Gpuio.Text_source

let%expect_test "byte-stream decoder preserves every Unicode split" =
  let text = "λ👨‍👩‍👧‍👦🦀終" in
  for split = 0 to String.length text do
    let state, before =
      Source.Decoder.feed Source.Decoder.empty (String.prefix text split)
      |> Or_error.ok_exn
    in
    assert (Source.Decoder.pending_bytes state <= 3);
    let state, after =
      Source.Decoder.feed state (String.drop_prefix text split) |> Or_error.ok_exn
    in
    Source.Decoder.finish state |> Or_error.ok_exn;
    assert (String.equal (before ^ after) text)
  done;
  List.iter [ "\255"; "\192\128"; "\237\160"; "\244\144"; "\224\128" ] ~f:(fun bytes ->
    assert (Or_error.is_error (Source.Decoder.feed Source.Decoder.empty bytes)));
  let pending, _ =
    Source.Decoder.feed Source.Decoder.empty "\240\159" |> Or_error.ok_exn
  in
  print_s
    [%sexp
      (Source.Decoder.pending_bytes pending : int)
    , (Or_error.is_error (Source.Decoder.finish pending) : bool)];
  [%expect {| (2 true) |}]
;;

let ok = Or_error.ok_exn

let%expect_test "coalesced Unicode appends and terminal status" =
  let first = Source.empty_stream () in
  let second = Source.append first "hello 👨‍👩‍👧‍👦" |> ok in
  let third = Source.append second "\nworld" |> ok in
  let final = Source.finish third |> ok in
  print_s
    [%sexp (Source.to_string final : string), (Source.status final : Source.Status.t)];
  print_s [%sexp (Source.Expert.changed_from final ~previous:second : int)];
  print_s
    [%sexp
      (Source.Expert.changed_from final ~previous:third = Source.byte_length third : bool)];
  print_s [%sexp (Or_error.is_error (Source.append final "!") : bool)];
  [%expect
    {|
    ( "hello \240\159\145\168\226\128\141\240\159\145\169\226\128\141\240\159\145\167\226\128\141\240\159\145\166\
     \nworld" Complete)
    31
    true
    true
    |}]
;;

let%expect_test "byte ranges, corrections and reset generations" =
  let source = Source.of_string "aλb" |> ok in
  print_s
    [%sexp (Or_error.is_error (Source.edit source ~first:2 ~last:3 ~text:"x") : bool)];
  let edited = Source.edit source ~first:1 ~last:3 ~text:"β" |> ok in
  print_s [%sexp (Source.Expert.changed_from edited ~previous:source : int)];
  print_s [%sexp (Source.slice edited ~first:1 ~last:3 |> ok : string)];
  let reset = Source.reset edited "new" |> ok in
  print_s
    [%sexp
      (Source.Expert.same_source reset source : bool)
    , (Source.Expert.same_generation reset source : bool)];
  print_s [%sexp (Or_error.is_error (Source.cancel source) : bool)];
  [%expect
    {|
    true
    1
    "\206\178"
    (true false)
    true
    |}]
;;

let%expect_test "bounded chunks and exact append suffix across chunk boundaries" =
  let source = ref (Source.empty_stream ()) in
  let exact = String.concat (List.init 1000 ~f:(fun _ -> "🦀abcdefghijklmno")) in
  for _ = 1 to 1000 do
    let before = !source in
    source := Source.append before "🦀abcdefghijklmno" |> ok;
    assert (
      Source.Expert.changed_from !source ~previous:before = Source.byte_length before)
  done;
  let chunks = ref 0 in
  Source.Expert.iter_chunks !source ~f:(fun ~offset:_ text ->
    Int.incr chunks;
    assert (String.length text <= 16384);
    assert (Stdlib.String.is_valid_utf_8 text));
  print_s [%sexp (String.equal exact (Source.to_string !source) : bool), (!chunks : int)];
  print_s [%sexp (Or_error.is_error (Source.append !source "\240\159") : bool)];
  [%expect
    {|
    (true 2)
    true
    |}]
;;
