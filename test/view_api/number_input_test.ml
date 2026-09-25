open Core
open Gpuio
module N = Number_input
module W = Gpuio_protocol.Number_input_wire

let ok = Or_error.ok_exn
let domain = Numeric.Domain.create ~min:(-2.) ~max:8. ~step:0.5 |> ok
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok

let config =
  N.Config.create
    ~domain
    ~label:"Temperature"
    ~placeholder:"e.g. 1.5"
    ~increment_label:"Higher"
    ~decrement_label:"Lower"
    ~step_controls:Stacked
    ~allow_empty:true
    ~read_only:true
    ()
  |> ok
;;

let snapshot =
  { W.Snapshot.revision = 7L
  ; domain = Numeric.Expert.to_wire domain
  ; draft = "é1e-"
  ; committed = Number 1.5
  ; selection = { anchor = 5L; head = 2L }
  ; composition = Some { anchor = 2L; head = 5L }
  ; focused = true
  }
;;

let settled =
  { snapshot with
    revision = 8L
  ; draft = "2.5"
  ; committed = Number 2.5
  ; selection = { anchor = 3L; head = 3L }
  ; composition = None
  }
;;

let encode writer value = Bin_prot.Utils.bin_dump writer value |> Bigstring.to_string

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let fixture fs name writer value =
  let bytes = encode writer value in
  assert (String.equal (hex bytes) (Eio.Path.load Eio.Path.(fs / name) |> String.strip))
;;

let%expect_test "numeric editor contracts match independent fixtures" =
  let replace_draft =
    N.Command.Replace_draft
      { text = "1e-"
      ; selection = Select (Text_input.Selection.create ~anchor:3 ~head:0 |> ok)
      ; undo = Reset
      ; if_revision = Some (N.Revision.of_int64 7L |> ok)
      }
  in
  let replace_value =
    N.Command.Replace_value
      { value = N.Value.of_float 4. |> ok
      ; selection = Preserve
      ; undo = Record
      ; if_revision = Some (N.Revision.of_int64 8L |> ok)
      }
  in
  Eio_main.run (fun env ->
    let fs = Eio.Stdenv.fs env in
    fixture
      fs
      "number-input-config.hex"
      W.Config.bin_writer_t
      (N.Expert.config_to_wire config);
    fixture fs "number-input-observed.hex" W.Event.bin_writer_t (Observed snapshot);
    fixture
      fs
      "number-input-committed.hex"
      W.Event.bin_writer_t
      (Committed (Stepper, settled));
    fixture
      fs
      "number-input-replace-draft.hex"
      W.Command.bin_writer_t
      (N.Expert.command_to_wire replace_draft);
    fixture
      fs
      "number-input-replace-value.hex"
      W.Command.bin_writer_t
      (N.Expert.command_to_wire replace_value);
    fixture
      fs
      "number-input-failed.hex"
      W.Response.bin_writer_t
      (Failed (Rejected Incomplete)));
  let observed = N.Expert.snapshot_of_wire ~window ~node snapshot |> ok in
  assert (Gpuio_protocol.Window_id.equal (N.Expert.window observed) window);
  assert (Gpuio_protocol.Node_id.equal (N.Expert.node observed) node);
  assert (Numeric.Domain.equal (N.Snapshot.domain observed) domain);
  print_s
    [%sexp
      (N.Snapshot.draft observed : string)
    , (N.Snapshot.classification observed : Numeric.Draft.t)
    , (N.Snapshot.committed observed : N.Value.t)
    , (N.Snapshot.selection observed : Text_input.Selection.t)
    , (N.Snapshot.composition observed : Text_input.Selection.t option)];
  [%expect
    {|
    ("\195\1691e-" (Invalid Syntax) (Number 1.5) ((anchor 5) (head 2))
     (((anchor 2) (head 5))))
    |}]
;;

let%expect_test "numeric public constructors protect finite values and bounded labels" =
  List.iter [ Float.nan; Float.infinity; Float.neg_infinity ] ~f:(fun n ->
    assert (Result.is_error (N.Value.of_float n)));
  assert (N.Value.equal (N.Value.of_float (-0.) |> ok) (N.Value.of_float 0. |> ok));
  assert (Result.is_error (N.Revision.of_int64 (-1L)));
  assert (
    Int64.equal
      (N.Revision.to_int64 (N.Revision.of_int64 Int64.max_value |> ok))
      Int64.max_value);
  assert (
    N.Config.allows_empty config
    && N.Config.is_read_only config
    && not (N.Config.is_disabled config));
  List.iter
    [ ""; " \t\011"; "bad\nlabel"; "bad\000"; "\255"; String.make 4097 'a' ]
    ~f:(fun label -> assert (Result.is_error (N.Config.create ~domain ~label ())));
  assert (Result.is_error (N.Config.create ~domain ~label:"Value" ~increment_label:"" ()));
  assert (
    Result.is_error (N.Config.create ~domain ~label:"Value" ~decrement_label:"\t" ()));
  assert (Result.is_error (N.Config.create ~domain ~label:"Value" ~placeholder:"a\rb" ()));
  List.iter
    [ ""; "é"; "-"; "1e-"; "\t"; String.make 4096 'a' ]
    ~f:(fun draft -> N.validate_draft draft |> ok);
  List.iter
    [ "\255"; "\000"; "\n"; "\r"; String.make 4097 'a' ]
    ~f:(fun draft -> assert (Result.is_error (N.validate_draft draft)));
  print_endline
    "finite values, nonnegative revisions, required labels, single-line UTF-8 and \
     4096-byte bound";
  [%expect
    {| finite values, nonnegative revisions, required labels, single-line UTF-8 and 4096-byte bound |}]
;;

let%expect_test "numeric snapshots preserve draft, direction and historical domain" =
  List.iter
    [ { snapshot with revision = -1L }
    ; { snapshot with selection = { anchor = 1L; head = 5L } }
    ; { snapshot with selection = { anchor = -1L; head = 5L } }
    ; { snapshot with selection = { anchor = 6L; head = 5L } }
    ; { snapshot with composition = Some { anchor = 5L; head = 2L } }
    ; { snapshot with composition = Some { anchor = 1L; head = 5L } }
    ; { snapshot with committed = Number 1.25 }
    ; { snapshot with committed = Number 9. }
    ; { snapshot with committed = Number Float.nan }
    ; { snapshot with draft = "\255" }
    ; { snapshot with draft = String.make 4097 'a' }
    ]
    ~f:(fun s -> assert (Result.is_error (N.Expert.snapshot_of_wire ~window ~node s)));
  let observed draft =
    N.Expert.snapshot_of_wire
      ~window
      ~node
      { snapshot with draft; selection = { anchor = 0L; head = 0L }; composition = None }
    |> ok
  in
  List.iter [ ""; "-"; "1e-"; "1."; "1e999"; "99"; "é" ] ~f:(fun draft ->
    print_s
      [%sexp
        (draft : string), (N.Snapshot.classification (observed draft) : Numeric.Draft.t)]);
  let past = observed "7" in
  let new_domain = Numeric.Domain.create ~min:0. ~max:3. ~step:1. |> ok in
  let current =
    N.Expert.snapshot_of_wire
      ~window
      ~node
      { settled with
        domain = Numeric.Expert.to_wire new_domain
      ; draft = "7"
      ; committed = Number 2.
      ; selection = { anchor = 1L; head = 1L }
      }
    |> ok
  in
  print_s
    [%sexp
      (N.Snapshot.classification past : Numeric.Draft.t)
    , (N.Snapshot.classification current : Numeric.Draft.t)];
  [%expect
    {|
    ("" Empty)
    (- Incomplete)
    (1e- Incomplete)
    (1. (Valid 1))
    (1e999 (Invalid Non_finite))
    (99 (Out_of_range 99))
    ("\195\169" (Invalid Syntax))
    ((Valid 7) (Out_of_range 7))
    |}]
;;

let%expect_test "numeric semantic events validate final states and rejection reasons" =
  let valid event = Result.is_ok (N.Expert.event_of_wire ~window ~node event) in
  assert (valid (Observed { settled with revision = 0L }));
  List.iter
    [ W.Event.Changed { settled with revision = 0L }
    ; Committed (Keyboard, snapshot)
    ; Cancelled (Escape, snapshot)
    ; Committed (Programmatic, { settled with committed = Number 3. })
    ; Committed (Stepper, { settled with revision = 0L })
    ]
    ~f:(fun event -> assert (not (valid event)));
  List.iter [ W.Source.Keyboard; Stepper; Accessibility; Programmatic ] ~f:(fun source ->
    assert (valid (Committed (source, settled))));
  List.iter [ W.Cancel_reason.Escape; Programmatic ] ~f:(fun reason ->
    assert (valid (Cancelled (reason, settled))));
  List.iter
    [ W.Rejection.Empty_required, "", false
    ; Incomplete, "1e-", false
    ; Syntax, "é", false
    ; Non_finite, "1e999", false
    ; Composing, "123", true
    ]
    ~f:(fun (reason, draft, composing) ->
      let s =
        { snapshot with
          draft
        ; selection = { anchor = 0L; head = 0L }
        ; composition = (if composing then Some { anchor = 0L; head = 0L } else None)
        }
      in
      assert (valid (Rejected (reason, s)));
      assert (not (valid (Rejected (reason, settled)))));
  print_endline
    "initial observation, semantic revisions, settled commit/cancel and exact rejection \
     reasons";
  [%expect
    {| initial observation, semantic revisions, settled commit/cancel and exact rejection reasons |}]
;;

let%expect_test "numeric command conversion preserves all variants and guards" =
  let selection = Text_input.Selection.create ~anchor:2 ~head:0 |> ok in
  List.iter
    [ N.Command.Select selection
    ; Focus
    ; Undo
    ; Redo
    ; Commit
    ; Cancel
    ; Step Increase
    ; Step Decrease
    ; Read_snapshot
    ]
    ~f:(fun command -> print_s [%sexp (N.Expert.command_to_wire command : W.Command.t)]);
  List.iter
    [ Text_input.Selection_policy.Start; End; Preserve; Select selection ]
    ~f:(fun selection ->
      List.iter [ Text_input.Undo_policy.Record; Reset ] ~f:(fun undo ->
        List.iter
          [ None
          ; Some (N.Revision.of_int64 0L |> ok)
          ; Some (N.Revision.of_int64 Int64.max_value |> ok)
          ]
          ~f:(fun if_revision ->
            List.iter
              [ N.Command.Replace_draft { text = "é"; selection; undo; if_revision }
              ; Replace_value { value = N.Value.empty; selection; undo; if_revision }
              ]
              ~f:(fun command ->
                assert (W.Command.valid (N.Expert.command_to_wire command))))));
  let replace text selection if_revision =
    W.Command.Replace_draft { text; selection; undo = Record; if_revision }
  in
  List.iter
    [ replace "é" (Select { anchor = 1L; head = 2L }) None
    ; replace "a\n" End None
    ; replace "\255" End None
    ; replace "12" End (Some (-1L))
    ; Select { anchor = 4097L; head = 0L }
    ; Replace_value
        { value = Number Float.nan; selection = End; undo = Reset; if_revision = None }
    ]
    ~f:(fun command -> assert (not (W.Command.valid command)));
  [%expect
    {|
    (Select ((anchor 2) (head 0)))
    Focus
    Undo
    Redo
    Commit
    Cancel
    (Step Increase)
    (Step Decrease)
    Read_snapshot |}]
;;
