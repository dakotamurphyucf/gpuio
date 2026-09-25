open Core
open Gpuio_protocol

let%expect_test "extension request and event agree with independently encoded Rust" =
  let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let handler = Handler_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let config : Extension_wire.Config.t =
    { schema = { name = "test.counter"; version = 1L; fingerprint = String.make 64 'a' }
    ; generation = 1L
    ; label = "x"
    ; disabled = false
    ; properties = "\000\255\128"
    ; command = Some { sequence = 2L; payload = "\009" }
    }
  in
  let message =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Extension, "", Some handler); Set_extension (node, config) ]
      }
  in
  let bytes = Wire.Message.encode message |> Or_error.ok_exn in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  assert (
    String.equal
      hex
      ("0300010001020000011e000100012300010c746573742e636f756e7465720140"
       ^ String.concat (List.init 64 ~f:(fun _ -> "61"))
       ^ "010178000300ff8001020109"));
  let event_bytes = "\001\038\000\001\000\001\000\001\001\001\000\003\000\255\128" in
  let events = Wire.Event.decode event_bytes |> Or_error.ok_exn in
  assert (
    List.equal
      Wire.Event.equal
      events
      [ Extension_event (window, node, handler, 1L, 1L, Data "\000\255\128") ]);
  print_endline "paired request/event fixtures and binary payloads match";
  [%expect {| paired request/event fixtures and binary payloads match |}]
;;
