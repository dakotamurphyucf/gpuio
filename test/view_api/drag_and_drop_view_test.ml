open Core
open Gpuio
open Gpuio_protocol
module Wire = Gpuio_protocol.Wire
module Drag = Gpuio.Drag_and_drop

let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
let handler = Handler_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let source ?(disabled = false) text =
  Drag.Source.create
    ~label:"Source"
    ~payload:(Drag.Payload.text text |> Or_error.ok_exn)
    ~disabled
    ()
  |> Or_error.ok_exn
;;

let target_sample phase : Wire.Drag_and_drop.Target_sample.t =
  { gesture = 7L
  ; phase
  ; window_x = 120.
  ; window_y = 80.
  ; local_x = -5.
  ; local_y = 30.
  ; modifiers =
      { shift = true; control = false; alt = false; command = false; function_ = false }
  }
;;

let%expect_test
    "source updates preserve ownership and route native snapshots to the current callback"
  =
  let r = Reconciler.create window in
  let prepare config prefix =
    Reconciler.prepare
      r
      ~theme:Theme.default
      (Some
         (View.drag_source
            ~config
            ~on_event:(fun e ->
              prefix ^ Sexp.to_string (Drag.Source_phase.sexp_of_t e.phase))
            []))
    |> Or_error.ok_exn
  in
  Reconciler.accept r (prepare (source "first") "old:") |> Or_error.ok_exn;
  let changed = prepare (source ~disabled:true "second") "new:" in
  (match Reconciler.message changed with
   | Some (Apply { operations = [ Set_drag_source (id, config) ]; _ }) ->
     assert (Node_id.equal id node && config.disabled)
   | _ -> assert false);
  Reconciler.accept r changed |> Or_error.ok_exn;
  let event phase =
    Wire.Event.Drag_source_event (window, node, handler, 1L, { gesture = 7L; phase })
  in
  assert (
    Option.equal
      String.equal
      (Reconciler.dispatch r (event (Ended (Cancelled Disabled))))
      (Some "new:(Ended(Cancelled Disabled))"));
  let removed = Reconciler.prepare r ~theme:Theme.default None |> Or_error.ok_exn in
  Reconciler.accept r removed |> Or_error.ok_exn;
  assert (Option.is_none (Reconciler.dispatch r (event (Started (Text "first")))));
  [%expect {| |}]
;;

let%expect_test
    "target routes observed drops but rejects malformed or stale native samples"
  =
  let config =
    Drag.Target.create ~label:"Target" ~accept:[ Text ] () |> Or_error.ok_exn
  in
  let r = Reconciler.create window in
  let update =
    Reconciler.prepare
      r
      ~theme:Theme.default
      (Some
         (View.drop_target
            ~config
            ~on_event:(fun e -> Sexp.to_string (Drag.Target_phase.sexp_of_t e.phase))
            []))
    |> Or_error.ok_exn
  in
  Reconciler.accept r update |> Or_error.ok_exn;
  let event sample = Wire.Event.Drop_target_event (window, node, handler, 1L, sample) in
  let sample = target_sample (Dropped (Text "snapshot")) in
  assert (
    Option.equal
      String.equal
      (Reconciler.dispatch r (event sample))
      (Some "(Dropped(Text snapshot))"));
  List.iter
    [ { sample with gesture = 0L }
    ; { sample with local_x = Float.nan }
    ; { sample with phase = Dropped (Text "\255") }
    ; { sample with
        phase =
          Entered { format = Files; data_bytes = 1L; file_count = 0L; origin = Desktop }
      }
    ]
    ~f:(fun sample -> assert (Option.is_none (Reconciler.dispatch r (event sample))));
  let wrong_handler = Handler_id.create ~slot:0L ~generation:2L |> Or_error.ok_exn in
  assert (
    Option.is_none
      (Reconciler.dispatch
         r
         (Drop_target_event (window, node, wrong_handler, 1L, sample))));
  assert (
    Option.is_none
      (Reconciler.dispatch r (Drop_target_event (window, node, handler, 2L, sample))));
  [%expect {| |}]
;;

let%expect_test "event decoding bounds source snapshots and drop payloads" =
  let bytes sample =
    Bin_prot.Utils.bin_dump
      [%bin_writer: Wire.Event.t list]
      [ Wire.Event.Drop_target_event (window, node, handler, 1L, sample) ]
    |> Bigstring.to_string
  in
  let good = target_sample (Dropped (Custom { kind = "x/v1"; data = "\000\255" })) in
  assert (Result.is_ok (Wire.Event.decode (bytes good)));
  List.iter
    [ { good with phase = Dropped (Files []) }
    ; { good with
        phase = Dropped (Custom { kind = "x"; data = String.make 262_145 'a' })
      }
    ; { good with
        phase =
          Entered
            { format = Text; data_bytes = 262_145L; file_count = 0L; origin = Internal }
      }
    ]
    ~f:(fun sample -> assert (Result.is_error (Wire.Event.decode (bytes sample))));
  let encode phase =
    Bin_prot.Utils.bin_dump
      [%bin_writer: Wire.Event.t list]
      [ Wire.Event.Drag_source_event (window, node, handler, 1L, { gesture = 1L; phase })
      ]
    |> Bigstring.to_string
  in
  assert (Result.is_error (Wire.Event.decode (encode (Started (Text "\255")))));
  assert (Result.is_ok (Wire.Event.decode (encode (Ended Unconfirmed))));
  [%expect {| |}]
;;

let%expect_test "independent Rust view-operation and lifecycle-event fixtures agree" =
  let target_node = Node_id.create ~slot:1L ~generation:1L |> Or_error.ok_exn in
  let target_handler = Handler_id.create ~slot:1L ~generation:1L |> Or_error.ok_exn in
  let config =
    Drag.Source.create ~label:"S" ~payload:(Drag.Payload.text "hi" |> Or_error.ok_exn) ()
    |> Or_error.ok_exn
  in
  let target = Drag.Target.create ~label:"T" ~accept:[ Text ] () |> Or_error.ok_exn in
  let request =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Drag_source, "", Some handler)
          ; Set_drag_source (node, Drag.Expert.source_to_wire config)
          ; Create (target_node, Drop_target, "", Some target_handler)
          ; Set_drop_target (target_node, Drag.Expert.target_to_wire target)
          ; Splice (node, 0L, 0L, [ target_node ])
          ; Set_root (Some node)
          ]
      }
  in
  let source_phases =
    [ Wire.Drag_and_drop.Source_phase.Started (Text "hi")
    ; Desktop_offered
    ; Desktop_unavailable
    ; Ended Internal_drop
    ; Ended Unconfirmed
    ]
    @ List.map
        [ Wire.Drag_and_drop.Cancel_reason.Escape
        ; Hidden
        ; Blocked
        ; Disabled
        ; Removed
        ; Reconfigured
        ; Window_closed
        ; Window_inactive
        ]
        ~f:(fun reason -> Wire.Drag_and_drop.Source_phase.Ended (Cancelled reason))
  in
  let target_phases =
    [ Wire.Drag_and_drop.Target_phase.Entered
        { format = Text; data_bytes = 2L; file_count = 0L; origin = Internal }
    ; Moved
    ; Left
    ; Dropped (Text "hi")
    ; Rejected Invalid_data
    ; Rejected Limit_exceeded
    ]
  in
  let events =
    List.map source_phases ~f:(fun phase ->
      Wire.Event.Drag_source_event (window, node, handler, 1L, { gesture = 7L; phase }))
    @ List.map target_phases ~f:(fun phase ->
      Wire.Event.Drop_target_event
        (window, target_node, target_handler, 1L, target_sample phase))
  in
  Eio_main.run (fun env ->
    let read name = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip in
    let unhex text =
      String.init
        (String.length text / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub text ~pos:(i * 2) ~len:2)))
    in
    assert (
      String.equal
        (Wire.Message.encode request |> Or_error.ok_exn)
        (read "drag-drop-v1-request.hex" |> unhex));
    let bytes = read "drag-drop-v1-events.hex" |> unhex in
    assert (
      String.equal
        bytes
        (Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events
         |> Bigstring.to_string));
    assert (List.equal Wire.Event.equal events (Wire.Event.decode bytes |> Or_error.ok_exn));
    for length = 0 to String.length bytes - 1 do
      assert (Result.is_error (Wire.Event.decode (String.prefix bytes length)))
    done);
  [%expect {| |}]
;;
