open Core
module C = Gpuio.Calendar
module W = Gpuio_protocol.Calendar_wire

let ok = Or_error.ok_exn
let day = Date.of_string

let constraints () =
  C.Constraints.create
    ~min:(day "2024-01-01")
    ~max:(day "2030-12-31")
    ~disabled_dates:[ day "2024-02-29" ]
    ~disabled_ranges:
      [ C.Range.create ~first:(day "2024-04-10") ~last:(day "2024-04-12") |> ok ]
    ~disabled_weekdays:[ Sun; Sat ]
    ()
  |> ok
;;

let config () =
  C.Config.create
    ~mode:Range
    ~constraints:(constraints ())
    ~today:(day "2024-03-01")
    ~label:"Review dates"
    ~auto_focus:true
    ()
  |> ok
;;

let hex value =
  String.to_list value
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let fixture fs name writer reader equal value =
  let bytes = Bin_prot.Utils.bin_dump writer value in
  assert (
    String.equal
      (hex (Bigstring.to_string bytes))
      (Eio.Path.load Eio.Path.(fs / name) |> String.strip));
  let pos_ref = ref 0 in
  assert (equal value (reader bytes ~pos_ref));
  assert (!pos_ref = Bigstring.length bytes)
;;

let%expect_test "independent calendar domain/config fixtures and semantic round trips" =
  let config = config () in
  let selection =
    C.Selection.range
      (C.Range.create ~first:(day "2024-02-29") ~last:(day "2024-03-01") |> ok)
  in
  Eio_main.run (fun env ->
    let fs = Eio.Stdenv.fs env in
    fixture
      fs
      "calendar-config.hex"
      W.Config.bin_writer_t
      W.Config.bin_read_t
      W.Config.equal
      (C.Expert.config_to_wire config);
    fixture
      fs
      "calendar-constraints.hex"
      W.Constraints.bin_writer_t
      W.Constraints.bin_read_t
      W.Constraints.equal
      (C.Expert.constraints_to_wire (constraints ()));
    fixture
      fs
      "calendar-selection.hex"
      W.Selection.bin_writer_t
      W.Selection.bin_read_t
      W.Selection.equal
      (C.Expert.selection_to_wire selection));
  assert (
    C.Config.equal config (C.Expert.config_of_wire (C.Expert.config_to_wire config) |> ok));
  assert (
    C.Selection.equal
      selection
      (C.Expert.selection_of_wire (C.Expert.selection_to_wire selection) |> ok));
  assert (C.Mode.equal (C.Config.mode config) Range);
  assert (Day_of_week.equal (C.Config.first_weekday config) Mon);
  assert (Option.equal Date.equal (C.Config.today config) (Some (day "2024-03-01")));
  assert (not (C.Config.is_disabled config || C.Config.is_read_only config));
  assert (not (C.Constraints.allows (C.Config.constraints config) (day "2024-02-29")));
  print_endline
    "three independently assembled fixtures agree; semantic round trips preserve \
     configuration and ordered selection";
  [%expect
    {| three independently assembled fixtures agree; semantic round trips preserve configuration and ordered selection |}]
;;

let%expect_test "untrusted wire configuration rejects before domain conversion" =
  let c = C.Expert.config_to_wire (config ()) in
  List.iter
    [ { c with first_weekday = -1L }
    ; { c with first_weekday = 7L }
    ; { c with today = Some Int64.max_value }
    ; { c with today = Some (-1L) }
    ; { c with constraints = { c.constraints with min = Int64.min_value } }
    ; { c with
        constraints =
          { c.constraints with min = c.constraints.max; max = c.constraints.min }
      }
    ; { c with
        constraints =
          { c.constraints with disabled_dates = List.init 513 ~f:(fun _ -> 0L) }
      }
    ; { c with
        constraints =
          { c.constraints with disabled_ranges = [ { first = 1L; last = 0L } ] }
      }
    ; { c with constraints = { c.constraints with disabled_weekdays = [ 7L ] } }
    ; { c with labels = { c.labels with months = List.tl_exn c.labels.months } }
    ; { c with labels = { c.labels with weekdays = [] } }
    ; { c with labels = { c.labels with short_weekdays = [] } }
    ]
    ~f:(fun config -> assert (Result.is_error (C.Expert.config_of_wire config)));
  List.iter
    [ ""; "  "; "bad\nlabel"; "\t"; "\127"; "\000"; "\255"; String.make 129 'x' ]
    ~f:(fun label ->
      let c = { c with labels = { c.labels with choose_month = label } } in
      assert (Result.is_error (C.Expert.config_of_wire c)));
  List.iter
    [ ""; "  "; "bad\nlabel"; "\000"; "\255"; String.make 4097 'x' ]
    ~f:(fun label -> assert (Result.is_error (C.Config.create ~label ())));
  List.iter
    [ W.Selection.Single (-1L)
    ; Range_start Int64.max_value
    ; Range { first = 1L; last = 0L }
    ]
    ~f:(fun selection -> assert (Result.is_error (C.Expert.selection_of_wire selection)));
  let canonical = C.Expert.constraints_to_wire (constraints ()) in
  let raw =
    { canonical with
      disabled_dates = canonical.disabled_dates @ canonical.disabled_dates
    ; disabled_weekdays = [ 6L; 0L; 6L ]
    }
  in
  assert (
    W.Constraints.equal
      canonical
      (C.Expert.constraints_of_wire raw |> ok |> C.Expert.constraints_to_wire));
  print_endline
    "invalid dates, ranges, quotas, weekday indices and label payloads reject; valid \
     duplicates canonicalize";
  [%expect
    {| invalid dates, ranges, quotas, weekday indices and label payloads reject; valid duplicates canonicalize |}]
;;

let%expect_test "locale and explicit today are presentation only" =
  let c = config () in
  let wire = C.Expert.config_to_wire c in
  let labels =
    { wire.labels with
      months =
        List.mapi wire.labels.months ~f:(fun index value ->
          if index = 2 then "März" else value)
    ; today = "Heute"
    }
  in
  let changed =
    C.Expert.config_of_wire { wire with labels; first_weekday = 0L; today = None } |> ok
  in
  assert (C.Constraints.equal (C.Config.constraints c) (C.Config.constraints changed));
  assert (C.Mode.equal (C.Config.mode c) (C.Config.mode changed));
  assert (Day_of_week.equal (C.Config.first_weekday changed) Sun);
  assert (Option.is_none (C.Config.today changed));
  assert (not (C.Labels.equal (C.Config.labels c) (C.Config.labels changed)));
  print_endline
    "Unicode labels, first weekday and today marker change independently of selection \
     constraints";
  [%expect
    {| Unicode labels, first weekday and today marker change independently of selection constraints |}]
;;

let%expect_test "calendar observations and guarded commands have paired wire contracts" =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let selection =
    C.Selection.range
      (C.Range.create ~first:(day "2024-03-04") ~last:(day "2024-03-05") |> ok)
  in
  let wire =
    { W.Snapshot.revision = 8L
    ; mode = Range
    ; selection = C.Expert.selection_to_wire selection
    ; selection_allowed = true
    ; month = 24278L
    ; focused_date = C.Expert.date_to_ordinal (day "2024-03-05") |> ok
    ; presentation = Days
    ; focused = true
    }
  in
  let replace =
    C.Command.Replace
      { selection = C.Selection.range_start (day "2024-02-28") |> ok
      ; if_revision = Some (C.Revision.of_int64 7L |> ok)
      }
  in
  Eio_main.run (fun env ->
    let fs = Eio.Stdenv.fs env in
    fixture
      fs
      "calendar-selected.hex"
      W.Event.bin_writer_t
      W.Event.bin_read_t
      W.Event.equal
      (Selected wire);
    fixture
      fs
      "calendar-replace.hex"
      W.Command.bin_writer_t
      W.Command.bin_read_t
      W.Command.equal
      (C.Expert.command_to_wire replace |> ok));
  let snapshot = C.Expert.snapshot_of_wire ~window ~node wire |> ok in
  assert (C.Selection.equal (C.Snapshot.selection snapshot) selection);
  assert (
    C.Month.equal (C.Snapshot.month snapshot) (C.Month.create ~year:2024 ~month:Mar |> ok));
  assert (Date.equal (C.Snapshot.focused_date snapshot) (day "2024-03-05"));
  assert (C.Snapshot.selection_allowed snapshot && C.Snapshot.focused snapshot);
  assert (Gpuio_protocol.Window_id.equal (C.Expert.window snapshot) window);
  assert (Gpuio_protocol.Node_id.equal (C.Expert.node snapshot) node);
  List.iter
    [ { wire with revision = -1L }
    ; { wire with mode = Single }
    ; { wire with month = 24277L }
    ; { wire with focused_date = Int64.max_value }
    ; { wire with selection = Empty; selection_allowed = false }
    ]
    ~f:(fun snapshot ->
      assert (Result.is_error (C.Expert.snapshot_of_wire ~window ~node snapshot)));
  List.iter
    [ W.Event.Selected { wire with selection = Range_start wire.focused_date }
    ; Selected { wire with selection_allowed = false }
    ; Changed { wire with revision = 0L }
    ]
    ~f:(fun event ->
      assert (Result.is_error (C.Expert.event_of_wire ~window ~node event)));
  assert (
    Result.is_ok
      (C.Expert.event_of_wire
         ~window
         ~node
         (Observed { wire with selection_allowed = false })));
  List.iter
    [ C.Command.Move_months Int.max_value
    ; Move_months Int.min_value
    ; Focus_date (Date.create_exn ~y:0 ~m:Jan ~d:1)
    ]
    ~f:(fun command -> assert (Result.is_error (C.Expert.command_to_wire command)));
  print_endline
    "typed snapshots retain leases and reject inconsistent state; guarded replacement \
     and completion agree across codecs";
  [%expect
    {| typed snapshots retain leases and reject inconsistent state; guarded replacement and completion agree across codecs |}]
;;
