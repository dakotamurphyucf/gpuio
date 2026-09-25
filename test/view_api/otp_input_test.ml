open Core
module O = Gpuio.Otp_input

let policy ?alphabet length = O.Policy.create ~length ?alphabet () |> Or_error.ok_exn
let show result = print_s [%sexp (result : (O.Value.t, O.Input_error.t) Result.t)]

let%expect_test "OTP policies, normalization and bounded input" =
  List.iter [ 0; 1; 32; 33 ] ~f:(fun length ->
    printf "%d: %b\n" length (Result.is_ok (O.Policy.create ~length ())));
  let digits = policy 6 in
  List.iter [ ""; "１２3４"; "１２-3"; "١"; "１\194\1602"; "1234567"; "\255" ] ~f:(fun text ->
    show (O.Value.of_string digits text));
  show (O.Value.of_paste digits "１２-3 \t4\r\n");
  show (O.Value.of_paste digits (String.make O.max_input_bytes ' '));
  show (O.Value.of_paste digits (String.make (O.max_input_bytes + 1) ' '));
  show (O.Value.of_string (policy ~alphabet:Ascii_alphanumeric 6) "ａＡzＺ０9");
  [%expect
    {| 
    0: false
    1: true
    32: true
    33: false
    (Ok "")
    (Ok 1234)
    (Error (Unexpected_character (byte_offset 6)))
    (Error (Unexpected_character (byte_offset 0)))
    (Error (Unexpected_character (byte_offset 3)))
    (Error Too_long)
    (Error Invalid_utf8)
    (Ok 1234)
    (Ok "")
    (Error Input_too_large)
    (Ok aAzZ09)
  |}]
;;

let%expect_test "OTP selection edits are atomic and codes remain policy checked" =
  let p = policy 6 in
  let value =
    O.Value.of_string p "123456"
    |> Result.map_error ~f:(fun error -> Error.create_s (O.Input_error.sexp_of_t error))
    |> Or_error.ok_exn
  in
  let select anchor head =
    Gpuio.Text_input.Selection.create ~anchor ~head |> Or_error.ok_exn
  in
  let show result =
    print_s
      [%sexp
        (result : (O.Value.t * Gpuio.Text_input.Selection.t, O.Input_error.t) Result.t)]
  in
  show (O.Value.paste value ~policy:p ~selection:(select 4 2) ~text:"９-８");
  show (O.Value.paste value ~policy:p ~selection:(select 4 2) ~text:" - ");
  show (O.Value.replace value ~policy:p ~selection:(select 4 2) ~text:"");
  show (O.Value.replace value ~policy:p ~selection:(select 3 3) ~text:"7");
  show (O.Value.paste value ~policy:p ~selection:(select 4 2) ~text:"7x");
  show (O.Value.replace value ~policy:p ~selection:(select 7 0) ~text:"1");
  printf
    "original=%s complete=%b narrower=%b\n"
    (O.Value.to_string value)
    (O.Value.is_complete value ~policy:p)
    (O.Value.fits value ~policy:(policy 5));
  [%expect
    {| 
    (Ok (129856 ((anchor 4) (head 4))))
    (Ok (123456 ((anchor 4) (head 2))))
    (Ok (1256 ((anchor 2) (head 2))))
    (Error Too_long)
    (Error (Unexpected_character (byte_offset 1)))
    (Error Invalid_selection)
    original=123456 complete=true narrower=false
  |}]
;;

let%expect_test "OTP policy encoding and untrusted-policy guard" =
  let module W = Gpuio_protocol.Otp_wire in
  List.iter
    [ { W.Policy.length = 6; alphabet = Digits }
    ; { length = 32; alphabet = Ascii_alphanumeric }
    ]
    ~f:(fun policy ->
      Bin_prot.Utils.bin_dump W.Policy.bin_writer_t policy
      |> Bigstring.to_string
      |> String.iter ~f:(fun ch -> printf "%02x" (Char.to_int ch));
      print_endline "");
  let invalid = { W.Policy.length = 0; alphabet = Digits } in
  print_s
    [%sexp (W.normalize invalid ~paste:false "" : (string, W.Input_error.t) Result.t)];
  [%expect
    {| 
    0600
    2001
    (Error Invalid_policy)
  |}]
;;
