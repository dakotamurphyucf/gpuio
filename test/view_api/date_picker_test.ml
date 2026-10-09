open Core
open Gpuio
module C = Calendar
module P = Date_picker

let ok = Or_error.ok_exn
let day = Date.of_string
let month = C.Month.of_date (day "2024-02-01") |> ok
let value = C.Selection.single (day "2024-02-29") |> ok
let single = C.Config.create ~label:"Date" () |> ok
let span = C.Config.create ~mode:Range ~label:"Dates" () |> ok

let snapshot ?(generation = 1L) ?(revision = 0L) config selection =
  C.Expert.snapshot_of_wire
    ~window:(Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok)
    ~node:(Gpuio_protocol.Node_id.create ~slot:0L ~generation |> ok)
    { Gpuio_protocol.Calendar_wire.Snapshot.revision
    ; mode =
        (match C.Config.mode config with
         | Single -> Single
         | Range -> Range)
    ; selection = C.Expert.selection_to_wire selection
    ; selection_allowed = true
    ; month = C.Expert.month_to_wire month
    ; focused_date = C.Expert.date_to_ordinal (day "2024-02-29") |> ok
    ; presentation = Days
    ; focused = false
    }
  |> ok
;;

let open_session ?(config = single) ?(value = value) state =
  let state = P.open_popup state ~config ~value ~initial_month:month in
  let session = P.session state ~config ~value |> Option.value_exn in
  state, session
;;

let%expect_test "picker confirms only a current complete draft and discards cancellation" =
  let state, session = open_session P.empty in
  let id = P.Session.id session in
  assert (C.Selection.equal (P.Session.initial session) value);
  let same = P.open_popup state ~config:single ~value ~initial_month:month in
  assert (P.equal state same);
  let native = snapshot single value in
  let _, result = P.confirm state ~config:single ~value ~session:id native in
  print_s [%sexp (result : (C.Selection.t, P.Error.t) Result.t)];
  let state = P.observe state ~session:id native in
  let closed, result = P.confirm state ~config:single ~value ~session:id native in
  print_s [%sexp (result : (C.Selection.t, P.Error.t) Result.t)];
  assert (Option.is_none (P.session closed ~config:single ~value));
  let next, next_session = open_session closed in
  assert (not (P.Session.Id.equal id (P.Session.id next_session)));
  assert (P.equal next (P.cancel next ~session:id));
  assert (P.equal next (P.observe next ~session:id native));
  let unchanged, stale = P.confirm next ~config:single ~value ~session:id native in
  assert (P.equal next unchanged);
  print_s [%sexp (stale : (C.Selection.t, P.Error.t) Result.t)];
  let cancelled = P.cancel next ~session:(P.Session.id next_session) in
  assert (Option.is_none (P.draft cancelled));
  let _, reopened = open_session cancelled in
  assert (C.Selection.equal (P.Session.initial reopened) value);
  [%expect
    {|
    (Error Not_ready)
    (Ok (Single 2024-02-29))
    (Error Stale_session)
    |}]
;;

let%expect_test "partial and empty ranges, readonly and configuration updates" =
  let value = C.Selection.empty in
  let state, session = open_session ~config:span ~value P.empty in
  let id = P.Session.id session in
  let partial = C.Selection.range_start (day "2024-02-28") |> ok in
  let native = snapshot span partial in
  let state = P.observe state ~session:id native in
  let state, result = P.confirm state ~config:span ~value ~session:id native in
  print_s [%sexp (result : (C.Selection.t, P.Error.t) Result.t)];
  let selection =
    C.Range.create ~first:(day "2024-02-28") ~last:(day "2024-02-29")
    |> ok
    |> C.Selection.range
  in
  let completed = snapshot ~revision:1L span selection in
  let state = P.observe state ~session:id completed in
  let readonly = C.Config.create ~mode:Range ~label:"Dates" ~read_only:true () |> ok in
  let _, result = P.confirm state ~config:readonly ~value ~session:id completed in
  print_s [%sexp (result : (C.Selection.t, P.Error.t) Result.t)];
  let constraints = C.Constraints.create ~disabled_dates:[ day "2024-02-29" ] () |> ok in
  let constrained = C.Config.create ~mode:Range ~constraints ~label:"Dates" () |> ok in
  let _, result = P.confirm state ~config:constrained ~value ~session:id completed in
  print_s [%sexp (result : (C.Selection.t, P.Error.t) Result.t)];
  let empty = snapshot ~revision:2L span C.Selection.empty in
  let _, result = P.confirm state ~config:constrained ~value ~session:id empty in
  print_s [%sexp (result : (C.Selection.t, P.Error.t) Result.t)];
  [%expect
    {|
    (Error Incomplete_range)
    (Error Read_only)
    (Error Disallowed_selection)
    (Ok Empty)
    |}]
;;

let%expect_test "external values, modes, leases and revisions fence delayed confirmation" =
  let state, session = open_session P.empty in
  let id = P.Session.id session in
  let native = snapshot ~revision:2L single value in
  let state = P.observe state ~session:id native in
  List.iter
    [ snapshot ~revision:1L single value
    ; snapshot ~generation:2L ~revision:3L single value
    ]
    ~f:(fun invalid ->
      assert (P.equal state (P.observe state ~session:id invalid));
      let _, result = P.confirm state ~config:single ~value ~session:id invalid in
      print_s [%sexp (result : (C.Selection.t, P.Error.t) Result.t)]);
  let externally_changed = C.Selection.single (day "2024-02-27") |> ok in
  List.iter
    [ single, externally_changed
    ; span, C.Selection.empty
    ; C.Config.create ~label:"Date" ~disabled:true () |> ok, value
    ]
    ~f:(fun (config, value) ->
      assert (Option.is_none (P.session state ~config ~value));
      let closed, result = P.confirm state ~config ~value ~session:id native in
      assert (Option.is_none (P.draft closed));
      print_s [%sexp (result : (C.Selection.t, P.Error.t) Result.t)]);
  [%expect
    {|
    (Error Stale_draft)
    (Error Stale_draft)
    (Error Stale_session)
    (Error Stale_session)
    (Error Stale_session)
    |}]
;;

let%expect_test "historical disallowed committed values survive opening and cancellation" =
  let constraints = C.Constraints.create ~disabled_dates:[ day "2024-02-29" ] () |> ok in
  let config = C.Config.create ~constraints ~label:"Date" () |> ok in
  let state, session = open_session ~config P.empty in
  assert (C.Selection.equal (P.Session.original session) value);
  assert (C.Selection.equal (P.Session.initial session) C.Selection.empty);
  let closed = P.cancel state ~session:(P.Session.id session) in
  let _, reopened = open_session ~config closed in
  assert (C.Selection.equal (P.Session.original reopened) value);
  let disabled = C.Config.create ~label:"Date" ~disabled:true () |> ok in
  let denied = P.open_popup P.empty ~config:disabled ~value ~initial_month:month in
  print_s [%sexp (P.error denied : P.Error.t option)];
  let wrong = P.open_popup P.empty ~config:span ~value ~initial_month:month in
  print_s [%sexp (P.error wrong : P.Error.t option)];
  print_endline "historical value preserved; cancellation never changes application data";
  [%expect
    {|
    (Disabled)
    (Wrong_mode)
    historical value preserved; cancellation never changes application data
    |}]
;;

let%expect_test "native remount re-seeds the draft and fences old command replies" =
  let state, session = open_session P.empty in
  let id = P.Session.id session in
  let old = snapshot ~revision:5L single C.Selection.empty in
  let state = P.observe_native state ~session:id old in
  let fresh = snapshot ~generation:2L single value in
  let state = P.observe_native state ~session:id fresh in
  assert (Option.equal C.Snapshot.equal (P.draft state) (Some fresh));
  assert (P.equal state (P.observe state ~session:id old));
  let unchanged, stale = P.confirm state ~config:single ~value ~session:id old in
  assert (P.equal state unchanged);
  print_s [%sexp (stale : (C.Selection.t, P.Error.t) Result.t)];
  let _, confirmed = P.confirm state ~config:single ~value ~session:id fresh in
  print_s [%sexp (confirmed : (C.Selection.t, P.Error.t) Result.t)];
  [%expect
    {|
    (Error Stale_draft)
    (Ok (Single 2024-02-29))
    |}]
;;

let preset ?(label = "Leap day") ?(selection = value) id =
  P.Preset.create ~id:(Choice.Id.of_string id |> ok) ~label ~selection |> ok
;;

let%expect_test "presets are bounded complete selections with stable distinct IDs" =
  let leap = preset "leap" in
  let clear = preset ~label:"Clear" ~selection:C.Selection.empty "clear" in
  let presets = P.Preset.Collection.create [ leap; clear ] |> ok in
  assert (List.equal P.Preset.equal (P.Preset.Collection.to_list presets) [ leap; clear ]);
  let invalid label selection =
    Result.is_error
      (P.Preset.create ~id:(Choice.Id.of_string "test" |> ok) ~label ~selection)
  in
  List.iter
    [ ""; "\000"; "\255"; String.make 4097 'x' ]
    ~f:(fun label -> assert (invalid label value));
  assert (invalid "Partial" (C.Selection.range_start (day "2024-02-28") |> ok));
  assert (Result.is_error (P.Preset.Collection.create [ leap; leap ]));
  let maximum =
    List.init P.Preset.Collection.max_presets ~f:(fun i -> preset (Int.to_string i))
  in
  assert (Result.is_ok (P.Preset.Collection.create maximum));
  assert (Result.is_error (P.Preset.Collection.create (leap :: maximum)));
  print_endline "32 complete presets; order preserved; labels and duplicate IDs validated";
  [%expect {| 32 complete presets; order preserved; labels and duplicate IDs validated |}]
;;

let%expect_test "preset availability follows current mode, constraints and policy" =
  let leap = preset "leap" in
  let constraints = C.Constraints.create ~disabled_dates:[ day "2024-02-29" ] () |> ok in
  let constrained = C.Config.create ~constraints ~label:"Date" () |> ok in
  let readonly = C.Config.create ~read_only:true ~label:"Date" () |> ok in
  let disabled = C.Config.create ~disabled:true ~label:"Date" () |> ok in
  List.iter [ single; span; constrained; readonly; disabled ] ~f:(fun config ->
    print_s [%sexp (P.Preset.validate leap ~config : (unit, P.Error.t) Result.t)]);
  let range =
    C.Range.create ~first:(day "2024-02-28") ~last:(day "2024-03-01")
    |> ok
    |> C.Selection.range
  in
  let range = preset ~selection:range "range" in
  List.iter [ C.Range_policy.Every_day; Endpoints_only ] ~f:(fun range_policy ->
    let constraints =
      C.Constraints.create ~disabled_dates:[ day "2024-02-29" ] ~range_policy () |> ok
    in
    let config = C.Config.create ~mode:Range ~constraints ~label:"Dates" () |> ok in
    print_s [%sexp (P.Preset.validate range ~config : (unit, P.Error.t) Result.t)]);
  assert (
    Result.is_ok
      (P.Preset.validate
         (preset ~selection:C.Selection.empty "clear")
         ~config:constrained));
  [%expect
    {|
    (Ok ())
    (Error Wrong_mode)
    (Error Disallowed_selection)
    (Error Read_only)
    (Error Disabled)
    (Error Disallowed_selection)
    (Ok ())
    |}]
;;
