open Core
open Gpuio
open Gpuio_protocol
module Wire = Gpuio_protocol.Wire

let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
let node slot = Node_id.create ~slot ~generation:1L |> Or_error.ok_exn
let handler = Handler_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let config ?timeout () =
  Toast.Config.create
    ~label:"Saved"
    ~close_label:"Dismiss"
    ~politeness:Assertive
    ?timeout
    ()
  |> Or_error.ok_exn
;;

let item ?(key = Key.of_string_exn "draft") ?timeout callback =
  View.toast
    ~key
    ~config:(config ?timeout ())
    ~on_dismiss:callback
    [ View.text "Draft saved" ]
;;

let stack items = View.toast_stack items |> Or_error.ok_exn

let%expect_test "toast validation bounds keys, labels, visible count and active time" =
  List.iter
    [ Time_ns.Span.zero; Time_ns.Span.of_sec (-1.); Time_ns.Span.of_sec 86401. ]
    ~f:(fun timeout -> assert (Result.is_error (Toast.Timeout.after timeout)));
  assert (Result.is_error (Toast.Config.create ~label:" " ()));
  assert (Result.is_error (Toast.Config.create ~label:"Saved" ~close_label:"\000" ()));
  List.iter [ Float.nan; Float.infinity; 0.; -1. ] ~f:(fun width ->
    assert (Result.is_error (Toast.Stack.create ~width ())));
  List.iter [ 0; 9 ] ~f:(fun max_visible ->
    assert (Result.is_error (Toast.Stack.create ~max_visible ())));
  let duplicate = item Fn.id in
  assert (Result.is_error (View.toast_stack [ duplicate; duplicate ]));
  assert (
    Result.is_error
      (View.toast_stack (List.init 33 ~f:(fun i -> item ~key:(Key.of_int i) Fn.id))));
  [%expect {| |}]
;;

let%expect_test
    "terminal native dismissal survives timeout metadata update but not unmount"
  =
  let reconciler = Reconciler.create window in
  let prepare view =
    Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> Or_error.ok_exn
  in
  let callback prefix reason =
    prefix ^ Sexp.to_string (Toast.Dismissal.sexp_of_t reason)
  in
  let initial = prepare (stack [ item (callback "first:") ]) in
  let toast_node, toast_handler =
    match Reconciler.message initial with
    | Some (Apply { operations; _ }) ->
      List.find_map_exn operations ~f:(function
        | Create (id, Toast, _, Some handler) -> Some (id, handler)
        | _ -> None)
    | _ -> assert false
  in
  Reconciler.accept reconciler initial |> Or_error.ok_exn;
  let changed =
    prepare (stack [ item ~timeout:Toast.Timeout.persistent (callback "current:") ])
  in
  (match Reconciler.message changed with
   | Some (Apply { operations = [ Set_toast (same_node, config) ]; _ }) ->
     assert (Node_id.equal toast_node same_node && Option.is_none config.timeout_ns)
   | _ -> assert false);
  Reconciler.accept reconciler changed |> Or_error.ok_exn;
  let event =
    Wire.Event.Toast_dismissed (window, toast_node, toast_handler, 1L, Timeout)
  in
  assert (
    Option.equal
      String.equal
      (Reconciler.dispatch reconciler event)
      (Some "current:Timeout"));
  let removed = prepare (stack []) in
  Reconciler.accept reconciler removed |> Or_error.ok_exn;
  assert (Option.is_none (Reconciler.dispatch reconciler event));
  [%expect {| |}]
;;

let%expect_test "independent toast request and terminal event bytes" =
  let request : Wire.Message.t =
    Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node 0L, Toast_stack, "", None)
          ; Set_toast_stack (node 0L, Toast.Expert.stack_to_wire Toast.Stack.default)
          ; Create (node 1L, Toast, "", Some handler)
          ; Set_toast (node 1L, Toast.Expert.to_wire (config ()))
          ; Create (node 2L, Text, "Draft saved", None)
          ; Splice (node 1L, 0L, 0L, [ node 2L ])
          ; Splice (node 0L, 0L, 0L, [ node 1L ])
          ; Set_root (Some (node 0L))
          ]
      }
  in
  let events =
    List.map
      [ Wire.Toast_dismissal.Timeout; Close_button; Escape; Overflow ]
      ~f:(fun reason -> Wire.Event.Toast_dismissed (window, node 1L, handler, 1L, reason))
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
        (fixture "toast-v1-request.hex")
        (Wire.Message.encode request |> Or_error.ok_exn));
    assert (
      List.equal
        Wire.Event.equal
        events
        (Wire.Event.decode (fixture "toast-v1-events.hex") |> Or_error.ok_exn)));
  [%expect {| |}]
;;
