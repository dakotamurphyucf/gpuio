open Core
open Gpuio
module P = Gpuio_protocol
module W = P.Wire

let ok = Or_error.ok_exn

let%expect_test "inert is a base-only interaction property with normal style replacement" =
  let inert = Style.create_exn [ Inert true ] in
  List.iter [ Style.State.Hovered; Focused; Pressed; Disabled ] ~f:(fun state ->
    assert (Result.is_error (Style.with_state Style.empty state [ Inert true ])));
  let cleared = Style.merge [ inert; Style.create_exn [ Inert false ] ] in
  assert (
    W.Style.equal
      (List.hd_exn (Style.Expert.to_wire cleared ~theme:Theme.default |> ok))
      (Fields [ Inert false ]));
  Eio_main.run (fun env ->
    let hex =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "inert-request.hex") |> String.strip
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
                , Style.Expert.to_wire inert ~theme:Theme.default |> ok )
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
    "inert field tag 65 agrees with independent fixture; state variants reject; base \
     replacement clears";
  [%expect
    {| inert field tag 65 agrees with independent fixture; state variants reject; base replacement clears |}]
;;
