open Core
module Choice = Gpuio.Choice

let id value = Choice.Id.of_string value |> Or_error.ok_exn

let choice ?disabled key label =
  Choice.create ~id:(id key) ~label ?disabled () |> Or_error.ok_exn
;;

let%expect_test "activation applies to the current checkbox value" =
  List.iter [ Gpuio.Check_state.Unchecked; Checked; Indeterminate ] ~f:(fun initial ->
    let once = Gpuio.Check_state.activate initial in
    let twice = Gpuio.Check_state.activate once in
    print_s
      [%sexp
        ((initial, once, twice)
         : Gpuio.Check_state.t * Gpuio.Check_state.t * Gpuio.Check_state.t)]);
  [%expect
    {|
    (Unchecked Checked Unchecked)
    (Checked Unchecked Checked)
    (Indeterminate Checked Unchecked)
    |}]
;;

let%expect_test "choice IDs survive ordering and labels need not be unique" =
  let first = choice "a" "Same label" in
  let second = choice ~disabled:true "b" "Same label" in
  let collection = Choice.Collection.create [ second; first ] |> Or_error.ok_exn in
  print_s
    [%sexp
      (List.map (Choice.Collection.to_list collection) ~f:(fun item ->
         Choice.Id.to_string (Choice.id item))
       : string list)];
  print_s [%sexp (Choice.Collection.find collection (id "a") : Choice.t option)];
  print_s
    [%sexp
      (Choice.Collection.validate_selection collection (Some (id "b")) : unit Or_error.t)];
  print_s
    [%sexp
      (Choice.Collection.validate_selection collection (Some (id "gone"))
       : unit Or_error.t)];
  print_s
    [%sexp (Choice.Collection.create [ first; first ] : Choice.Collection.t Or_error.t)];
  [%expect
    {|
    (b a)
    (((id a) (label "Same label") (disabled false)))
    (Ok ())
    (Error "selected choice does not exist: gone")
    (Error "duplicate choice id: a")
    |}]
;;

let%expect_test "native choice strings reject malformed input" =
  List.iter
    [ ""; "bad\000id"; "\255"; String.make 257 'x' ]
    ~f:(fun value -> print_s [%sexp (Choice.Id.of_string value : Choice.Id.t Or_error.t)]);
  print_s [%sexp (Choice.create ~id:(id "ok") ~label:"" () : Choice.t Or_error.t)];
  [%expect
    {|
    (Error "choice id must contain 1..256 bytes")
    (Error "choice id must be UTF-8 without NUL")
    (Error "choice id must be UTF-8 without NUL")
    (Error "choice id must contain 1..256 bytes")
    (Error "choice label must contain 1..4096 bytes")
    |}]
;;

let%expect_test "choice collections enforce both item and aggregate text budgets" =
  let check count label =
    let items = List.init count ~f:(fun index -> choice (Int.to_string index) label) in
    print_s
      [%sexp
        (Choice.Collection.create items |> Result.map ~f:(fun _ -> ()) : unit Or_error.t)]
  in
  check 4096 "x";
  check 4097 "x";
  check 65 (String.make 4000 'x');
  check 66 (String.make 4000 'x');
  [%expect
    {|
    (Ok ())
    (Error "choice collection exceeds 4096 items")
    (Ok ())
    (Error "choice collection exceeds 262144 text bytes")
    |}]
;;
