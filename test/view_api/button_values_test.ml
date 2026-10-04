open Core
open Gpuio
module W = Gpuio_protocol.Button_wire

let ok = Or_error.ok_exn

let fixture policy content name =
  let value = { W.Config.policy = Button.Expert.to_wire policy; content } in
  assert (W.Config.valid value);
  let hex =
    Bin_prot.Utils.bin_dump W.Config.bin_writer_t value
    |> Bigstring.to_string
    |> String.to_list
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected = Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip in
    assert (String.equal expected hex))
;;

let%expect_test "button focus policies distinguish a skipped stop from preserving focus" =
  let default = Button.Config.default in
  assert (not (Button.Config.is_loading default));
  assert (Button.Focus.equal (Button.Config.focus default) (Focusable Tab_order.default));
  let skip = Tab_order.create ~tab_stop:false ~index:(-2) () |> ok in
  let loading = Button.Config.create ~loading:true ~focus:(Focusable skip) () in
  let preserve = Button.Config.create ~focus:Preserve () in
  assert (
    not (Button.Focus.equal (Button.Config.focus loading) (Button.Config.focus preserve)));
  List.iter [ default; loading; preserve ] ~f:(fun value ->
    assert (
      Button.Config.equal value (Button.Expert.of_wire (Button.Expert.to_wire value) |> ok)));
  fixture default Icon_slots "button-default.hex";
  fixture loading Rich "button-loading.hex";
  fixture preserve Rich "button-preserve.hex";
  print_s
    [%sexp
      (default : Button.Config.t)
    , (loading : Button.Config.t)
    , (preserve : Button.Config.t)];
  [%expect
    {| (((loading false) (focus (Focusable ((tab_stop true) (index 0)))))
  ((loading true) (focus (Focusable ((tab_stop false) (index -2)))))
  ((loading false) (focus Preserve))) |}]
;;

let%expect_test "button wire conversion revalidates signed Tab indices" =
  List.iter [ Int64.min_value; -1_000_001L; 1_000_001L; Int64.max_value ] ~f:(fun index ->
    List.iter [ true; false ] ~f:(fun loading ->
      let policy : W.Policy.t =
        { loading; focus = Focusable { tab_stop = false; index } }
      in
      assert (not (W.Policy.valid policy));
      assert (Result.is_error (Button.Expert.of_wire policy))));
  List.iter [ -1_000_000; -1; 0; 1; 1_000_000 ] ~f:(fun index ->
    let policy =
      Button.Config.create ~focus:(Focusable (Tab_order.create ~index () |> ok)) ()
    in
    assert (W.Policy.valid (Button.Expert.to_wire policy)));
  [%expect {| |}]
;;

let%expect_test "button operation has a paired append-only tag and explicit reset" =
  let module Wire = Gpuio_protocol.Wire in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let message : Wire.Message.t =
    Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_button_presentation
              ( node
              , Some
                  { W.Config.policy = { loading = false; focus = Preserve }
                  ; content = Rich
                  } )
          ; Set_button_presentation (node, None)
          ]
      }
  in
  let hex =
    Bin_prot.Utils.bin_dump Wire.Message.bin_writer_t message
    |> Bigstring.to_string
    |> String.to_list
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "button-operation.hex") |> String.strip
    in
    assert (String.equal hex expected));
  [%expect {| |}]
;;

let%expect_test
    "current epoch rejects old and future native responses before opening windows"
  =
  let module Wire = Gpuio_protocol.Wire in
  assert (Int64.equal Wire.version 3L);
  List.iter [ 0L; 1L; 2L; 4L; Int64.max_value ] ~f:(fun protocol_version ->
    assert (
      Result.is_error
        (Wire.validate_welcome
           ~protocol_version
           ~available_capabilities:Wire.capabilities)));
  List.iter
    [ 0L; Int64.min_value; Int64.minus_one; Int64.(Wire.capabilities - 1L) ]
    ~f:(fun available_capabilities ->
      assert (
        Result.is_error
          (Wire.validate_welcome ~protocol_version:Wire.version ~available_capabilities)));
  Wire.validate_welcome
    ~protocol_version:Wire.version
    ~available_capabilities:Wire.capabilities
  |> ok;
  [%expect {| |}]
;;
