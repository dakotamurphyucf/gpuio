open Core
module N = Gpuio.Notification
module W = Gpuio_protocol.Notification_wire

let%expect_test "notification and chart support require the paired M6 backend" =
  let module Bridge = Gpuio_protocol.Wire in
  List.iter [ 4398046511104L; 8796093022208L ] ~f:(fun bit ->
    assert (Int64.equal (Int64.bit_and Bridge.capabilities bit) bit));
  assert (Int64.equal Bridge.capabilities 281474976710655L);
  let hello =
    Bridge.Message.encode (Hello (Bridge.version, Bridge.capabilities)) |> Or_error.ok_exn
  in
  String.iter hello ~f:(fun byte -> printf "%02x" (Char.to_int byte));
  [%expect {| 0001fcffffffffffff0000 |}]
;;

let encode writer value = Bin_prot.Utils.bin_dump writer value |> Bigstring.to_string

let content : W.Content.t =
  { title = "Ready"
  ; body = "OK"
  ; actions = [ { id = "open"; label = "Open" } ]
  ; sound = Silent
  }
;;

let receipt : W.Receipt.t = { id = 7L; tag = "build" }

let%expect_test "notification requests match independent Rust bytes" =
  let body = "\005Ready\002OK\001\004open\004Open\000" in
  let cases : (W.Request.t * string) list =
    [ Capabilities, "\000"
    ; Authorization, "\001"
    ; Request_authorization, "\002"
    ; Post ("build", content), "\003\005build" ^ body
    ; Replace (receipt, content), "\004\007\005build" ^ body
    ; Dismiss receipt, "\005\007\005build"
    ; Take_events, "\006"
    ; Close, "\007"
    ]
  in
  List.iter cases ~f:(fun (value, bytes) ->
    assert (W.Request.valid value);
    assert (String.equal (encode W.Request.bin_writer_t value) bytes));
  [%expect {| |}]
;;

let%expect_test "notification responses and all event cases match Rust" =
  let cases : (W.Response.t * string) list =
    [ ( Capabilities
          { body = true
          ; actions = true
          ; activation = true
          ; replacement = true
          ; dismissal = true
          ; permission_request = false
          ; sound = true
          }
      , "\000\001\001\001\001\001\000\001" )
    ; Authorization Not_required, "\001\004"
    ; Posted receipt, "\002\007\005build"
    ; Replaced, "\003"
    ; Dismiss_requested, "\004"
    ; ( Events
          [ Activated receipt
          ; Action (receipt, "open")
          ; Closed (receipt, User)
          ; Failed Unavailable
          ]
      , "\005\004\000\007\005build\001\007\005build\004open\002\007\005build\001\003\003"
      )
    ; Closed, "\006"
    ; Failed Native_failure, "\007\008"
    ]
  in
  List.iter cases ~f:(fun (value, bytes) ->
    assert (W.Response.valid value);
    assert (String.equal (encode W.Response.bin_writer_t value) bytes);
    let pos_ref = ref 0 in
    let decoded = W.Response.bin_read_t (Bigstring.of_string bytes) ~pos_ref in
    assert (!pos_ref = String.length bytes && W.Response.equal value decoded));
  [%expect {| |}]
;;

let%expect_test "public content validates UTF8, byte budgets and unique action identities"
  =
  let id = N.Action_id.of_string "open" |> Or_error.ok_exn in
  let action = N.Action.create id ~label:"Open 🦀" |> Or_error.ok_exn in
  let value =
    N.create ~title:"日本語" ~body:"a\nb\rc\td" ~actions:[ action ] () |> Or_error.ok_exn
  in
  assert (N.Sound.equal (N.sound value) Silent);
  assert (N.equal value (N.Expert.of_wire (N.Expert.to_wire value) |> Or_error.ok_exn));
  List.iter
    [ ""; " "; "\255"; "x\000y"; "x\ny"; String.make 257 'x' ]
    ~f:(fun title -> assert (Result.is_error (N.create ~title ())));
  List.iter
    [ ""; " "; "a/b"; "日本語"; String.make 65 'x' ]
    ~f:(fun id -> assert (Result.is_error (N.Action_id.of_string id)));
  assert (Result.is_error (N.create ~title:"ok" ~actions:[ action; action ] ()));
  assert (Result.is_error (N.create ~title:"ok" ~body:(String.make 8193 'x') ()));
  assert (
    Result.is_ok (N.create ~title:(String.make 256 'x') ~body:(String.make 8192 'x') ()));
  assert (
    Result.is_error
      (N.Expert.of_wire
         { content with
           actions =
             List.init 5 ~f:(fun i -> { W.Action.id = Int.to_string i; label = "Open" })
         }));
  List.iter
    [ ""; " "; "x\127"; "\255"; String.make 129 'x' ]
    ~f:(fun tag -> assert (Result.is_error (N.Tag.of_string tag)));
  assert (Result.is_ok (N.Tag.of_string (String.make 128 'x')));
  [%expect {| |}]
;;

let%expect_test "wire conversion cannot manufacture malformed public identities" =
  List.iter
    [ { receipt with id = 0L }; { receipt with id = -1L }; { receipt with tag = "" } ]
    ~f:(fun value -> assert (Result.is_error (N.Expert.receipt_of_wire value)));
  assert (Result.is_error (N.Expert.event_of_wire (Action (receipt, "bad/id"))));
  let old = N.Expert.receipt_of_wire receipt |> Or_error.ok_exn in
  let next = N.Expert.receipt_of_wire { receipt with id = 8L } |> Or_error.ok_exn in
  assert (N.Tag.equal (N.Receipt.tag old) (N.Receipt.tag next));
  assert (not (N.Receipt.equal old next));
  let events = List.init W.max_events ~f:(fun _ -> W.Event.Failed Unavailable) in
  assert (W.Response.valid (Events events));
  assert (not (W.Response.valid (Events (Failed Unavailable :: events))));
  [%expect {| |}]
;;

let%expect_test "application envelopes distinguish notification availability" =
  let module Bridge = Gpuio_protocol.Wire in
  assert (
    String.equal
      (Bridge.Message.encode (Notification (7L, Capabilities)) |> Or_error.ok_exn)
      "\020\007\000");
  assert (Result.is_error (Bridge.Message.encode (Notification (0L, Capabilities))));
  print_s
    [%sexp
      (Bridge.Event.decode "\002\059\007\003\060" |> Or_error.ok_exn
       : Bridge.Event.t list)];
  assert (Result.is_error (Bridge.Event.decode "\001\059\000\003"));
  [%expect {| ((Notification_response 7 Replaced) Notification_pending) |}]
;;
