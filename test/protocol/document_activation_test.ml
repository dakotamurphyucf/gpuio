open Core
open Gpuio_protocol
module D = Document_wire

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "link activation paired bytes and malformed event validation" =
  let ok = Or_error.ok_exn in
  let window = Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Node_id.create ~slot:0L ~generation:1L |> ok in
  let handler = Handler_id.create ~slot:0L ~generation:1L |> ok in
  let source = Resource_id.create ~slot:0L ~generation:1L |> ok in
  let mods flag : Pointer_wire.Modifiers.t =
    { shift = flag; control = flag; alt = flag; command = flag; function_ = flag }
  in
  let encode url activation =
    Bin_prot.Utils.bin_dump
      [%bin_writer: Wire.Event.t list]
      [ Document_navigation
          (window, node, handler, 1L, source, 3L, Link_activated (url, activation))
      ]
    |> Bigstring.to_string
  in
  List.iter
    [ D.Activation.Source.Mouse Middle, true
    ; Keyboard, false
    ; Touch { long_press = true }, false
    ]
    ~f:(fun (source, flag) ->
      let bytes = encode "x" { source; modifiers = mods flag } in
      assert (Or_error.is_ok (Wire.Event.decode bytes));
      for n = 0 to String.length bytes - 1 do
        assert (Or_error.is_error (Wire.Event.decode (String.prefix bytes n)))
      done;
      assert (Or_error.is_error (Wire.Event.decode (bytes ^ "x")));
      print_endline (hex bytes));
  let malformed index =
    let data =
      Bytes.of_string (encode "x" { source = Mouse Middle; modifiers = mods false })
    in
    Bytes.set data index '\255';
    assert (Or_error.is_error (Wire.Event.decode (Bytes.to_string data)))
  in
  (* Unknown navigation/source/button tags and non-Boolean modifier bytes. *)
  List.iter [ 12; 15; 16; 17; 18; 19; 20; 21 ] ~f:malformed;
  List.iter
    [ ""; String.make 4097 'x'; "a\000b"; "\255" ]
    ~f:(fun url ->
      assert (
        Or_error.is_error
          (Wire.Event.decode (encode url { source = Keyboard; modifiers = mods false }))));
  List.iter
    [ D.Activation.Source.Keyboard; Touch { long_press = false } ]
    ~f:(fun source ->
      assert (
        Or_error.is_error
          (Wire.Event.decode (encode "x" { source; modifiers = mods true }))));
  [%expect
    {|
    011e0001000100010100010302017800020101010101
    011e00010001000101000103020178010000000000
    011e0001000100010100010302017802010000000000
  |}]
;;
