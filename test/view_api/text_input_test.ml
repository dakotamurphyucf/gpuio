open Core
module Input = Gpuio.Text_input

let%expect_test "configuration and UTF-8 selections validate domain boundaries" =
  assert (Result.is_error (Input.Config.create ~mode:Single_line ~label:"" ()));
  assert (
    Result.is_error (Input.Config.create ~mode:Single_line ~label:"Name" ~max_rows:2 ()));
  assert (
    Result.is_error
      (Input.Config.create ~mode:Multiline ~label:"Message" ~min_rows:3 ~max_rows:2 ()));
  assert (Result.is_error (Input.validate_text ~mode:Single_line "a\nb"));
  assert (
    Result.is_error
      (Input.validate_text ~mode:Multiline (String.make (Input.max_text_bytes + 1) 'x')));
  let select anchor head = Input.Selection.create ~anchor ~head |> Or_error.ok_exn in
  let text = "é界" in
  assert (Result.is_error (Input.Selection.validate (select 1 1) ~text));
  assert (Result.is_error (Input.Selection.validate (select 0 6) ~text));
  assert (Result.is_error (Input.Selection.validate (select 0 0) ~text:"\255"));
  let reversed = select 5 2 in
  Input.Selection.validate reversed ~text |> Or_error.ok_exn;
  print_s
    [%sexp (Input.Selection.anchor reversed : int), (Input.Selection.head reversed : int)];
  [%expect {| (5 2) |}]
;;

let%expect_test
    "submission preserves exact native identity and cannot represent composition"
  =
  let window =
    Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
  in
  let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:2L |> Or_error.ok_exn in
  let revision = Input.Revision.of_int64 7L |> Or_error.ok_exn in
  let selection = Input.Selection.create ~anchor:2 ~head:2 |> Or_error.ok_exn in
  let snapshot composition =
    Input.Expert.snapshot
      ~window
      ~node
      ~revision
      ~text:"é"
      ~selection
      ~composition
      ~focused:true
    |> Or_error.ok_exn
  in
  assert (Result.is_error (Input.Expert.submission (snapshot (Some selection))));
  let submitted = Input.Expert.submission (snapshot None) |> Or_error.ok_exn in
  let retained = Input.Expert.submission_snapshot submitted in
  assert (Gpuio_protocol.Node_id.equal node (Input.Expert.node retained));
  assert (Result.is_error (Input.Revision.of_int64 (-1L)));
  print_s
    [%sexp
      (Input.Submission.text submitted : string)
    , (Input.Revision.to_int64 (Input.Submission.revision submitted) : int64)];
  [%expect {| ("\195\169" 7) |}]
;;

let%expect_test
    "editor observations do not replace text; one controller has one placement"
  =
  let open Gpuio in
  let open Gpuio_protocol in
  let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let reconciler = Reconciler.create window in
  let config =
    Input.Config.create ~mode:Multiline ~label:"Message" () |> Or_error.ok_exn
  in
  let key = Key.of_string_exn "composer" in
  let view text callback =
    View.text_input
      ~controller:key
      ~initial_text:text
      ~config
      ~on_event:(fun _ -> callback)
      ()
    |> Or_error.ok_exn
  in
  let commit view =
    let update =
      Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> Or_error.ok_exn
    in
    Reconciler.accept reconciler update |> Or_error.ok_exn;
    update
  in
  let initial = commit (view "initial" "first") in
  let node, handler =
    match Reconciler.message initial with
    | Some (Wire.Message.Apply tx) ->
      List.find_map_exn tx.operations ~f:(function
        | Create (node, Textarea, _, Some handler) -> Some (node, handler)
        | _ -> None)
    | _ -> assert false
  in
  let same = commit (view "this later initial value is ignored" "latest") in
  assert (Option.is_none (Reconciler.message same));
  let snapshot : Wire.Editor.Snapshot.t =
    { revision = 2L
    ; text = "native"
    ; selection = { anchor = 6L; head = 6L }
    ; composition = None
    ; focused = true
    }
  in
  let event = Wire.Event.Editor_event (window, node, handler, 1L, Changed, snapshot) in
  assert (Option.equal String.equal (Reconciler.dispatch reconciler event) (Some "latest"));
  assert (
    Option.is_none (Reconciler.dispatch reconciler (Press (window, node, handler, 1L))));
  let shared = View.column ~key:(Key.of_string_exn "left") [ view "" "left" ] in
  ignore (commit (View.column [ shared ]) : string Reconciler.update);
  let duplicate =
    View.column
      [ shared; View.column ~key:(Key.of_string_exn "right") [ view "" "right" ] ]
  in
  assert (
    Result.is_error (Reconciler.prepare reconciler ~theme:Theme.default (Some duplicate)));
  assert (Option.is_none (Reconciler.dispatch reconciler event));
  print_endline "EDITOR_OWNERSHIP_PASS";
  [%expect {| EDITOR_OWNERSHIP_PASS |}]
;;

let%expect_test
    "native snapshot decoding rejects invalid ranges and composing submissions"
  =
  let open Gpuio_protocol in
  let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let handler = Handler_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let snapshot : Wire.Editor.Snapshot.t =
    { revision = 0L
    ; text = "é"
    ; selection = { anchor = 2L; head = 2L }
    ; composition = None
    ; focused = true
    }
  in
  let decode kind snapshot =
    Bin_prot.Utils.bin_dump
      [%bin_writer: Wire.Event.t list]
      [ Editor_event (window, node, handler, 1L, kind, snapshot) ]
    |> Bigstring.to_string
    |> Wire.Event.decode
  in
  assert (Result.is_ok (decode Changed snapshot));
  assert (
    Result.is_error
      (decode Changed { snapshot with selection = { anchor = 1L; head = 1L } }));
  assert (
    Result.is_error
      (decode Changed { snapshot with composition = Some { anchor = 2L; head = 0L } }));
  assert (
    Result.is_error
      (decode Submitted { snapshot with composition = Some { anchor = 0L; head = 2L } }));
  assert (Result.is_error (decode Changed { snapshot with revision = -1L }));
  print_endline "EDITOR_SNAPSHOT_VALIDATION_PASS";
  [%expect {| EDITOR_SNAPSHOT_VALIDATION_PASS |}]
;;
