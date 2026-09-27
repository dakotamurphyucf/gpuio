open Core
open Gpuio
module C = Calendar
module W = Gpuio_protocol.Calendar_wire
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let day = Date.of_string
let initial_month = C.Month.of_date (day "2024-02-01") |> ok
let initial = C.Selection.single (day "2024-02-29") |> ok
let config = C.Config.create ~label:"Calendar" () |> ok
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok

let snapshot =
  { W.Snapshot.revision = 0L
  ; mode = Single
  ; selection = C.Expert.selection_to_wire initial
  ; selection_allowed = true
  ; month = C.Expert.month_to_wire initial_month
  ; focused_date = C.Expert.date_to_ordinal (day "2024-02-29") |> ok
  ; presentation = Days
  ; focused = false
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

let%expect_test "calendar correlated command fixtures and malformed responses" =
  let request =
    Wire.Message.Calendar_command
      ( 9L
      , window
      , node
      , Replace
          { selection = Range_start (C.Expert.date_to_ordinal (day "2024-02-28") |> ok)
          ; if_revision = Some 7L
          } )
  in
  let selected =
    { snapshot with
      revision = 8L
    ; mode = Range
    ; selection =
        C.Expert.selection_to_wire
          (C.Selection.range
             (C.Range.create ~first:(day "2024-03-04") ~last:(day "2024-03-05") |> ok))
    ; month = C.Expert.month_to_wire (C.Month.of_date (day "2024-03-01") |> ok)
    ; focused_date = C.Expert.date_to_ordinal (day "2024-03-05") |> ok
    ; focused = true
    }
  in
  let response = Wire.Event.Calendar_result (9L, window, node, Applied selected) in
  let encoded = event_bytes [ response ] in
  Eio_main.run (fun env ->
    let fs = Eio.Stdenv.fs env in
    assert (
      String.equal
        (Wire.Message.encode request |> ok |> hex)
        (Eio.Path.load Eio.Path.(fs / "calendar-command-request.hex") |> String.strip));
    assert (
      String.equal
        (hex encoded)
        (Eio.Path.load Eio.Path.(fs / "calendar-command-events.hex") |> String.strip)));
  assert (List.equal Wire.Event.equal (Wire.Event.decode encoded |> ok) [ response ]);
  for length = 0 to String.length encoded - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix encoded length)))
  done;
  assert (Result.is_error (Wire.Event.decode (encoded ^ "\000")));
  List.iter [ 0L; -1L; Int64.min_value ] ~f:(fun correlation ->
    assert (
      Result.is_error
        (Wire.Message.encode
           (Calendar_command (correlation, window, node, Read_snapshot))));
    assert (
      Result.is_error
        (Wire.Event.decode
           (event_bytes [ Calendar_result (correlation, window, node, Applied selected) ]))));
  List.iter
    [ { selected with revision = -1L }
    ; { selected with mode = Single }
    ; { selected with focused_date = -1L }
    ; { selected with month = 0L }
    ; { selected with selection = Empty; selection_allowed = false }
    ]
    ~f:(fun invalid ->
      assert (
        Result.is_error
          (Wire.Event.decode
             (event_bytes [ Calendar_result (9L, window, node, Applied invalid) ]))));
  List.iter
    [ W.Command.Move_months Int64.max_value
    ; Move_months (-119988L)
    ; Focus_date (-1L)
    ; Show_month 119988L
    ; Clear { if_revision = Some (-1L) }
    ]
    ~f:(fun invalid ->
      assert (
        Result.is_error
          (Wire.Message.encode (Calendar_command (9L, window, node, invalid)))));
  let failed = Wire.Event.Calendar_result (9L, window, node, Failed Stale_revision) in
  assert (
    List.equal
      Wire.Event.equal
      (Wire.Event.decode (event_bytes [ failed ]) |> ok)
      [ failed ]);
  print_endline
    "message 17 / event 51: independent fixtures, correlation, bounded commands and \
     validated snapshots";
  [%expect
    {| message 17 / event 51: independent fixtures, correlation, bounded commands and validated snapshots |}]
;;

let%expect_test "calendar retained envelopes use independent tags and validate events" =
  let config = Calendar_wire_test.config () in
  let request =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Calendar, "", Some handler)
          ; Set_calendar
              ( node
              , C.Expert.config_to_wire config
              , Empty
              , C.Expert.month_to_wire initial_month )
          ; Set_root (Some node)
          ]
      }
  in
  let selected =
    { snapshot with
      revision = 8L
    ; mode = Range
    ; selection =
        C.Expert.selection_to_wire
          (C.Selection.range
             (C.Range.create ~first:(day "2024-03-04") ~last:(day "2024-03-05") |> ok))
    ; month = C.Expert.month_to_wire (C.Month.of_date (day "2024-03-01") |> ok)
    ; focused_date = C.Expert.date_to_ordinal (day "2024-03-05") |> ok
    ; focused = true
    }
  in
  let event = Wire.Event.Calendar_event (window, node, handler, 1L, Selected selected) in
  let bytes = event_bytes [ event ] in
  Eio_main.run (fun env ->
    let fs = Eio.Stdenv.fs env in
    assert (
      String.equal
        (Wire.Message.encode request |> ok |> hex)
        (Eio.Path.load Eio.Path.(fs / "calendar-request.hex") |> String.strip));
    assert (
      String.equal
        (hex bytes)
        (Eio.Path.load Eio.Path.(fs / "calendar-events.hex") |> String.strip)));
  assert (List.equal Wire.Event.equal (Wire.Event.decode bytes |> ok) [ event ]);
  for length = 0 to String.length bytes - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix bytes length)))
  done;
  assert (Result.is_error (Wire.Event.decode (bytes ^ "\000")));
  List.iter
    [ -1L, W.Event.Observed snapshot
    ; 1L, Changed snapshot
    ; 1L, Selected { snapshot with revision = 1L; selection_allowed = false }
    ; 1L, Selected { snapshot with revision = 1L; selection = Empty }
    ; 1L, Observed { snapshot with focused_date = -1L }
    ; 1L, Observed { snapshot with month = 0L }
    ]
    ~f:(fun (revision, event) ->
      assert (
        Result.is_error
          (Wire.Event.decode
             (event_bytes [ Calendar_event (window, node, handler, revision, event) ]))));
  print_endline
    "Kind 40, operation 46, event 50: independent fixtures, exact consumption and \
     semantic validation";
  [%expect
    {| Kind 40, operation 46, event 50: independent fixtures, exact consumption and semantic validation |}]
;;

let%expect_test
    "calendar retained identity, history, pending callbacks and stale event fences"
  =
  let reconciler = Reconciler.create window in
  let controller = Key.of_string_exn "calendar" in
  let view
        ?(config = config)
        ?(initial = initial)
        ?(initial_month = initial_month)
        callback
    =
    View.calendar ~controller ~config ~initial ~initial_month ~on_event:callback ()
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
      | Wire.Op.Create (node, Calendar, "", Some handler) -> Some (node, handler)
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
    Wire.Event.Calendar_event
      (window, node, handler, tree_revision, Observed { snapshot with revision })
  in
  let dispatch = Reconciler.dispatch reconciler in
  assert (Option.equal Int.equal (dispatch (event 0L)) (Some 1));
  assert (
    List.is_empty
      (commit
         (Some
            (view
               ~initial:C.Selection.empty
               ~initial_month:(C.Month.of_date (day "2030-01-01") |> ok)
               (fun _ -> 2)))));
  assert (Option.is_none (dispatch (event 0L)));
  assert (Option.equal Int.equal (dispatch (event 1L)) (Some 2));
  List.iter
    [ event ~tree_revision:99L 2L
    ; event ~tree_revision:(-1L) 2L
    ; event ~window:(Gpuio_protocol.Window_id.create ~slot:0L ~generation:2L |> ok) 2L
    ; event ~node:(Gpuio_protocol.Node_id.create ~slot:0L ~generation:2L |> ok) 2L
    ; event ~handler:(Gpuio_protocol.Handler_id.create ~slot:0L ~generation:2L |> ok) 2L
    ; Calendar_event
        ( window
        , node
        , handler
        , 1L
        , Observed { snapshot with revision = 99L; mode = Range; selection = Empty } )
    ; Calendar_event
        (window, node, handler, 1L, Observed { snapshot with revision = 99L; month = 0L })
    ; Calendar_event
        ( window
        , node
        , handler
        , 1L
        , Selected { snapshot with revision = 99L; selection_allowed = false } )
    ]
    ~f:(fun event -> assert (Option.is_none (dispatch event)));
  assert (Option.equal Int.equal (dispatch (event 2L)) (Some 2));
  let constraints = C.Constraints.create ~disabled_dates:[ day "2024-02-29" ] () |> ok in
  let updated =
    C.Config.create ~label:"History" ~constraints ~disabled:true ~read_only:true () |> ok
  in
  let operations = commit (Some (view ~config:updated (fun _ -> 3))) in
  assert (List.length operations = 1);
  assert (
    List.for_all operations ~f:(function
      | Wire.Op.Set_calendar _ -> true
      | _ -> false));
  let historical revision =
    Wire.Event.Calendar_event
      ( window
      , node
      , handler
      , 1L
      , Observed { snapshot with revision; selection_allowed = false } )
  in
  assert (Option.equal Int.equal (dispatch (historical 3L)) (Some 3));
  let pending =
    Reconciler.prepare
      reconciler
      ~theme:Theme.default
      (Some (view ~config:updated (fun _ -> 4)))
    |> ok
  in
  assert (Option.equal Int.equal (dispatch (historical 4L)) (Some 3));
  Reconciler.accept reconciler pending |> ok;
  assert (Option.is_none (dispatch (historical 4L)));
  assert (Option.equal Int.equal (dispatch (historical 5L)) (Some 4));
  let range_config = C.Config.create ~mode:Range ~label:"Range" () |> ok in
  List.iter
    [ view ~config:range_config ~initial:C.Selection.empty (fun _ -> 9)
    ; view ~initial:(C.Selection.range_start (day "2024-02-01") |> ok) (fun _ -> 9)
    ; View.column
        [ View.column ~key:(Key.of_int 1) [ view (fun _ -> 9) ]
        ; View.column ~key:(Key.of_int 2) [ view (fun _ -> 9) ]
        ]
    ]
    ~f:(fun view ->
      assert (
        Result.is_error (Reconciler.prepare reconciler ~theme:Theme.default (Some view))));
  assert (Option.equal Int.equal (dispatch (historical 6L)) (Some 4));
  ignore (commit None : Wire.Op.t list);
  assert (Option.is_none (dispatch (event 7L)));
  assert (
    Result.is_error
      (Reconciler.prepare
         reconciler
         ~theme:Theme.default
         (Some (view ~config:updated (fun _ -> 9)))));
  let new_node, new_handler =
    identity
      (commit (Some (view ~config:range_config ~initial:C.Selection.empty (fun _ -> 5))))
  in
  assert (not (Gpuio_protocol.Node_id.equal new_node node));
  assert (Option.is_none (dispatch (event 8L)));
  let fresh =
    Wire.Event.Calendar_event
      ( window
      , new_node
      , new_handler
      , 4L
      , Observed { snapshot with revision = 0L; mode = Range; selection = Empty } )
  in
  assert (Option.equal Int.equal (dispatch fresh) (Some 5));
  Reconciler.close reconciler;
  assert (Option.is_none (dispatch fresh));
  print_endline
    "seed-only updates, historical invalidation, callback refresh, immutable mode, \
     duplicate owners and stale revisions/leases";
  [%expect
    {| seed-only updates, historical invalidation, callback refresh, immutable mode, duplicate owners and stale revisions/leases |}]
;;
