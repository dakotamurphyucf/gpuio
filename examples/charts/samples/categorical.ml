open Core
module D = Gpuio.Chart_data

let ok = Or_error.ok_exn

let categories =
  List.map
    [ 42L, "Research"; 7L, "Code"; 99L, "Research"; 3L, "Review" ]
    ~f:(fun (id, label) ->
      D.Category.create ~id:(D.Category_id.of_int64 id |> ok) ~label |> ok)
;;

let data_exn phase =
  if (not (Float.is_finite phase)) || Float.(phase < 0.)
  then invalid_arg "Categorical sample phase must be finite and nonnegative";
  let series id name values =
    D.Categorical_series.create
      ~id:(D.Series_id.of_int64 id |> ok)
      ~name
      (List.map2_exn categories values ~f:(fun category (id, value) ->
         D.Categorical_point.create
           ~id:(D.Datum_id.of_int64 id |> ok)
           ~category:(D.Category.id category)
           ~value:(Option.map value ~f:(fun v -> v +. phase))
           ()
         |> ok))
    |> ok
  in
  D.categorical
    ~categories
    [ D.Categorical_layer.Bar
        (series 1L "Completed" [ 8L, Some 30.; 3L, None; 22L, Some 45.; 6L, Some 20. ])
    ; Line (series 2L "Target" [ 1L, Some 40.; 2L, Some 50.; 3L, Some 60.; 4L, Some 30. ])
    ]
  |> ok
;;

let describe_selection data (selection : Gpuio.Chart_selection.t) =
  let open Option.Let_syntax in
  match D.Expert.contents data, selection with
  | Categorical (categories, layers), Cartesian { series; span; aggregation } ->
    let%bind series =
      List.find_map layers ~f:(fun (D.Categorical_layer.Line s | Area s | Bar s) ->
        Option.some_if (D.Series_id.equal (D.Categorical_series.id s) series) s)
    in
    let points = D.Categorical_series.points series in
    let%bind first = List.nth points span.start_index in
    let%bind last = List.nth points (span.start_index + span.length - 1) in
    if
      not
        (D.Datum_id.equal (D.Categorical_point.id first) span.first
         && D.Datum_id.equal (D.Categorical_point.id last) span.last)
    then None
    else (
      let%bind category =
        List.find categories ~f:(fun c ->
          D.Category_id.equal (D.Category.id c) (D.Categorical_point.category first))
      in
      match aggregation with
      | Exact ->
        let%map value = D.Categorical_point.value first in
        sprintf
          "%s · %s (category %Ld) · value %.3g"
          (D.Categorical_series.name series)
          (D.Category.label category)
          (D.Category_id.to_int64 (D.Category.id category))
          value
      | Sum | Mean ->
        Some
          (sprintf
             "%s · %d categories selected"
             (D.Categorical_series.name series)
             span.length))
  | _ -> None
;;
