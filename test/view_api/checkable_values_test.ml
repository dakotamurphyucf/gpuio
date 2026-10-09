open Core
open Gpuio
module W = Gpuio_protocol.Checkable_wire

let ok = Or_error.ok_exn

let fixture writer value name =
  let hex =
    Bin_prot.Utils.bin_dump writer value
    |> Bigstring.to_string
    |> String.to_list
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected = Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip in
    assert (String.equal hex expected))
;;

let%expect_test "Tab order has explicit stop policy and bounded signed order" =
  assert (Tab_order.equal Tab_order.default (Tab_order.create () |> ok));
  List.iter [ true; false ] ~f:(fun tab_stop ->
    List.iter [ -1_000_000; -1; 0; 1; 1_000_000 ] ~f:(fun index ->
      let value = Tab_order.create ~tab_stop ~index () |> ok in
      assert (Bool.equal (Tab_order.tab_stop value) tab_stop);
      assert (Tab_order.index value = index);
      assert (
        Tab_order.equal
          value
          (Tab_order.Expert.of_wire (Tab_order.Expert.to_wire value) |> ok))));
  List.iter [ Int64.min_value; -1_000_001L; 1_000_001L; Int64.max_value ] ~f:(fun index ->
    assert (Result.is_error (Tab_order.Expert.of_wire { tab_stop = true; index })));
  assert (Result.is_error (Tab_order.create ~index:1_000_001 ()));
  let value = Tab_order.create ~tab_stop:false ~index:(-2) () |> ok in
  fixture
    W.Tab_order.bin_writer_t
    (Tab_order.Expert.to_wire value)
    "checkable-tab-order.hex";
  print_s [%sexp (Tab_order.default : Tab_order.t), (value : Tab_order.t)];
  [%expect {| (((tab_stop true) (index 0)) ((tab_stop false) (index -2))) |}]
;;

let%expect_test "Radio set positions validate the pair before native conversion" =
  List.iter [ 1; 5; 100_000 ] ~f:(fun count ->
    List.iter
      [ 0; count - 1 ]
      ~f:(fun index ->
        let value = Radio.Position.create ~index ~count |> ok in
        assert (Radio.Position.index value = index);
        assert (Radio.Position.count value = count);
        assert (
          Radio.Position.equal
            value
            (Radio.Position.Expert.of_wire (Radio.Position.Expert.to_wire value) |> ok))));
  List.iter
    [ -1, 5; 0, 0; 5, 5; 0, 100_001 ]
    ~f:(fun (index, count) ->
      assert (Result.is_error (Radio.Position.create ~index ~count)));
  List.iter
    [ Int64.min_value, 1L; 0L, Int64.max_value; Int64.max_value, Int64.max_value ]
    ~f:(fun (index, count) ->
      assert (Result.is_error (Radio.Position.Expert.of_wire { index; count })));
  let position = Radio.Position.create ~index:2 ~count:5 |> ok in
  fixture
    W.Position.bin_writer_t
    (Radio.Position.Expert.to_wire position)
    "checkable-position.hex";
  print_s [%sexp (position : Radio.Position.t)];
  [%expect {| ((index 2) (count 5)) |}]
;;

let%expect_test "standalone radio and Tab order retain owner while retiring activation" =
  let module Wire = Gpuio_protocol.Wire in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let commit view =
    let update = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept r update |> ok;
    match Reconciler.message update with
    | Some (Apply { operations; _ }) -> operations
    | None -> []
    | Some _ -> assert false
  in
  let radio ?tab_order checked action =
    View.radio
      ~key:(Key.of_string_exn "radio")
      ?tab_order
      ~checked
      ~position:(Radio.Position.create ~index:0 ~count:2 |> ok)
      ~on_select:(fun () -> action)
      "Choose"
  in
  let order = Tab_order.create ~index:(-2) ~tab_stop:false () |> ok in
  let ops = commit (radio ~tab_order:order false 1) in
  let owner, handler =
    List.find_map_exn ops ~f:(function
      | Wire.Op.Create (id, Radio, _, Some handler) -> Some (id, handler)
      | _ -> None)
  in
  assert (
    List.exists ops ~f:(function
      | Wire.Op.Set_tab_order (id, Some value) ->
        Gpuio_protocol.Node_id.equal id owner
        && W.Tab_order.equal value (Tab_order.Expert.to_wire order)
      | _ -> false));
  let old_press = Wire.Event.Press (window, owner, handler, 1L) in
  assert (Option.equal Int.equal (Reconciler.dispatch r old_press) (Some 1));
  let ops = commit (radio true 2) in
  assert (
    List.exists ops ~f:(function
      | Wire.Op.Set_tab_order (_, None) -> true
      | _ -> false));
  assert (
    List.exists ops ~f:(function
      | Wire.Op.Bind (_, None) -> true
      | _ -> false));
  assert (
    not
      (List.exists ops ~f:(function
         | Wire.Op.Create _ | Remove _ -> true
         | _ -> false)));
  assert (Option.is_none (Reconciler.dispatch r old_press));
  let ops = commit (radio false 3) in
  let next_handler =
    List.find_map_exn ops ~f:(function
      | Wire.Op.Bind (_, Some id) -> Some id
      | _ -> None)
  in
  assert (not (Gpuio_protocol.Handler_id.equal handler next_handler));
  assert (Option.is_none (Reconciler.dispatch r old_press));
  assert (
    Option.equal
      Int.equal
      (Reconciler.dispatch r (Press (window, owner, next_handler, 3L)))
      (Some 3));
  let rich =
    View.radio_with_label
      ~key:(Key.of_string_exn "radio")
      ~accessible_name:"Choose"
      ~checked:false
      ~on_select:(fun () -> 4)
      (View.text "Rich caption")
    |> ok
  in
  let ops = commit rich in
  assert (
    not
      (List.exists ops ~f:(function
         | Wire.Op.Remove id -> Gpuio_protocol.Node_id.equal id owner
         | _ -> false)));
  let group = Accessibility.create ~role:(Radio_group Vertical) ~label:"Modes" () |> ok in
  assert (Result.is_ok (View.with_accessibility (View.column [ rich ]) group));
  assert (Result.is_error (View.with_accessibility rich group));
  fixture
    Gpuio_protocol.Accessibility_wire.Config.bin_writer_t
    (Accessibility.Expert.to_wire group)
    "checkable-radio-group.hex";
  print_endline
    "stable radio, checked callback retirement, fresh reactivation, order reset, rich \
     label and explicit group";
  [%expect
    {| stable radio, checked callback retirement, fresh reactivation, order reset, rich label and explicit group |}]
;;

let%expect_test "standalone radio kind/control and Tab order have independent bytes" =
  let module Wire = Gpuio_protocol.Wire in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let message =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Radio, "Mode", Some handler)
          ; Set_control (node, Radio (false, Some { index = 2L; count = 5L }, false))
          ; Set_tab_order (node, Some { tab_stop = false; index = -2L })
          ]
      }
  in
  fixture Wire.Message.bin_writer_t message "checkable-transaction.hex";
  print_endline "Kind52 / Control3 / Op68";
  [%expect {| Kind52 / Control3 / Op68 |}]
;;

let%expect_test "checkable navigation consumes the last positive capability bit" =
  let module Wire = Gpuio_protocol.Wire in
  assert (Int64.equal Wire.capabilities Int64.max_value);
  let bit = Int64.shift_left 1L 62 in
  let hex mask =
    Wire.Message.encode (Hello (Wire.version, mask))
    |> ok
    |> String.to_list
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  assert (String.equal (hex bit) "0003fc0000000000000040");
  assert (String.equal (hex Wire.capabilities) "0003fcffffffffffffff7f");
  [%expect {| |}]
;;
