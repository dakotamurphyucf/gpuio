open Core
module D = Gpuio.Chart_data

let ok = Or_error.ok_exn

let data_exn ~area phase =
  if (not (Float.is_finite phase)) || Float.(phase < 0.)
  then invalid_arg "Stacked sample phase must be finite and nonnegative";
  let categories =
    List.map
      [ 42L, "Mon"; 7L, "Tue"; 99L, "Wed"; 3L, "Thu"; 15L, "Fri" ]
      ~f:(fun (id, label) ->
        D.Category.create ~id:(D.Category_id.of_int64 id |> ok) ~label |> ok)
  in
  let series id name values =
    D.Categorical_series.create
      ~id:(D.Series_id.of_int64 id |> ok)
      ~name
      (List.mapi values ~f:(fun i value ->
         D.Categorical_point.create
           ~id:(D.Datum_id.of_int64 (Int64.of_int (100 - i)) |> ok)
           ~category:(D.Category.id (List.nth_exn categories i))
           ~value:(Option.map value ~f:(fun value -> value +. phase))
           ()
         |> ok))
    |> ok
  in
  let layer series =
    if area then D.Categorical_layer.Area series else D.Categorical_layer.Bar series
  in
  D.categorical
    ~categories
    [ layer (series 1L "Completed" [ Some 30.; Some 40.; None; Some 20.; Some 35. ])
    ; layer (series 2L "Adjustment" [ Some 10.; Some (-5.); Some 15.; Some 10.; Some 5. ])
    ; D.Categorical_layer.Line
        (series 3L "Target" [ Some 45.; Some 45.; Some 45.; Some 45.; Some 45. ])
    ]
  |> ok
;;
