open Core
module O = Gpuio.Otp_input
module W = Gpuio_protocol.Otp_wire

let policy ?alphabet length = O.Policy.create ~length ?alphabet () |> Or_error.ok_exn
let show result = print_s [%sexp (result : (O.Value.t, O.Input_error.t) Result.t)]

let wire_snapshot =
  { W.Snapshot.revision = 7L
  ; policy = { length = 6; alphabet = Digits }
  ; value = "12"
  ; draft = "12３"
  ; selection = { anchor = 5L; head = 2L }
  ; composition = Some { anchor = 2L; head = 5L }
  ; focused = true
  ; can_undo = false
  ; can_redo = true
  }
;;

let full_snapshot =
  { wire_snapshot with
    revision = 9L
  ; value = "123456"
  ; draft = "123456"
  ; selection = { anchor = 6L; head = 6L }
  ; composition = None
  ; can_undo = true
  ; can_redo = false
  }
;;

let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let fixture writer value expected =
  let hex =
    Bin_prot.Utils.bin_dump writer value
    |> Bigstring.to_string
    |> String.to_list
    |> List.map ~f:(fun ch -> sprintf "%02x" (Char.to_int ch))
    |> String.concat
  in
  assert (String.equal hex expected)
;;

let%expect_test "OTP typed configuration, snapshots and independent wire layouts" =
  let config =
    O.Config.create ~policy:(policy 6) ~label:"Code" ~masked:true ~auto_focus:true ()
    |> Or_error.ok_exn
  in
  assert (
    O.Config.is_masked config
    && (not (O.Config.is_disabled config))
    && not (O.Config.is_read_only config));
  assert (O.Policy.equal (O.Config.policy config) (policy 6));
  fixture W.Config.bin_writer_t (O.Expert.config_to_wire config) "060004436f646501000001";
  fixture
    W.Event.bin_writer_t
    (Observed wire_snapshot)
    "00070600023132053132efbc930502010205010001";
  fixture
    W.Event.bin_writer_t
    (Complete full_snapshot)
    "020906000631323334353606313233343536060600010100";
  fixture W.Response.bin_writer_t (Failed Stale_revision) "0103";
  fixture
    W.Event.bin_writer_t
    (Rejected (Unexpected_character { byte_offset = 4095 }, full_snapshot))
    "0303feff0f0906000631323334353606313233343536060600010100";
  let value =
    O.Value.of_string (policy 6) "１２34"
    |> Result.map_error ~f:(fun e -> Error.create_s (O.Input_error.sexp_of_t e))
    |> Or_error.ok_exn
  in
  let replace =
    O.Command.Replace
      { value
      ; selection =
          Select (Gpuio.Text_input.Selection.create ~anchor:4 ~head:1 |> Or_error.ok_exn)
      ; undo = Reset
      ; if_revision = Some (O.Revision.of_int64 7L |> Or_error.ok_exn)
      }
  in
  fixture
    W.Command.bin_writer_t
    (O.Expert.command_to_wire replace)
    "000431323334030401010107";
  fixture
    W.Command.bin_writer_t
    (O.Expert.command_to_wire (Clear { undo = Record; if_revision = None }))
    "010000";
  let snapshot =
    O.Expert.snapshot_of_wire ~window ~node wire_snapshot |> Or_error.ok_exn
  in
  assert (Gpuio_protocol.Window_id.equal (O.Expert.window snapshot) window);
  assert (Gpuio_protocol.Node_id.equal (O.Expert.node snapshot) node);
  assert (O.Policy.equal (O.Snapshot.policy snapshot) (policy 6));
  assert (String.equal (O.Value.to_string (O.Snapshot.value snapshot)) "12");
  assert (String.equal (O.Snapshot.draft snapshot) "12３");
  assert (Gpuio.Text_input.Selection.anchor (O.Snapshot.selection snapshot) = 5);
  assert (Gpuio.Text_input.Selection.head (O.Snapshot.selection snapshot) = 2);
  assert (Option.is_some (O.Snapshot.composition snapshot));
  assert (
    O.Snapshot.focused snapshot
    && (not (O.Snapshot.can_undo snapshot))
    && O.Snapshot.can_redo snapshot);
  assert (not (O.Snapshot.is_complete snapshot));
  assert (Int64.equal (O.Revision.to_int64 (O.Snapshot.revision snapshot)) 7L);
  let full = O.Expert.snapshot_of_wire ~window ~node full_snapshot |> Or_error.ok_exn in
  assert (O.Snapshot.is_complete full);
  print_endline
    "configuration, composing/completed snapshots, replacement/clear and failure \
     fixtures match";
  [%expect
    {| configuration, composing/completed snapshots, replacement/clear and failure fixtures match |}]
;;

let%expect_test "OTP untrusted snapshots, revisions and configuration are validated" =
  List.iter
    [ ""; " \t\011\012"; "a\n"; "a\r"; "a\000"; "\255"; String.make 4097 'a' ]
    ~f:(fun label ->
      assert (Result.is_error (O.Config.create ~policy:(policy 6) ~label ())));
  List.iter
    [ "Code"; "-"; String.make 4096 'a' ]
    ~f:(fun label ->
      O.Config.create ~policy:(policy 6) ~label () |> Or_error.ok_exn |> ignore);
  assert (Result.is_error (O.Revision.of_int64 (-1L)));
  O.Revision.of_int64 Int64.max_value |> Or_error.ok_exn |> ignore;
  List.iter
    [ { wire_snapshot with revision = -1L }
    ; { wire_snapshot with policy = { length = 0; alphabet = Digits } }
    ; { wire_snapshot with value = "AB" }
    ; { wire_snapshot with value = "1234567" }
    ; { wire_snapshot with draft = "\255" }
    ; { wire_snapshot with draft = "\000" }
    ; { wire_snapshot with draft = String.make 4097 'a' }
    ; { wire_snapshot with composition = None }
    ; { wire_snapshot with composition = Some { anchor = 2L; head = 2L } }
    ; { wire_snapshot with composition = Some { anchor = 5L; head = 2L } }
    ; { wire_snapshot with composition = Some { anchor = 3L; head = 5L } }
    ; { wire_snapshot with selection = { anchor = -1L; head = 2L } }
    ; { wire_snapshot with selection = { anchor = 6L; head = 2L } }
    ; { wire_snapshot with selection = { anchor = 3L; head = 2L } }
    ]
    ~f:(fun s ->
      assert (not (W.Snapshot.valid s));
      assert (Result.is_error (O.Expert.snapshot_of_wire ~window ~node s)));
  List.iter
    [ W.Event.Complete wire_snapshot
    ; Complete
        { full_snapshot with
          value = "12"
        ; draft = "12"
        ; selection = { anchor = 2L; head = 2L }
        }
    ; Changed { full_snapshot with revision = 0L }
    ; Complete { full_snapshot with revision = 0L }
    ; Rejected (Too_long, wire_snapshot)
    ; Rejected (Unexpected_character { byte_offset = -1 }, full_snapshot)
    ; Rejected (Unexpected_character { byte_offset = 4096 }, full_snapshot)
    ]
    ~f:(fun event ->
      assert (not (W.Event.valid event));
      assert (Result.is_error (O.Expert.event_of_wire ~window ~node event)));
  List.iter
    [ W.Event.Observed { full_snapshot with revision = 0L }
    ; Changed wire_snapshot
    ; Complete full_snapshot
    ; Rejected (Unexpected_character { byte_offset = 4095 }, full_snapshot)
    ]
    ~f:(fun event ->
      assert (W.Event.valid event);
      O.Expert.event_of_wire ~window ~node event |> Or_error.ok_exn |> ignore);
  print_endline
    "invalid labels, policies, revisions, Unicode boundaries and impossible event states \
     reject";
  [%expect
    {| invalid labels, policies, revisions, Unicode boundaries and impossible event states reject |}]
;;

let%expect_test "OTP command tags and validation" =
  List.iteri
    [ W.Error.Not_mounted
    ; Closed
    ; Stale_input
    ; Stale_revision
    ; Composing
    ; Invalid_selection
    ; Limit_exceeded
    ; Busy
    ; Native_failure
    ; Invalid_value
    ; Focus_blocked
    ; Disabled
    ; Read_only
    ; Invalid_config
    ]
    ~f:(fun tag error ->
      fixture W.Response.bin_writer_t (Failed error) (sprintf "01%02x" tag));
  let commands =
    [ ( O.Command.Select
          (Gpuio.Text_input.Selection.create ~anchor:2 ~head:0 |> Or_error.ok_exn)
      , "020200" )
    ; Focus, "03"
    ; Undo, "04"
    ; Redo, "05"
    ; Cancel_composition, "06"
    ; Read_snapshot, "07"
    ]
  in
  List.iter commands ~f:(fun (command, expected) ->
    let command = O.Expert.command_to_wire command in
    assert (W.Command.valid command);
    fixture W.Command.bin_writer_t command expected);
  List.iter
    [ W.Command.Replace
        { value = "１２"; selection = End; undo = Record; if_revision = None }
    ; Replace { value = "1-2"; selection = End; undo = Record; if_revision = None }
    ; Replace
        { value = String.make 33 'a'; selection = End; undo = Record; if_revision = None }
    ; Replace
        { value = "12"
        ; selection = Select { anchor = 3L; head = 0L }
        ; undo = Reset
        ; if_revision = None
        }
    ; Clear { undo = Reset; if_revision = Some (-1L) }
    ; Select { anchor = 4097L; head = 0L }
    ]
    ~f:(fun command -> assert (not (W.Command.valid command)));
  assert (
    W.Command.valid
      (Replace
         { value = "aZ09"; selection = Preserve; undo = Record; if_revision = Some 0L }));
  print_endline
    "all command tags, canonical values, exact selection and revision guards validated";
  [%expect
    {| all command tags, canonical values, exact selection and revision guards validated |}]
;;

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
