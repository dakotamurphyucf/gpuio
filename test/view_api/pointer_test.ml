open Core
open Gpuio
open Gpuio_protocol
module Wire = Gpuio_protocol.Wire

let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
let handler = Handler_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let config ?disabled () =
  Pointer.Config.create ~label:"Resize" ?disabled () |> Or_error.ok_exn
;;

let sample phase : Wire.Pointer.Sample.t =
  { gesture = 7L
  ; phase
  ; button = Left
  ; window_x = 120.
  ; window_y = 80.
  ; local_x = -5.
  ; local_y = 30.
  ; modifiers =
      { shift = true; control = false; alt = false; command = false; function_ = false }
  }
;;

let%expect_test "pointer config and finite samples reject invalid native data" =
  List.iter
    [ ""; " "; "\000"; "\255"; String.make 4097 'x' ]
    ~f:(fun label -> assert (Result.is_error (Pointer.Config.create ~label ())));
  let good = sample Moved in
  List.iter
    [ { good with gesture = 0L }
    ; { good with local_x = Float.nan }
    ; { good with window_y = Float.infinity }
    ]
    ~f:(fun sample -> assert (Result.is_error (Pointer.Expert.event_of_wire sample)));
  let event = Pointer.Expert.event_of_wire good |> Or_error.ok_exn in
  assert (Float.equal event.local_position.x (-5.));
  assert event.modifiers.shift;
  [%expect {| |}]
;;

let%expect_test "disabled metadata keeps terminal cancellation and current callback" =
  let r = Reconciler.create window in
  let prepare disabled prefix =
    Reconciler.prepare
      r
      ~theme:Theme.default
      (Some
         (View.pointer_area
            ~config:(config ~disabled ())
            ~on_event:(fun event ->
              prefix ^ Sexp.to_string (Pointer.Phase.sexp_of_t event.phase))
            []))
    |> Or_error.ok_exn
  in
  let first = prepare false "old:" in
  Reconciler.accept r first |> Or_error.ok_exn;
  let changed = prepare true "new:" in
  (match Reconciler.message changed with
   | Some (Apply { operations = [ Set_pointer (id, config) ]; _ }) ->
     assert (Node_id.equal id node && config.disabled)
   | _ -> assert false);
  Reconciler.accept r changed |> Or_error.ok_exn;
  let event =
    Wire.Event.Pointer_event (window, node, handler, 1L, sample (Cancelled Disabled))
  in
  assert (
    Option.equal
      String.equal
      (Reconciler.dispatch r event)
      (Some "new:(Cancelled Disabled)"));
  let invalid =
    Wire.Event.Pointer_event
      (window, node, handler, 1L, { (sample Moved) with gesture = 0L })
  in
  assert (Option.is_none (Reconciler.dispatch r invalid));
  let removed = Reconciler.prepare r ~theme:Theme.default None |> Or_error.ok_exn in
  Reconciler.accept r removed |> Or_error.ok_exn;
  assert (Option.is_none (Reconciler.dispatch r event));
  [%expect {| |}]
;;

let%expect_test "independent Rust pointer request and event fixtures agree" =
  let request =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Pointer_area, "", Some handler)
          ; Set_pointer (node, Pointer.Expert.to_wire (config ()))
          ; Set_root (Some node)
          ]
      }
  in
  let events =
    List.map
      [ Wire.Pointer.Phase.Started; Moved; Released; Cancelled Hidden ]
      ~f:(fun phase -> Wire.Event.Pointer_event (window, node, handler, 1L, sample phase))
  in
  Eio_main.run (fun env ->
    let fixture name =
      let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip in
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    assert (
      String.equal
        (fixture "pointer-v1-request.hex")
        (Wire.Message.encode request |> Or_error.ok_exn));
    assert (
      List.equal
        Wire.Event.equal
        events
        (Wire.Event.decode (fixture "pointer-v1-events.hex") |> Or_error.ok_exn)));
  [%expect {| |}]
;;
