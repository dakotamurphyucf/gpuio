open Core
module G = Gpuio_agent_chat_runtime.Generation_settings
module Backend = Gpuio_agent_chat_runtime.Conversation.Backend
module Range = Gpuio_agent_chat_runtime.Score_range
module Data = Gpuio_agent_chat_runtime.Result_data

let ok = Or_error.ok_exn

let%expect_test "generation preferences affect real fixture chunks and pace" =
  let prompt = "Native λ" in
  let inspect size interval =
    let settings = G.with_chunk_bytes G.default size |> ok in
    let settings = G.with_interval_ms settings interval |> ok in
    let backend = G.backend settings in
    let plan = Backend.plan backend ~prompt in
    let chunks =
      List.filter_map plan ~f:(function
        | Backend.Step.Chunk text -> Some text
        | Fail _ | Finish -> None)
    in
    print_s
      [%sexp
        (String.length (List.hd_exn chunks) : int)
      , (List.for_all chunks ~f:(fun text -> String.length text <= size) : bool)
      , (String.equal (String.concat chunks) (Backend.response ~prompt) : bool)
      , (Backend.Config.delay_seconds backend : float)
      , (List.last_exn plan : Backend.Step.t)]
  in
  inspect 4 200;
  inspect 128 10;
  print_s
    [%sexp
      (Result.is_error (G.with_chunk_bytes G.default 0) : bool)
    , (Result.is_error (G.with_chunk_bytes G.default 129) : bool)
    , (Result.is_error (G.with_interval_ms G.default 11) : bool)
    , (Result.is_error (G.with_interval_ms G.default 210) : bool)];
  [%expect
    {|
    (4 true true 0.2 Finish)
    (128 true true 0.01 Finish)
    (true true true true)
    |}]
;;

let%expect_test "score intervals preserve exact endpoints and full-range cardinality" =
  let point = Range.create ~lower:80 ~upper:80 |> ok in
  let query = Data.Query.create ~filter:(Between point) () |> ok in
  print_s
    [%sexp
      (Data.rows query |> List.map ~f:(fun (_, row) -> Data.Row.number row) : int list)];
  let all = Data.Query.create ~size:Large ~filter:(Between Range.all) () |> ok in
  print_s [%sexp (List.length (Data.rows all) : int)];
  print_s
    [%sexp
      (Result.is_error (Range.create ~lower:(-1) ~upper:100) : bool)
    , (Result.is_error (Range.create ~lower:0 ~upper:101) : bool)
    , (Result.is_error (Range.create ~lower:81 ~upper:80) : bool)
    , (Range.contains point 79 : bool)
    , (Range.contains point 80 : bool)
    , (Range.contains point 81 : bool)];
  [%expect
    {|
    (24)
    100000
    (true true true false true false)
    |}]
;;
