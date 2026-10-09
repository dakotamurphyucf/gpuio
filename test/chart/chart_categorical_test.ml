open Core
module D = Gpuio.Chart_data
module W = Gpuio_protocol.Chart_data_wire

let ok = Or_error.ok_exn

let category id label =
  D.Category.create ~id:(D.Category_id.of_int64 id |> ok) ~label |> ok
;;

let point id category value label =
  D.Categorical_point.create
    ~id:(D.Datum_id.of_int64 id |> ok)
    ~category:(D.Category.id category)
    ~value
    ~label
    ()
  |> ok
;;

let categories = [ category 42L "Same"; category 7L "冬"; category 99L "Same" ]

let series points =
  D.Categorical_series.create ~id:(D.Series_id.of_int64 9L |> ok) ~name:"Load" points
;;

let points =
  List.map3_exn
    categories
    [ 8L; 3L; 22L ]
    [ Some 2.; None; Some 6. ]
    ~f:(fun category id value ->
      point
        id
        category
        value
        (if Int64.equal id 8L then "a" else if Int64.equal id 22L then "z" else ""))
;;

let%expect_test
    "categorical wire preserves explicit order, duplicate labels and missing bars"
  =
  let data =
    D.categorical ~categories [ D.Categorical_layer.Bar (series points |> ok) ] |> ok
  in
  let bytes = D.Expert.encode data |> ok in
  assert (D.equal data (D.Expert.decode bytes |> ok));
  assert (D.value_count data = 3);
  print_endline
    (String.to_list bytes
     |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
     |> String.concat);
  [%expect
    {| 0305032a0453616d650703e586ac630453616d65010209044c6f616403082a0100000000000000400161030700001663010000000000001840017a0000 |}]
;;

let%expect_test "category admission rejects membership/order/identity and numeric errors" =
  let layer points = D.Categorical_layer.Line (series points |> ok) in
  assert (
    Result.is_error (D.categorical ~categories:(List.rev categories) [ layer points ]));
  assert (Result.is_error (D.categorical ~categories [ layer (List.take points 2) ]));
  assert (Result.is_error (D.categorical ~categories:[] [ layer points ]));
  assert (
    Result.is_error
      (D.categorical ~categories:[ List.hd_exn categories; List.hd_exn categories ] []));
  assert (Result.is_error (series [ List.hd_exn points; List.hd_exn points ]));
  List.iter [ Float.nan; Float.infinity; 1e101 ] ~f:(fun value ->
    assert (
      Result.is_error
        (D.Categorical_point.create
           ~id:(D.Datum_id.of_int64 1L |> ok)
           ~category:(D.Category_id.of_int64 42L |> ok)
           ~value:(Some value)
           ())));
  let empty = D.categorical ~categories [] |> ok in
  assert (D.value_count empty = 0);
  assert (
    D.Expert.retained_bytes empty
    > D.Expert.retained_bytes (D.categorical ~categories:[] [] |> ok));
  let wire = D.Expert.to_wire (D.categorical ~categories [ layer points ] |> ok) in
  let encoded =
    D.Expert.encode (D.categorical ~categories [ layer points ] |> ok) |> ok
  in
  for length = 0 to String.length encoded - 1 do
    assert (Result.is_error (D.Expert.decode (String.prefix encoded length)))
  done;
  assert (Result.is_error (D.Expert.decode (encoded ^ "x")));
  let malformed =
    match wire.contents with
    | W.Contents.Categorical (domain, layers) ->
      { wire with contents = Categorical (List.rev domain, layers) }
    | _ -> assert false
  in
  assert (Result.is_error (D.Expert.of_wire malformed));
  print_endline
    "explicit domain, identity, finite values, bounds and exact decoding enforced";
  [%expect
    {| explicit domain, identity, finite values, bounds and exact decoding enforced |}]
;;
