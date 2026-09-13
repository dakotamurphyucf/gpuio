open Core
open Gpuio_protocol

let window = Window_id.create ~slot:3L ~generation:2L |> Or_error.ok_exn
let node = Node_id.create ~slot:4L ~generation:3L |> Or_error.ok_exn
let handler = Handler_id.create ~slot:5L ~generation:4L |> Or_error.ok_exn

let request =
  Wire.Message.Apply
    { window
    ; base = 127L
    ; revision = 128L
    ; operations =
        [ Create (node, Radio_group, "", Some handler)
        ; Create
            ( Node_id.create ~slot:5L ~generation:3L |> Or_error.ok_exn
            , Select
            , ""
            , Some (Handler_id.create ~slot:6L ~generation:4L |> Or_error.ok_exn) )
        ; Set_choice
            ( node
            , { label = "Mode"
              ; items =
                  [ { id = "fast"; label = "Fast"; disabled = false }
                  ; { id = "深い"; label = "Deep"; disabled = true }
                  ]
              ; selected = Some "深い"
              ; disabled = false
              } )
        ]
    }
;;

let events = [ Wire.Event.Choice (window, node, handler, 128L, "深い") ]

let%expect_test "stable choices encode and decode independently of their native widget" =
  Eio_main.run (fun env ->
    let load name =
      let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip in
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    let request_bytes = load "choice-v1-request.hex"
    and event_bytes = load "choice-v1-events.hex" in
    assert (String.equal request_bytes (Wire.Message.encode request |> Or_error.ok_exn));
    assert (
      String.equal
        event_bytes
        (Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events
         |> Bigstring.to_string));
    assert (
      List.equal Wire.Event.equal events (Wire.Event.decode event_bytes |> Or_error.ok_exn));
    for length = 0 to String.length event_bytes - 1 do
      assert (Result.is_error (Wire.Event.decode (String.prefix event_bytes length)))
    done;
    print_endline "CHOICE_CODEC_PASS");
  [%expect {| CHOICE_CODEC_PASS |}]
;;
