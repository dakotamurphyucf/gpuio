open Core
open Gpuio
module A = Split_button.Appearance
module W = Gpuio_protocol.Wire.Split_button

let ok = Or_error.ok_exn

let%expect_test "split operation tag and reset have independent paired bytes" =
  let module P = Gpuio_protocol in
  let node = P.Node_id.create ~slot:0L ~generation:1L |> ok in
  let window = P.Window_id.create ~slot:0L ~generation:1L |> ok in
  let config =
    Split_button.Expert.to_wire A.default ~parts:Split ~theme:Theme.default |> ok
  in
  let message =
    P.Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_split_button (node, Some config); Set_split_button (node, None) ]
      }
  in
  let bytes =
    Bin_prot.Utils.bin_dump P.Wire.Message.bin_writer_t message |> Bigstring.to_string
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    assert (
      String.equal
        hex
        (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "split-button-operation.hex")
         |> String.strip)));
  print_endline "Op70 set/reset matches the independent fixture";
  [%expect {| Op70 set/reset matches the independent fixture |}]
;;

let%expect_test "split parts validate and retain the surviving keyed menu owner" =
  let menu () =
    View.menu_button
      ~key:(Key.of_string_exn (String.make 256 'm'))
      ~menu:(Menu.create ~label:"More" [] |> ok)
      ()
  in
  let primary () =
    View.button ~key:(Key.of_string_exn "action") ~on_click:(fun () -> ()) "Run"
  in
  assert (Result.is_error (View.split_button ()));
  assert (Result.is_error (View.split_button ~primary:(View.text "bad") ()));
  assert (Result.is_error (View.split_button ~menu:(primary ()) ()));
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let submit view =
    let prepared = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept reconciler prepared |> ok;
    match Reconciler.message prepared with
    | Some (Gpuio_protocol.Wire.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  let single = View.split_button ~menu:(menu ()) () |> ok in
  let first = submit single in
  let menu_id =
    List.find_map_exn first ~f:(function
      | Gpuio_protocol.Wire.Op.Create (id, Menu, _, _) -> Some id
      | _ -> None)
  in
  let paired =
    submit (View.split_button ~primary:(primary ()) ~menu:(menu ()) () |> ok)
  in
  assert (
    List.exists paired ~f:(function
      | Gpuio_protocol.Wire.Op.Set_split_button (_, Some { parts = Split; _ }) -> true
      | _ -> false));
  let removed = submit single in
  List.iter [ paired; removed ] ~f:(fun operations ->
    assert (
      not
        (List.exists operations ~f:(function
           | Gpuio_protocol.Wire.Op.Remove id -> Gpuio_protocol.Node_id.equal id menu_id
           | Gpuio_protocol.Wire.Op.Create (_, Menu, _, _) -> true
           | _ -> false))));
  let plain = View.column [] in
  assert (
    List.exists (submit plain) ~f:(function
      | Gpuio_protocol.Wire.Op.Set_split_button (_, None) -> true
      | _ -> false));
  print_endline
    "invalid parts rejected; full caller key and menu identity survive mode changes; \
     reset emitted";
  [%expect
    {| invalid parts rejected; full caller key and menu identity survive mode changes; reset emitted |}]
;;

let%expect_test "split parts allow bounded tooltip anchors, not arbitrary wrappers" =
  let wrap anchor =
    View.tooltip
      ~config:(Tooltip.Config.create ~label:"Help" () |> ok)
      ~anchor
      ~content:(View.text "Help")
      ()
  in
  let nest n part = Fn.apply_n_times ~n wrap part in
  let primary = View.button ~on_click:(fun () -> ()) "Run" in
  let menu = View.menu_button ~menu:(Menu.create ~label:"More" [] |> ok) () in
  assert (
    Result.is_ok (View.split_button ~primary:(nest 8 primary) ~menu:(nest 8 menu) ()));
  assert (Result.is_error (View.split_button ~primary:(nest 9 primary) ()));
  assert (Result.is_error (View.split_button ~menu:(nest 9 menu) ()));
  assert (Result.is_error (View.split_button ~primary:(View.row [ primary ]) ()));
  assert (Result.is_error (View.split_button ~primary:(wrap menu) ()));
  print_endline "eight tooltip anchors accepted; excess nesting and wrong owners rejected";
  [%expect {| eight tooltip anchors accepted; excess nesting and wrong owners rejected |}]
;;

let%expect_test "default split hover region is compact but explicit layout can stretch" =
  let primary = View.button ~on_click:(fun () -> ()) "Run" in
  let alignment ?style () =
    let view = View.split_button ?style ~primary () |> ok in
    Style.Expert.to_wire (View.Expert.describe view).style ~theme:Theme.default
    |> ok
    |> List.concat_map ~f:(function
      | Gpuio_protocol.Wire.Style.Fields fields -> fields
      | _ -> [])
    |> List.filter_map ~f:(function
      | Gpuio_protocol.Wire.Field.Align_self value -> Some value
      | _ -> None)
  in
  assert (List.equal Int64.equal (alignment ()) [ 0L ]);
  assert (
    List.equal
      Int64.equal
      (alignment ~style:(Style.create_exn [ Align_self Stretch ]) ())
      [ 6L ]);
  print_endline "compact default; caller alignment override preserved";
  [%expect {| compact default; caller alignment override preserved |}]
;;

let%expect_test "split paint has independent paired bytes and explicit part modes" =
  let paint alpha =
    Style.create_exn [ Foreground (Color.rgba ~red:0 ~green:0 ~blue:0 ~alpha |> ok) ]
  in
  let appearance = A.create ~surface:(paint 0x11) ~menu_open:(paint 0x22) () |> ok in
  let wire =
    Split_button.Expert.to_wire appearance ~parts:Split ~theme:Theme.default |> ok
  in
  let bytes = Bin_prot.Utils.bin_dump W.bin_writer_t wire |> Bigstring.to_string in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "split-button-paint.hex")
      |> String.strip
    in
    assert (String.equal hex expected));
  List.iter [ W.Parts.Primary; Menu; Split ] ~f:(fun parts ->
    let wire = Split_button.Expert.to_wire A.default ~parts ~theme:Theme.default |> ok in
    assert (W.Parts.equal wire.parts parts);
    assert (List.is_empty wire.surface && List.is_empty wire.menu_open));
  print_endline "independent paint bytes; primary/menu/split modes";
  [%expect {| independent paint bytes; primary/menu/split modes |}]
;;

let%expect_test "shared paint cannot change layout or interaction" =
  List.iter
    [ Style.create_exn [ Width (Length.px_exn 40.) ]
    ; Style.create_exn [ Opacity 0.5 ]
    ; Style.create_exn [ Inert true ]
    ; Style.create_exn [ Disabled true ]
    ; Style.create_exn [ Pointer_events false ]
    ; Style.create_exn [ Font_size 20. ]
    ; Style.create_exn [ Radius 4. ]
    ]
    ~f:(fun style ->
      assert (Result.is_error (A.create ~surface:style ()));
      assert (Result.is_error (A.create ~menu_open:style ())));
  List.iter
    [ Style.State.Focused; Hovered; Pressed; Checked; Indeterminate; Disabled; Selected ]
    ~f:(fun state ->
      let style =
        Style.with_state_exn Style.empty state [ Foreground (Color.rgb_exn 0x112233) ]
      in
      assert (Result.is_error (A.create ~surface:style ())));
  print_endline "structural fields and nested interaction states rejected";
  [%expect {| structural fields and nested interaction states rejected |}]
;;

let%expect_test "shared paint resolves the current theme" =
  let appearance =
    A.create ~surface:(Style.create_exn [ Foreground (Color.token_exn "surface") ]) ()
    |> ok
  in
  assert (
    Result.is_error
      (Split_button.Expert.to_wire appearance ~parts:Split ~theme:Theme.default));
  let resolve color =
    Split_button.Expert.to_wire
      appearance
      ~parts:Split
      ~theme:(Theme.create [ "surface", Color.rgb_exn color ] |> ok)
    |> ok
  in
  assert (not (W.equal (resolve 0x123456) (resolve 0xabcdef)));
  print_endline "missing token rejected; theme replacement changes resolved paint";
  [%expect {| missing token rejected; theme replacement changes resolved paint |}]
;;
