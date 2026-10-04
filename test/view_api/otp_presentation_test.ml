open Core
open Gpuio
module W = Gpuio_protocol.Wire
module P = Gpuio_protocol.Otp_presentation_wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let policy = Otp_input.Policy.create ~length:6 () |> ok
let config = Otp_input.Config.create ~policy ~label:"Code" () |> ok

let view ?appearance () =
  View.otp_input
    ?appearance
    ~controller:(Key.of_string_exn "code")
    ~config
    ~initial:Otp_input.Value.empty
    ~on_event:Fn.id
    ()
;;

let commit r theme view =
  let update = Reconciler.prepare r ~theme (Some view) |> ok in
  Reconciler.accept r update |> ok;
  match Reconciler.message update with
  | Some (W.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let%expect_test
    "OTP appearance is validated, theme-resolved and independent of native ownership"
  =
  List.iter [ Float.nan; Float.infinity; -1.; 4097. ] ~f:(fun bad ->
    assert (Result.is_error (Otp_input.Appearance.create ~cell_width:bad ()));
    assert (Result.is_error (Otp_input.Appearance.create ~cell_gap:bad ()));
    assert (Result.is_error (Otp_input.Appearance.create ~radius:bad ())));
  List.iter [ 0; 33 ] ~f:(fun groups ->
    assert (Result.is_error (Otp_input.Appearance.create ~groups ())));
  assert (Result.is_error (Otp_input.Appearance.create ~cell_width:0.5 ()));
  assert (Result.is_error (Otp_input.Appearance.create ~border_width:65. ()));
  let appearance =
    Otp_input.Appearance.create
      ~groups:2
      ~cell_width:40.
      ~background:(Color.token_exn "cell")
      ()
    |> ok
  in
  let theme value = Theme.create [ "cell", Color.rgb_exn value ] |> ok in
  let r = Reconciler.create window in
  let initial = commit r (theme 0x112233) (view ()) in
  let owner =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (node, Otp_input, _, _) -> Some node
      | _ -> None)
  in
  let check expected = function
    | [ W.Op.Set_otp_appearance (node, Some value) ] ->
      assert (Gpuio_protocol.Node_id.equal owner node);
      assert (value.groups = 2);
      assert (Option.equal Int64.equal value.background (Some expected))
    | _ -> assert false
  in
  check 0x112233ffL (commit r (theme 0x112233) (view ~appearance ()));
  check 0x445566ffL (commit r (theme 0x445566) (view ~appearance ()));
  assert (List.is_empty (commit r (theme 0x445566) (view ~appearance ())));
  let revision = Reconciler.revision r in
  assert (
    Result.is_error
      (Reconciler.prepare r ~theme:Theme.default (Some (view ~appearance ()))));
  assert (Int64.equal revision (Reconciler.revision r));
  (match commit r Theme.default (view ~appearance:Otp_input.Appearance.default ()) with
   | [ W.Op.Set_otp_appearance (node, None) ] ->
     assert (Gpuio_protocol.Node_id.equal owner node)
   | _ -> assert false);
  assert (List.is_empty (commit r Theme.default (view ())));
  print_endline
    "only appearance changes; theme failure is atomic; default removes override without \
     replacing editor";
  [%expect
    {| only appearance changes; theme failure is atomic; default removes override without replacing editor |}]
;;

let%expect_test "independent OTP presentation operation fixture" =
  let value =
    { P.default with
      groups = 2
    ; cell_width = Some 40.
    ; background = Some 0x11223344L
    ; focus_border = Some 0xffffffffL
    ; caret = Some 0L
    }
  in
  let message =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_otp_appearance (node, Some value); Set_otp_appearance (node, None) ]
      }
  in
  let bytes =
    Bin_prot.Utils.bin_dump W.Message.bin_writer_t message |> Bigstring.to_string
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  let expected =
    Eio_main.run (fun env ->
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "otp-presentation.hex") |> String.strip)
  in
  assert (String.equal hex expected);
  assert (P.valid value);
  assert (not (P.valid { value with caret = Some (-1L) }));
  print_endline "tag 81, optional geometry/colors, and reset match independent bytes";
  [%expect {| tag 81, optional geometry/colors, and reset match independent bytes |}]
;;
