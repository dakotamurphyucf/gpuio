open Core
module F = Fake_backend

let%expect_test "byte chunks preserve Unicode and failure has exactly one terminal" =
  List.iter [ 1; 3; 17; 4096 ] ~f:(fun chunk_bytes ->
    let config = F.Config.create ~chunk_bytes () |> Or_error.ok_exn in
    let steps = F.plan config ~prompt:"λ" in
    let text =
      List.filter_map steps ~f:(function
        | F.Step.Chunk text -> Some text
        | Fail _ | Finish -> None)
      |> String.concat
    in
    assert (String.equal text (F.response ~prompt:"λ"));
    assert (F.Step.equal (List.last_exn steps) Finish));
  let config =
    F.Config.create ~chunk_bytes:1 ~fail_after_chunks:3 () |> Or_error.ok_exn
  in
  print_s [%sexp (F.plan config ~prompt:"test" : F.Step.t list)];
  List.iter
    [ F.Config.create ~chunk_bytes:0 ()
    ; F.Config.create ~delay_seconds:Float.nan ()
    ; F.Config.create ~fail_after_chunks:(-1) ()
    ]
    ~f:(fun result -> assert (Result.is_error result));
  [%expect
    {|
    ((Chunk #) (Chunk #) (Chunk " ")
     (Fail "Simulated connection interrupted. Retry to finish this response."))
    |}]
;;
