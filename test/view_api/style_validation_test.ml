open Core
open Gpuio
open Gpuio_protocol

let accepts property = Result.is_ok (Style.create [ property ])

let%expect_test "numeric boundaries are explicit rather than clamped" =
  let nonnegative : (float -> Style.Property.t) list =
    [ (fun v -> Grow v)
    ; (fun v -> Shrink v)
    ; (fun v -> Border_width v)
    ; (fun v -> Radius v)
    ]
  in
  List.iter nonnegative ~f:(fun property ->
    List.iter [ 0.; 1.; 1_000_000. ] ~f:(fun v -> assert (accepts (property v)));
    List.iter
      [ -1.; 1_000_001.; Float.nan; Float.infinity; Float.neg_infinity ]
      ~f:(fun v -> assert (not (accepts (property v)))));
  List.iter [ 0.001; 1_000_000. ] ~f:(fun v -> assert (accepts (Font_size v)));
  List.iter [ 0.; -1.; 1_000_001.; Float.nan; Float.infinity ] ~f:(fun v ->
    assert (not (accepts (Font_size v))));
  List.iter [ 0.; 1. ] ~f:(fun v -> assert (accepts (Opacity v)));
  List.iter [ -0.001; 1.001; Float.nan; Float.infinity ] ~f:(fun v ->
    assert (not (accepts (Opacity v))));
  let counts : (int -> Style.Property.t) list =
    [ (fun v -> Grid_columns v); (fun v -> Grid_rows v); (fun v -> Line_clamp v) ]
  in
  List.iter counts ~f:(fun property ->
    List.iter [ 1; 1024 ] ~f:(fun v -> assert (accepts (property v)));
    List.iter [ -1; 0; 1025 ] ~f:(fun v -> assert (not (accepts (property v)))));
  List.iter [ 1; 1000 ] ~f:(fun v -> assert (accepts (Font_weight v)));
  List.iter [ -1; 0; 1001 ] ~f:(fun v -> assert (not (accepts (Font_weight v))));
  print_endline "finite float bounds, opacity, font size, grid/clamp counts and weight";
  [%expect {| finite float bounds, opacity, font size, grid/clamp counts and weight |}]
;;

let%expect_test "length units and per-property auto/sign policies" =
  List.iter [ Length.px; Length.percent ] ~f:(fun length ->
    List.iter [ -1_000_000.; 0.; 1_000_000. ] ~f:(fun v ->
      assert (Result.is_ok (length v)));
    List.iter [ -1_000_001.; 1_000_001.; Float.nan; Float.infinity ] ~f:(fun v ->
      assert (Result.is_error (length v))));
  let dimensions : (Length.t -> Style.Property.t) list =
    [ (fun v -> Width v)
    ; (fun v -> Height v)
    ; (fun v -> Min_width v)
    ; (fun v -> Min_height v)
    ; (fun v -> Max_width v)
    ; (fun v -> Max_height v)
    ; (fun v -> Basis v)
    ]
  in
  let definite : (Length.t -> Style.Property.t) list =
    [ (fun v -> Padding v); (fun v -> Gap v); (fun v -> Line_height v) ]
  in
  let signed : (Length.t -> Style.Property.t) list =
    [ (fun v -> Margin v)
    ; (fun v -> Top v)
    ; (fun v -> Right v)
    ; (fun v -> Bottom v)
    ; (fun v -> Left v)
    ]
  in
  List.iter
    (dimensions @ definite @ signed)
    ~f:(fun property ->
      List.iter
        [ Length.px_exn 0.; Length.percent_exn 200. ]
        ~f:(fun v -> assert (accepts (property v))));
  List.iter (dimensions @ definite) ~f:(fun property ->
    assert (not (accepts (property (Length.px_exn (-1.))))));
  List.iter signed ~f:(fun property ->
    assert (accepts (property (Length.percent_exn (-10.)))));
  List.iter (dimensions @ signed) ~f:(fun property ->
    assert (accepts (property Length.auto)));
  List.iter definite ~f:(fun property -> assert (not (accepts (property Length.auto))));
  print_endline
    "percentage points above 100; signed margins/offsets; definite gaps/padding/leading";
  [%expect
    {| percentage points above 100; signed margins/offsets; definite gaps/padding/leading |}]
;;

let%expect_test "text and shadow limits count bytes and entries" =
  assert (
    Result.is_ok (Style.create (List.init 128 ~f:(fun _ -> Style.Property.Font_size 16.))));
  assert (
    Result.is_error
      (Style.create (List.init 129 ~f:(fun _ -> Style.Property.Font_size 16.))));
  List.iter [ 1; 256 ] ~f:(fun n -> assert (accepts (Font_family (String.make n 'a'))));
  List.iter [ 0; 257 ] ~f:(fun n ->
    assert (not (accepts (Font_family (String.make n 'a')))));
  assert (accepts (Font_family (String.concat (List.init 128 ~f:(fun _ -> "é")))));
  assert (not (accepts (Font_family (String.concat (List.init 129 ~f:(fun _ -> "é"))))));
  assert (accepts (Accessible_name (String.make 1024 'a')));
  assert (not (accepts (Accessible_name (String.make 1025 'a'))));
  let make ~offset_x ~blur ~spread =
    Shadow.create ~color:(Color.rgb_exn 0) ~offset_x ~offset_y:0. ~blur ~spread ()
  in
  let shadow =
    make ~offset_x:(-1_000_000.) ~blur:1_000_000. ~spread:(-1_000_000.) |> Or_error.ok_exn
  in
  List.iter [ 0; 8 ] ~f:(fun n ->
    assert (accepts (Shadows (List.init n ~f:(fun _ -> shadow)))));
  assert (not (accepts (Shadows (List.init 9 ~f:(fun _ -> shadow)))));
  assert (Result.is_error (make ~offset_x:0. ~blur:(-1.) ~spread:0.));
  List.iter [ Float.nan; Float.infinity; 1_000_001. ] ~f:(fun v ->
    assert (Result.is_error (make ~offset_x:v ~blur:0. ~spread:0.));
    assert (Result.is_error (make ~offset_x:0. ~blur:v ~spread:0.));
    assert (Result.is_error (make ~offset_x:0. ~blur:0. ~spread:v)));
  print_endline "font bytes 256; name bytes 1024; shadows 8; negative spread permitted";
  [%expect {| font bytes 256; name bytes 1024; shadows 8; negative spread permitted |}]
;;

let%expect_test "shorthands obey source list order, including later shorthand resets" =
  let show properties =
    match
      Style.create_exn properties
      |> Style.Expert.to_wire ~theme:Theme.default
      |> Or_error.ok_exn
    with
    | [ Wire.Style.Fields fields ] ->
      List.filter_map fields ~f:(function
        | Padding_left (Px v) -> Some (sprintf "left=%.0f" v)
        | Overflow_x v -> Some (sprintf "x=%Ld" v)
        | Overflow_y v -> Some (sprintf "y=%Ld" v)
        | _ -> None)
      |> String.concat ~sep:" "
      |> print_endline
    | _ -> assert false
  in
  show
    [ Padding (Length.px_exn 8.)
    ; Padding_left (Length.px_exn 20.)
    ; Overflow Scroll
    ; Overflow_x Hidden
    ];
  show
    [ Padding_left (Length.px_exn 20.)
    ; Padding (Length.px_exn 8.)
    ; Overflow_x Hidden
    ; Overflow Scroll
    ];
  [%expect
    {|
    left=20 x=2 y=3
    left=8 x=3 y=3
    |}]
;;

let%expect_test "state styles accept layout fields but keep interaction policy in base" =
  assert (
    Result.is_ok
      (Style.with_state Style.empty Hovered [ Width (Length.px_exn 80.); Display Hidden ]));
  let policies : Style.Property.t list =
    [ Pointer_occlusion Pointer
    ; Pointer_events false
    ; User_select true
    ; Selection_color (Color.rgb_exn 0xff0000)
    ; Accessible_name "Label"
    ; Inert true
    ]
  in
  List.iter policies ~f:(fun property ->
    assert (Result.is_error (Style.with_state Style.empty Hovered [ property ]));
    assert (Result.is_error (Style.with_state Style.empty Pressed [ property ]));
    assert (Result.is_ok (Style.create [ property ])));
  print_endline "layout accepted; six interaction declarations remain base-only";
  [%expect {| layout accepted; six interaction declarations remain base-only |}]
;;
