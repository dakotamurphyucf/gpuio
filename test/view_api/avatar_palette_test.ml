open Core
open Gpuio
module P = Avatar.Palette
module W = Gpuio_protocol.Wire

let key = Key.of_string_exn
let ok = Or_error.ok_exn

let opaque_rgb color =
  match Color.Expert.value color with
  | Rgba value ->
    assert (Int64.equal (Int64.bit_and value 255L) 255L);
    Int64.shift_right_logical value 8 |> Int64.to_int_exn
  | Token _ | Opacity _ -> failwith "palette must use concrete opaque colors"
;;

let luminance color =
  let rgb = opaque_rgb color in
  let channel shift =
    let value = Float.of_int ((rgb lsr shift) land 255) /. 255. in
    if Float.(value <= 0.04045)
    then value /. 12.92
    else ((value +. 0.055) /. 1.055) ** 2.4
  in
  (0.2126 *. channel 16) +. (0.7152 *. channel 8) +. (0.0722 *. channel 0)
;;

let contrast first second =
  let first, second = luminance first, luminance second in
  (Float.max first second +. 0.05) /. (Float.min first second +. 0.05)
;;

let%expect_test "avatar palette identity mapping is stable over exact bytes" =
  List.iter
    [ "a", 4
    ; "hello", 3
    ; "ada", 7
    ; "AL", 4
    ; "京都", 4
    ; "\000\255", 4
    ; String.make 256 'a', 5
    ]
    ~f:(fun (identity, index) ->
      let light = P.for_key (key identity) ~appearance:Light in
      let dark = P.for_key (key identity) ~appearance:Dark in
      assert (P.index light = index && P.index dark = index);
      assert (not (Color.equal (P.background light) (P.background dark))));
  let fallback = Avatar.Fallback.create "京都" |> ok in
  assert (
    P.equal
      (P.for_fallback fallback ~appearance:Light)
      (P.for_key (key "京都") ~appearance:Light));
  let for_identity = P.for_key (key "person-42") ~appearance:Dark in
  List.iter [ "AL"; "Ada"; "京都" ] ~f:(fun display ->
    let config =
      Avatar.Config.create
        ~fallback:(Avatar.Fallback.create display |> ok)
        ~description:Image.Description.decorative
        ()
    in
    ignore (View.avatar ~style:(P.style for_identity) config : unit View.t);
    assert (P.equal for_identity (P.for_key (key "person-42") ~appearance:Dark)));
  print_endline
    "fixed ASCII/UTF-8/binary/256-byte mappings; appearance-independent index; key \
     identity independent of displayed initials";
  [%expect
    {| fixed ASCII/UTF-8/binary/256-byte mappings; appearance-independent index; key identity independent of displayed initials |}]
;;

let%expect_test
    "all fixed light/dark avatar pairs meet nominal contrast after byte quantization"
  =
  let representatives =
    [ "10"; "2"; "3"; "0"; "1"; "6"; "7"; "4"; "5"; "13"; "12"; "11" ]
  in
  List.iter [ P.Appearance.Light; Dark ] ~f:(fun appearance ->
    List.iteri representatives ~f:(fun index identity ->
      let palette = P.for_key (key identity) ~appearance in
      assert (P.index palette = index);
      let ratio = contrast (P.background palette) (P.foreground palette) in
      assert (Float.(ratio >= 4.5));
      printf
        "%d #%06X #%06X #%06X %.3f\n"
        index
        (opaque_rgb (P.background palette))
        (opaque_rgb (P.foreground palette))
        (opaque_rgb (P.border palette))
        ratio));
  [%expect
    {|
    0 #FFEDF4 #A13760 #F8CED9 5.779
    1 #FFEEE9 #A63A2D #FACFC7 5.716
    2 #FFF1E0 #9C4900 #F4D4BA 5.617
    3 #FDF5DD #835C00 #E7DAB6 5.515
    4 #F2F9E1 #5A6E00 #D6E0BC 5.297
    5 #E7FCEA #007932 #C4E5CA 5.160
    6 #DFFDF6 #007C66 #B8E6DC 4.790
    7 #DEFCFF #007790 #B5E4ED 4.819
    8 #E3F9FF #006AAC #BEE0F9 5.266
    9 #ECF5FF #445AB5 #CEDAFD 5.650
    10 #F8F1FF #714BA8 #DFD4F8 5.809
    11 #FFEEFF #8F3E8A #EFCFEB 5.870
    0 #42232D #FFA6C1 #572F3C 7.601
    1 #44241F #FFA99A #59302A 7.506
    2 #402712 #F9B379 #54351A 7.747
    3 #382C0C #E0C16D #493B13 7.836
    4 #2B3113 #BBCF7C #39421B 7.947
    5 #1A3520 #8FD89E #24462C 7.931
    6 #07362F #68DCC7 #0C473E 8.018
    7 #03343C #60D8EB #05454E 8.024
    8 #133144 #7ECEFF #1C4159 7.842
    9 #242C47 #A8C1FF #313B5D 7.690
    10 #322843 #CFB4FF #423658 7.647
    11 #3C243A #EDAAE6 #4F314C 7.634
    |}]
;;

let%expect_test
    "palette style supplies colors without changing avatar semantics or geometry"
  =
  let palette = P.for_key (key "ada") ~appearance:Dark in
  let fields style =
    match Style.Expert.to_wire style ~theme:(Theme.create [] |> ok) |> ok with
    | [ W.Style.Fields fields ] -> fields
    | styles -> raise_s [%message "unexpected palette style" (styles : W.Style.t list)]
  in
  assert (List.length (fields (P.style palette)) = 3);
  let override = Color.rgb_exn 0x123456 in
  let styled =
    Style.merge [ P.style palette; Style.create_exn [ Foreground override ] ]
  in
  let color =
    match Color.Expert.value override with
    | Rgba value -> W.Color.Rgba value
    | _ -> assert false
  in
  assert (List.mem (fields styled) (W.Field.Foreground color) ~equal:W.Field.equal);
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let view appearance =
    View.avatar
      ~style:(P.style (P.for_key (key "ada") ~appearance))
      (Avatar.Config.create
         ~fallback:(Avatar.Fallback.create "AL" |> ok)
         ~description:(Image.Description.label "Ada" |> ok)
         ())
  in
  let commit view =
    let update =
      Reconciler.prepare reconciler ~theme:(Theme.create [] |> ok) (Some view) |> ok
    in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  ignore (commit (view Light) : W.Op.t list);
  let changed = commit (view Dark) in
  assert (List.length changed = 1);
  (match changed with
   | [ W.Op.Set_style _ ] -> ()
   | _ -> failwith "palette update changed more than style");
  assert (List.is_empty (commit (view Dark)));
  Reconciler.close reconciler;
  print_endline
    "three concrete colors; caller foreground override; no theme tokens; appearance \
     change is style-only and repeats idle";
  [%expect
    {| three concrete colors; caller foreground override; no theme tokens; appearance change is style-only and repeats idle |}]
;;
