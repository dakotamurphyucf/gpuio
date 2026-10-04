open Core
open Gpuio_protocol

let%expect_test
    "native appearance snapshots preserve every variant and reject old layouts"
  =
  let prefix =
    "\001\034\000\001\001t" ^ String.make 52 '\000' ^ "\000\001\001\001\001\001"
  in
  List.iteri
    Window_wire.Appearance.[ Light; Vibrant_light; Dark; Vibrant_dark ]
    ~f:(fun tag expected ->
      let bytes = prefix ^ String.of_char (Char.of_int_exn tag) in
      (match Wire.Event.decode bytes |> Or_error.ok_exn with
       | [ Window_changed (_, snapshot) ] ->
         assert (Window_wire.Appearance.equal snapshot.appearance expected);
         print_s
           [%sexp
             (snapshot.appearance : Window_wire.Appearance.t)
           , (Window_wire.Appearance.is_dark snapshot.appearance : bool)]
       | _ -> assert false);
      List.iter
        (List.range 0 (String.length bytes))
        ~f:(fun length ->
          assert (Result.is_error (Wire.Event.decode (String.prefix bytes length)))));
  assert (Result.is_error (Wire.Event.decode (prefix ^ "\004")));
  [%expect
    {|
    (Light false)
    (Vibrant_light false)
    (Dark true)
    (Vibrant_dark true)
  |}]
;;

let%expect_test "window selection uses bounded UTF-8 replies and additive tags" =
  let id = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  List.iter
    [ Wire.Window.Command.Has_text_selection, "\011\007\000\001\010"
    ; Selected_text 4L, "\011\007\000\001\011\004"
    ; Clear_text_selection, "\011\007\000\001\012"
    ; End_text_selection, "\011\007\000\001\013"
    ]
    ~f:(fun (command, expected) ->
      assert (
        String.equal
          (Wire.Message.encode (Window_command (7L, id, command)) |> Or_error.ok_exn)
          expected));
  List.iter [ -1L; 262145L; Int64.max_value ] ~f:(fun limit ->
    assert (
      Result.is_error (Wire.Message.encode (Window_command (7L, id, Selected_text limit)))));
  List.iter [ 0L; 262144L ] ~f:(fun limit ->
    ignore
      (Wire.Message.encode (Window_command (7L, id, Selected_text limit))
       |> Or_error.ok_exn
       : string));
  List.iter
    [ "\003\001", Wire.Window.Response.Selection_present true
    ; "\004\004é\n ", Selected_text "é\n "
    ; "\005", Selection_updated
    ; "\001\006", Failed Limit_exceeded
    ]
    ~f:(fun (suffix, expected) ->
      let bytes = "\001\035\007\000\001" ^ suffix in
      (match Wire.Event.decode bytes |> Or_error.ok_exn with
       | [ Window_response (7L, window, response) ] ->
         assert (Window_id.equal window id);
         assert (Wire.Window.Response.equal response expected)
       | _ -> assert false);
      List.iter
        (List.range 0 (String.length bytes))
        ~f:(fun length ->
          assert (Result.is_error (Wire.Event.decode (String.prefix bytes length)))));
  (* Maximum legal string length, one byte beyond, and invalid UTF-8. *)
  let reply length suffix = "\001\035\007\000\001\004" ^ length ^ suffix in
  ignore
    (Wire.Event.decode (reply "\253\000\000\004\000" (String.make 262144 'x'))
     |> Or_error.ok_exn
     : Wire.Event.t list);
  assert (
    Result.is_error
      (Wire.Event.decode (reply "\253\001\000\004\000" (String.make 262145 'x'))));
  assert (Result.is_error (Wire.Event.decode (reply "\001" "\255")));
  print_endline "selection tags, limits, Unicode and truncation checked";
  [%expect {| selection tags, limits, Unicode and truncation checked |}]
;;

let id = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "window command and configured-open independent fixtures" =
  let messages : Wire.Message.t list =
    [ Window_command (7L, id, Observe)
    ; Window_command (7L, id, Set_title "x")
    ; Window_command (7L, id, Resize (640., 400.))
    ; Window_command (7L, id, Activate)
    ; Window_command (7L, id, Zoom)
    ; Window_command (7L, id, Toggle_fullscreen)
    ; Window_command (7L, id, Minimize)
    ; Window_command (7L, id, Set_edited true)
    ; Window_command (7L, id, Set_document { path = None; edited = false })
    ; Window_command (7L, id, Set_document { path = Some "/tmp/\255"; edited = true })
    ; Open_configured
        ( 7L
        , id
        , { title = "x"
          ; width = 640.
          ; height = 400.
          ; focus = false
          ; chrome = Hidden
          ; resizable = false
          ; frame = Window_wire.Frame.default
          } )
    ]
  in
  List.iter messages ~f:(fun message ->
    print_endline (hex (Wire.Message.encode message |> Or_error.ok_exn)));
  [%expect
    {| 
0b07000100
0b070001010178
0b0700010200000000000084400000000000007940
0b07000103
0b07000104
0b07000105
0b07000108
0b0700010601
0b070001070000
0b0700010701062f746d702fff01
0c07000101780000000000008440000000000000794000010000000000000034400000000000001040
|}]
;;

let%expect_test "custom titlebar keeps the configured-open wire tag" =
  let message =
    Wire.Message.Open_configured
      ( 7L
      , id
      , { title = "x"
        ; width = 640.
        ; height = 400.
        ; focus = false
        ; chrome = Custom
        ; resizable = false
        ; frame = Window_wire.Frame.default
        } )
  in
  print_endline (hex (Wire.Message.encode message |> Or_error.ok_exn));
  [%expect
    {| 0c07000101780000000000008440000000000000794000020000000000000034400000000000001040 |}]
;;

let%expect_test "invalid outgoing window data is rejected" =
  List.iter
    [ Wire.Window.Command.Resize (Float.nan, 400.)
    ; Resize (0., 400.)
    ; Set_title "bad\000title"
    ; Set_document { path = Some "relative"; edited = false }
    ]
    ~f:(fun command ->
      print_s
        [%sexp
          (Result.is_error (Wire.Message.encode (Window_command (7L, id, command)))
           : bool)]);
  [%expect
    {|
    true
    true
    true
    true
    |}]
;;

let%expect_test "lifecycle events decode the independent native fixture" =
  (* Three events: close generation 1/slot 0, quit and reopen. *)
  let events = Wire.Event.decode "\003\031\000\001\032\033" |> Or_error.ok_exn in
  print_s [%sexp (events : Wire.Event.t list)];
  [%expect
    {| ((Close_requested ((slot 0) (generation 1))) Quit_requested Reopen_requested) |}]
;;

let%expect_test "document observations preserve raw path bytes and unsupported errors" =
  let fixture =
    "\002\035\007\000\001\000\001t"
    ^ String.make 48 '\000'
    ^ "\000\000\000\001\001\006/tmp/\255\001"
    ^ "\000\001\001\001\001\001\000"
    ^ "\035\008\000\001\001\005"
  in
  let events = Wire.Event.decode fixture |> Or_error.ok_exn in
  (match events with
   | [ Window_response (_, _, Observed snapshot)
     ; Window_response (_, _, Failed Unsupported)
     ] ->
     assert (Wire.Window.Snapshot.valid snapshot);
     let document = Option.value_exn snapshot.document in
     assert (Option.equal String.equal document.path (Some "/tmp/\255"));
     assert document.edited
   | _ -> failwith "unexpected document observation fixture");
  [%expect {| |}]
;;

let%expect_test "client decorations and supported controls decode independent bytes" =
  let prefix = "\001\034\000\001\001t" ^ String.make 52 '\000' in
  let suffix = "\001\001\000\001\000\000\001\000\001\001\000" in
  let expected : Window_wire.Presentation.t =
    { decorations = Client { top = true; right = false; bottom = true; left = false }
    ; controls =
        { fullscreen = false; maximize = true; minimize = false; window_menu = true }
    ; resizable = true
    }
  in
  (match Wire.Event.decode (prefix ^ suffix) |> Or_error.ok_exn with
   | [ Window_changed (_, snapshot) ] ->
     assert (Window_wire.Presentation.equal snapshot.presentation expected)
   | _ -> assert false);
  List.iter (List.range 0 10) ~f:(fun offset ->
    let bad = String.mapi suffix ~f:(fun i c -> if i = offset then '\002' else c) in
    assert (Result.is_error (Wire.Event.decode (prefix ^ bad))));
  (* Maximize must not be advertised against the native resizability policy. *)
  let bad = String.mapi suffix ~f:(fun i c -> if i = 9 then '\000' else c) in
  assert (Result.is_error (Wire.Event.decode (prefix ^ bad)));
  print_endline "client tiling, capability flags and policy are checked independently";
  [%expect {| client tiling, capability flags and policy are checked independently |}]
;;

let%expect_test "focused input queries contain only typed mounted identity" =
  let encoded =
    Wire.Message.encode (Window_command (7L, id, Focused_input)) |> Or_error.ok_exn
  in
  print_endline (hex encoded);
  let kinds =
    Window_wire.Input_kind.
      [ Input; Textarea; Combobox; Otp; Number; Color; Command_palette ]
  in
  List.iteri kinds ~f:(fun tag kind ->
    let bytes =
      "\001\035\007\000\001\002\001\003\004" ^ String.of_char (Char.of_int_exn tag)
    in
    (match Wire.Event.decode bytes |> Or_error.ok_exn with
     | [ Window_response (7L, window, Focused_input (Some input)) ] ->
       assert (Window_id.equal window id);
       assert (Window_wire.Input_kind.equal input.kind kind);
       assert (
         Node_id.equal
           input.node
           (Node_id.create ~slot:3L ~generation:4L |> Or_error.ok_exn))
     | _ -> assert false);
    List.iter
      (List.range 0 (String.length bytes))
      ~f:(fun length ->
        assert (Result.is_error (Wire.Event.decode (String.prefix bytes length)))));
  List.iter
    [ "\001\035\007\000\001\002\001\003\004\007"
    ; "\001\035\007\000\001\002\001\003\000\000"
    ]
    ~f:(fun bytes -> assert (Result.is_error (Wire.Event.decode bytes)));
  (match Wire.Event.decode "\001\035\007\000\001\002\000" |> Or_error.ok_exn with
   | [ Window_response (_, _, Focused_input None) ] -> ()
   | _ -> assert false);
  [%expect {| 0b07000109 |}]
;;
