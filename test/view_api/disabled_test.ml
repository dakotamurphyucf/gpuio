open Core
open Gpuio
module P = Gpuio_protocol
module W = P.Wire

let ok = Or_error.ok_exn

let%expect_test "disabled subtree capability agrees with the native host" =
  let capability = Int64.shift_left 1L 54 in
  assert (Int64.equal (Int64.bit_and W.capabilities capability) capability);
  List.iter [ capability; W.capabilities ] ~f:(fun required ->
    W.Message.encode (Hello (W.version, required))
    |> ok
    |> String.iter ~f:(fun byte -> printf "%02x" (Char.to_int byte));
    print_endline "");
  [%expect
    {|
    0001fc0000000000004000
    0001fcffffffffffff7f00
    |}]
;;

let%expect_test
    "disabled is a base-only interaction property with normal style replacement"
  =
  let disabled = Style.create_exn [ Disabled true ] in
  List.iter [ Style.State.Hovered; Focused; Pressed; Disabled ] ~f:(fun state ->
    assert (Result.is_error (Style.with_state Style.empty state [ Disabled true ])));
  let cleared = Style.merge [ disabled; Style.create_exn [ Disabled false ] ] in
  assert (
    W.Style.equal
      (List.hd_exn (Style.Expert.to_wire cleared ~theme:Theme.default |> ok))
      (Fields [ Disabled false ]));
  Eio_main.run (fun env ->
    let hex =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "disabled-request.hex") |> String.strip
    in
    let expected =
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    let request =
      W.Message.Apply
        { window = P.Window_id.create ~slot:0L ~generation:1L |> ok
        ; base = 0L
        ; revision = 1L
        ; operations =
            [ Set_style
                ( P.Node_id.create ~slot:0L ~generation:1L |> ok
                , Style.Expert.to_wire disabled ~theme:Theme.default |> ok )
            ]
        }
    in
    assert (String.equal expected (W.Message.encode request |> ok));
    let pos_ref = ref 0 in
    assert (
      W.Message.equal
        request
        (W.Message.bin_read_t (Bigstring.of_string expected) ~pos_ref));
    assert (!pos_ref = String.length expected));
  print_endline
    "disabled field tag 69 agrees with independent fixture; state variants reject; base \
     replacement clears";
  [%expect
    {| disabled field tag 69 agrees with independent fixture; state variants reject; base replacement clears |}]
;;
