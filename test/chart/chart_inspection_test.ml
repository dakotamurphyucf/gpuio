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

let%expect_test
    "cursor placement appends its wire tag and survives public style resolution"
  =
  let module P = Gpuio_protocol.Chart_inspection_wire.Placement in
  List.iteri [ I.Placement.Corner; Anchor; Cursor ] ~f:(fun tag placement ->
    let card = I.Card.create ~placement () |> ok in
    let wire =
      Gpuio.Chart_style.create ~inspection:(I.create ~card ()) ()
      |> ok
      |> Gpuio.Chart_style.Expert.to_wire
    in
    let bytes = Bin_prot.Utils.bin_dump P.bin_writer_t wire.inspection.card.placement in
    assert (Bigstring.length bytes = 1);
    assert (Char.to_int (Bigstring.get bytes 0) = tag);
    printf "%d\n" tag);
  [%expect
    {|
    0
    1
    2 |}]
;;

let%expect_test "guide spans validate independently and have paired wire bytes" =
  let module S = Gpuio_protocol.Chart_inspection_wire.Span in
  let spans =
    [ I.Span.full
    ; I.Span.pixels ~start:(-12.) ~length:30. |> ok
    ; I.Span.fraction ~start:0.25 ~length:0.5 |> ok
    ]
  in
  List.iter spans ~f:(fun span ->
    let crosshair =
      I.Crosshair.create ~vertical_span:span ~horizontal_span:span () |> ok
    in
    let wire =
      I.Expert.to_wire (I.create ~crosshair ()) ~theme:(Gpuio.Theme.create [] |> ok) |> ok
    in
    assert (S.equal wire.crosshair.vertical_span wire.crosshair.horizontal_span);
    let bytes = Bin_prot.Utils.bin_dump S.bin_writer_t wire.crosshair.vertical_span in
    String.iter (Bigstring.to_string bytes) ~f:(fun c -> printf "%02x" (Char.to_int c));
    print_endline "");
  List.iter
    [ Float.nan; Float.infinity; Float.neg_infinity; -32769.; 32769. ]
    ~f:(fun start -> assert (Result.is_error (I.Span.pixels ~start ~length:0.)));
  List.iter [ Float.nan; Float.infinity; -0.01; 65537. ] ~f:(fun length ->
    assert (Result.is_error (I.Span.pixels ~start:0. ~length)));
  List.iter [ Float.nan; Float.infinity; -1.01; 1.01 ] ~f:(fun start ->
    assert (Result.is_error (I.Span.fraction ~start ~length:0.)));
  List.iter [ Float.nan; Float.infinity; -0.01; 2.01 ] ~f:(fun length ->
    assert (Result.is_error (I.Span.fraction ~start:0. ~length)));
  List.iter [ -32768.; 32768. ] ~f:(fun start ->
    List.iter [ 0.; 65536. ] ~f:(fun length ->
      ignore (I.Span.pixels ~start ~length |> ok : I.Span.t)));
  List.iter [ -1.; 1. ] ~f:(fun start ->
    List.iter [ 0.; 2. ] ~f:(fun length ->
      ignore (I.Span.fraction ~start ~length |> ok : I.Span.t)));
  [%expect
    {|
    00
    0100000000000028c00000000000003e40
    02000000000000d03f000000000000e03f |}]
;;
