open Core
open Gpuio
module O = Otp_input
module W = Gpuio_protocol.Otp_wire
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let policy = O.Policy.create ~length:6 () |> ok
let config = O.Config.create ~policy ~label:"Code" ~masked:true ~auto_focus:true () |> ok

let value text =
  O.Value.of_string policy text
  |> Result.map_error ~f:(fun e -> Error.create_s (O.Input_error.sexp_of_t e))
  |> ok
;;

let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok

let snapshot =
  { W.Snapshot.revision = 7L
  ; policy = O.Expert.policy_to_wire policy
  ; value = "12"
  ; draft = "12３"
  ; selection = { anchor = 5L; head = 2L }
  ; composition = Some { anchor = 2L; head = 5L }
  ; focused = true
  ; can_undo = false
  ; can_redo = true
  }
;;

let full =
  { snapshot with
    value = "123456"
  ; draft = "123456"
  ; selection = { anchor = 6L; head = 6L }
  ; composition = None
  }
;;

let event_bytes events =
  Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events |> Bigstring.to_string
;;

let hex text =
  String.to_list text
  |> List.map ~f:(fun ch -> sprintf "%02x" (Char.to_int ch))
  |> String.concat
;;

let%expect_test "OTP retained tags and bounded envelope validation" =
  let request =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Otp_input, "", Some handler)
          ; Set_otp_input (node, O.Expert.config_to_wire config, "12")
          ; Set_root (Some node)
          ]
      }
  in
  let event = Wire.Event.Otp_input_event (window, node, handler, 1L, Observed snapshot) in
  let bytes = event_bytes [ event ] in
  Eio_main.run (fun env ->
    let fs = Eio.Stdenv.fs env in
    assert (
      String.equal
        (Wire.Message.encode request |> ok |> hex)
        (Eio.Path.load Eio.Path.(fs / "otp-input-request.hex") |> String.strip));
    assert (
      String.equal
        (hex bytes)
        (Eio.Path.load Eio.Path.(fs / "otp-input-events.hex") |> String.strip)));
  assert (List.equal Wire.Event.equal (Wire.Event.decode bytes |> ok) [ event ]);
  for length = 0 to String.length bytes - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix bytes length)))
  done;
  assert (Result.is_error (Wire.Event.decode (bytes ^ "\000")));
  List.iter
    [ -1L, W.Event.Observed snapshot
    ; 1L, Complete snapshot
    ; 1L, Changed { snapshot with revision = 0L }
    ; 1L, Observed { snapshot with selection = { anchor = 3L; head = 2L } }
    ; 1L, Observed { snapshot with draft = String.make 4097 'a' }
    ]
    ~f:(fun (revision, event) ->
      assert (
        Result.is_error
          (Wire.Event.decode
             (event_bytes [ Otp_input_event (window, node, handler, revision, event) ]))));
  print_endline
    "Kind 39, operation 45, event 48; independent bytes, full consumption and semantic \
     validation";
  [%expect
    {| Kind 39, operation 45, event 48; independent bytes, full consumption and semantic validation |}]
;;

let%expect_test "OTP reconciliation retains identity, immutable policy and event fences" =
  let reconciler = Reconciler.create window in
  let controller = Key.of_string_exn "otp" in
  let view ?(controller = controller) ?(config = config) ?(initial = value "12") callback =
    View.otp_input ~controller ~config ~initial ~on_event:callback ()
  in
  let commit view =
    let update = Reconciler.prepare reconciler ~theme:Theme.default view |> ok in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply { operations; _ }) -> operations
    | None -> []
    | Some _ -> assert false
  in
  let identity operations =
    List.find_map_exn operations ~f:(function
      | Wire.Op.Create (node, Otp_input, "", Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let node, handler = identity (commit (Some (view (fun _ -> 1)))) in
  let event
        ?(window = window)
        ?(node = node)
        ?(handler = handler)
        ?(tree_revision = 1L)
        revision
    =
    Wire.Event.Otp_input_event
      (window, node, handler, tree_revision, Observed { snapshot with revision })
  in
  let dispatch = Reconciler.dispatch reconciler in
  assert (Option.equal Int.equal (dispatch (event 0L)) (Some 1));
  assert (List.is_empty (commit (Some (view ~initial:(value "654321") (fun _ -> 2)))));
  assert (Option.is_none (dispatch (event 0L)));
  assert (Option.equal Int.equal (dispatch (event 1L)) (Some 2));
  List.iter
    [ event ~tree_revision:99L 2L
    ; event ~tree_revision:(-1L) 2L
    ; event ~window:(Gpuio_protocol.Window_id.create ~slot:0L ~generation:2L |> ok) 2L
    ; event ~node:(Gpuio_protocol.Node_id.create ~slot:0L ~generation:2L |> ok) 2L
    ; event ~handler:(Gpuio_protocol.Handler_id.create ~slot:0L ~generation:2L |> ok) 2L
    ; Otp_input_event (window, node, handler, 1L, Complete snapshot)
    ; Otp_input_event
        ( window
        , node
        , handler
        , 1L
        , Observed
            { snapshot with
              revision = 99L
            ; policy = { length = 6; alphabet = Ascii_alphanumeric }
            } )
    ]
    ~f:(fun event -> assert (Option.is_none (dispatch event)));
  assert (Option.equal Int.equal (dispatch (event 2L)) (Some 2));
  let updated =
    O.Config.create ~policy ~label:"Updated" ~disabled:true ~read_only:true () |> ok
  in
  let operations = commit (Some (view ~config:updated (fun _ -> 3))) in
  assert (
    List.for_all operations ~f:(function
      | Wire.Op.Set_otp_input _ -> true
      | _ -> false));
  assert (not (List.is_empty operations));
  assert (
    Option.equal
      Int.equal
      (dispatch
         (Otp_input_event (window, node, handler, 1L, Changed { full with revision = 3L })))
      (Some 3));
  assert (
    Option.equal
      Int.equal
      (dispatch
         (Otp_input_event (window, node, handler, 1L, Complete { full with revision = 4L })))
      (Some 3));
  let pending =
    Reconciler.prepare
      reconciler
      ~theme:Theme.default
      (Some (view ~config:updated (fun _ -> 4)))
    |> ok
  in
  assert (Option.equal Int.equal (dispatch (event 5L)) (Some 3));
  Reconciler.accept reconciler pending |> ok;
  assert (Option.is_none (dispatch (event 5L)));
  assert (Option.equal Int.equal (dispatch (event 6L)) (Some 4));
  let changed_policy = O.Policy.create ~length:5 () |> ok in
  let changed_config = O.Config.create ~policy:changed_policy ~label:"Changed" () |> ok in
  assert (
    Result.is_error
      (Reconciler.prepare
         reconciler
         ~theme:Theme.default
         (Some (view ~config:changed_config (fun _ -> 9)))));
  let duplicate =
    View.column
      [ View.column ~key:(Key.of_int 1) [ view (fun _ -> 9) ]
      ; View.column ~key:(Key.of_int 2) [ view (fun _ -> 9) ]
      ]
  in
  assert (
    Result.is_error (Reconciler.prepare reconciler ~theme:Theme.default (Some duplicate)));
  assert (Option.equal Int.equal (dispatch (event 7L)) (Some 4));
  let foreign_value =
    O.Value.of_string
      (O.Policy.create ~length:6 ~alphabet:Ascii_alphanumeric () |> ok)
      "ABC"
    |> Result.map_error ~f:(fun e -> Error.create_s (O.Input_error.sexp_of_t e))
    |> ok
  in
  assert (
    Result.is_error
      (Reconciler.prepare
         reconciler
         ~theme:Theme.default
         (Some (view ~initial:foreign_value (fun _ -> 9)))));
  ignore (commit None : Wire.Op.t list);
  assert (Option.is_none (dispatch (event 8L)));
  let new_node, new_handler =
    identity (commit (Some (view ~config:changed_config (fun _ -> 5))))
  in
  assert (not (Gpuio_protocol.Node_id.equal new_node node));
  assert (Option.is_none (dispatch (event 9L)));
  let fresh =
    Wire.Event.Otp_input_event
      ( window
      , new_node
      , new_handler
      , 4L
      , Observed
          { snapshot with revision = 0L; policy = O.Expert.policy_to_wire changed_policy }
      )
  in
  assert (Option.equal Int.equal (dispatch fresh) (Some 5));
  Reconciler.close reconciler;
  assert (Option.is_none (dispatch fresh));
  print_endline
    "stable seed/owner, latest callbacks, policy and revision fences, atomic failures, \
     disabled delivery and remount";
  [%expect
    {| stable seed/owner, latest callbacks, policy and revision fences, atomic failures, disabled delivery and remount |}]
;;
