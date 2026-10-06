open Core
module I = Gpuio.Chart_inspection

let ok = Or_error.ok_exn

let%expect_test "inspection validates lengths and resolves all optional theme colors" =
  let token = Gpuio.Color.token_exn "inspection.accent" in
  let card =
    I.Card.create
      ~placement:Anchor
      ~border_width:2.
      ~text_color:token
      ~background:token
      ~border_color:token
      ()
    |> ok
  in
  let crosshair =
    I.Crosshair.create ~axis:Both ~pattern:Solid ~thickness:12. ~color:token () |> ok
  in
  let marker =
    I.Marker.create ~size:24. ~stroke_width:3. ~fill:token ~stroke:token () |> ok
  in
  let inspection = I.create ~card ~crosshair ~marker () in
  assert (Result.is_error (Gpuio.Chart_style.create ~inspection ()));
  let theme =
    Gpuio.Theme.create [ "inspection.accent", Gpuio.Color.rgb_exn 0x2dd4bf ] |> ok
  in
  let wire =
    Gpuio.Chart_style.create ~inspection ~theme ()
    |> ok
    |> Gpuio.Chart_style.Expert.to_wire
  in
  assert (Gpuio_protocol.Chart_inspection_wire.valid wire.inspection);
  let colors =
    [ wire.inspection.card.text_color
    ; wire.inspection.card.background
    ; wire.inspection.card.border_color
    ; wire.inspection.crosshair.color
    ; wire.inspection.marker.fill
    ; wire.inspection.marker.stroke
    ]
  in
  assert (List.for_all colors ~f:(Option.equal Int64.equal (Some 0x2dd4bfffL)));
  List.iter [ Float.nan; Float.infinity; Float.neg_infinity; -1.; 65537. ] ~f:(fun n ->
    assert (Result.is_error (I.Card.create ~width:n ()));
    assert (Result.is_error (I.Card.create ~gap:n ()));
    assert (Result.is_error (I.Card.create ~padding:n ()));
    assert (Result.is_error (I.Card.create ~radius:n ()));
    assert (Result.is_error (I.Card.create ~font_size:n ()));
    assert (Result.is_error (I.Card.create ~line_height:n ()));
    assert (Result.is_error (I.Card.create ~border_width:n ()));
    assert (Result.is_error (I.Crosshair.create ~thickness:n ()));
    assert (Result.is_error (I.Marker.create ~size:n ()));
    assert (Result.is_error (I.Marker.create ~stroke_width:n ())));
  assert (Result.is_error (I.Card.create ~font_size:32. ~line_height:31. ()));
  assert (Result.is_error (I.Marker.create ~size:2. ~stroke_width:2. ()));
  print_endline "All six colors resolved; invalid bounds and coupled dimensions rejected.";
  [%expect {| All six colors resolved; invalid bounds and coupled dimensions rejected. |}]
;;
