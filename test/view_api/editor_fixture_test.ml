open Core
open Gpuio_protocol
module W = Wire

let window = Window_id.create ~slot:3L ~generation:2L |> Or_error.ok_exn
let node = Node_id.create ~slot:4L ~generation:3L |> Or_error.ok_exn
let handler = Handler_id.create ~slot:5L ~generation:4L |> Or_error.ok_exn

let requests =
  let selection : W.Editor.Selection.t = { anchor = 5L; head = 2L } in
  let commands : W.Editor.Command.t list =
    [ Replace ("é界", Start, Record, None)
    ; Replace ("é界", End, Reset, Some 128L)
    ; Replace ("é界", Preserve, Record, Some 127L)
    ; Replace ("é界", Select selection, Reset, None)
    ; Select selection
    ; Focus
    ; Undo
    ; Redo
    ]
  in
  [ W.Message.Hello (W.version, W.capabilities)
  ; Apply
      { window
      ; base = 127L
      ; revision = 128L
      ; operations =
          [ Create (node, Input, "é界", Some handler)
          ; Create
              ( Node_id.create ~slot:6L ~generation:1L |> Or_error.ok_exn
              , Textarea
              , ""
              , None )
          ; Set_editor
              ( node
              , { label = "Nom"
                ; placeholder = "écrire"
                ; read_only = true
                ; disabled = false
                ; submit_on_enter = true
                ; auto_focus = false
                ; min_rows = 1L
                ; max_rows = 1L
                } )
          ]
      }
  ]
  @ List.mapi commands ~f:(fun i command ->
    W.Message.Editor_command (Int64.of_int (128 + i), window, node, command))
;;

let events =
  let snapshot : W.Editor.Snapshot.t =
    { revision = 128L
    ; text = "é界"
    ; selection = { anchor = 5L; head = 2L }
    ; composition = None
    ; focused = true
    }
  in
  [ W.Event.Editor_event
      ( window
      , node
      , handler
      , 128L
      , Changed
      , { snapshot with composition = Some { anchor = 0L; head = 2L } } )
  ; Editor_event (window, node, handler, 128L, Submitted, snapshot)
  ; Editor_result (128L, window, node, Applied snapshot)
  ]
  @ List.mapi
      W.Editor.Error.
        [ Not_mounted
        ; Closed
        ; Stale_editor
        ; Stale_revision
        ; Composing
        ; Invalid_selection
        ; Limit_exceeded
        ; Busy
        ; Native_failure
        ; Invalid_text
        ]
      ~f:(fun i error ->
        W.Event.Editor_result (Int64.of_int (129 + i), window, node, Failed error))
;;

let%expect_test "editor wire tags independently agree in OCaml and Rust" =
  Eio_main.run (fun env ->
    let load name =
      let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip in
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    let request_bytes = load "editor-v1-requests.hex"
    and event_bytes = load "editor-v1-events.hex" in
    let actual =
      Bin_prot.Utils.bin_dump [%bin_writer: W.Message.t list] requests
      |> Bigstring.to_string
    in
    assert (String.equal request_bytes actual);
    let actual =
      Bin_prot.Utils.bin_dump [%bin_writer: W.Event.t list] events |> Bigstring.to_string
    in
    assert (String.equal event_bytes actual);
    assert (List.equal W.Event.equal events (W.Event.decode event_bytes |> Or_error.ok_exn));
    for length = 0 to String.length event_bytes - 1 do
      assert (Result.is_error (W.Event.decode (String.prefix event_bytes length)))
    done;
    assert (Result.is_error (W.Event.decode (event_bytes ^ "\000")));
    print_endline "EDITOR_CODEC_PASS");
  [%expect {| EDITOR_CODEC_PASS |}]
;;
