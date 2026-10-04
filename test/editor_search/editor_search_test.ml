open Core
module S = Gpuio.Text_input.Search
module W = Gpuio_protocol.Editor_search_wire

let ok = Or_error.ok_exn

let observation : W.Snapshot.t =
  { stamp = { editor_revision = 7L; search_revision = 9L }
  ; activation_revision = 2L
  ; mode = Replace
  ; query = "é"
  ; case = Ascii_insensitive
  ; text_bytes = 8L
  ; match_count = 3L
  ; current = Some { index = 1L; byte_start = 3L; byte_end = 5L }
  ; can_replace = true
  }
;;

let window slot = Gpuio_protocol.Window_id.create ~slot ~generation:1L |> ok
let node generation = Gpuio_protocol.Node_id.create ~slot:1L ~generation |> ok
let observe value = S.Expert.snapshot_of_wire ~window:(window 0L) ~node:(node 1L) value

let%expect_test "search opt-in is multiline only and does not reseed the editor" =
  let open Gpuio in
  assert (
    Result.is_error
      (Text_input.Config.create ~mode:Single_line ~searchable:true ~label:"Draft" ()));
  let reconciler = Reconciler.create (window 0L) in
  let render searchable =
    let config =
      Text_input.Config.create ~mode:Multiline ~searchable ~label:"Draft" () |> ok
    in
    let view =
      View.text_input
        ~controller:(Key.of_string_exn "draft")
        ~config
        ~initial_text:"seed"
        ~on_event:(fun _ -> ())
        ()
      |> ok
    in
    let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
    let message = Reconciler.message update in
    Reconciler.accept reconciler update |> ok;
    match message with
    | Some (Gpuio_protocol.Wire.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  let id =
    List.find_map_exn (render false) ~f:(function
      | Gpuio_protocol.Wire.Op.Create (id, Textarea, _, _) -> Some id
      | _ -> None)
  in
  List.iter [ true; false ] ~f:(fun flag ->
    assert (
      List.equal
        Gpuio_protocol.Wire.Op.equal
        (render flag)
        [ Set_editor_searchable (id, flag) ]));
  print_endline "one retained multiline editor; configuration changes only";
  [%expect {| one retained multiline editor; configuration changes only |}]
;;

let%expect_test
    "literal queries are byte bounded and stamps retain exact editor ownership"
  =
  List.iter
    [ String.make 2049 'x'; "a\000b"; "\255" ]
    ~f:(fun query -> assert (Result.is_error (S.Query.create query)));
  assert (Result.is_ok (S.Query.create (String.concat (List.init 1024 ~f:(fun _ -> "é")))));
  assert (String.equal (S.Query.to_string S.Query.empty) "");
  let snapshot = observe observation |> ok in
  let current = S.Snapshot.current snapshot |> Option.value_exn in
  print_s
    [%sexp
      (S.Snapshot.query snapshot : S.Query.t)
    , (S.Snapshot.case snapshot : S.Case.t)
    , (S.Snapshot.match_count snapshot : int)
    , (S.Occurrence.index current : int)
    , (S.Occurrence.byte_start current : int)
    , (S.Occurrence.byte_end current : int)];
  let stamp = S.Snapshot.stamp snapshot in
  let close = S.Command.Close_and_focus snapshot in
  assert (Option.equal S.Stamp.equal (S.Expert.expected_stamp close) (Some stamp));
  assert (W.Command.equal (S.Expert.command_to_wire close) (Close_and_focus 2L));
  List.iter
    [ S.Expert.snapshot_of_wire ~window:(window 1L) ~node:(node 1L) observation
    ; S.Expert.snapshot_of_wire ~window:(window 0L) ~node:(node 2L) observation
    ; observe
        { observation with stamp = { observation.stamp with search_revision = 10L } }
    ; observe { observation with stamp = { observation.stamp with editor_revision = 8L } }
    ]
    ~f:(fun other -> assert (not (S.Stamp.equal stamp (S.Snapshot.stamp (ok other)))));
  [%expect {| ("\195\169" Ascii_insensitive 3 1 3 5) |}]
;;

let%expect_test "paired observation bytes and malformed metadata" =
  Eio_main.run (fun env ->
    let hex =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "editor-search-snapshot.hex")
      |> String.strip
    in
    let expected =
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    let encoded =
      Bin_prot.Utils.bin_dump [%bin_writer: W.Snapshot.t] observation
      |> Bigstring.to_string
    in
    assert (String.equal encoded expected);
    let data = Bigstring.of_string expected in
    let pos_ref = ref 0 in
    let decoded = W.Snapshot.bin_read_t data ~pos_ref in
    assert (Int.equal !pos_ref (String.length expected));
    assert (W.Snapshot.equal decoded observation);
    assert (Result.is_ok (observe decoded)));
  List.iter
    [ { observation with stamp = { observation.stamp with search_revision = -1L } }
    ; { observation with activation_revision = 10L }
    ; { observation with query = "\255" }
    ; { observation with query = "" }
    ; { observation with query = "abc" }
    ; { observation with text_bytes = 262_145L }
    ; { observation with match_count = Int64.max_value }
    ; { observation with match_count = 0L }
    ; { observation with current = None }
    ; { observation with current = Some { index = 3L; byte_start = 3L; byte_end = 5L } }
    ; { observation with current = Some { index = 1L; byte_start = 7L; byte_end = 9L } }
    ; { observation with mode = Closed }
    ]
    ~f:(fun value -> assert (Result.is_error (observe value)));
  assert (
    Result.is_ok
      (observe { observation with query = ""; match_count = 0L; current = None }));
  print_endline "paired metadata; invalid bounds/counts/stamps rejected";
  [%expect {| paired metadata; invalid bounds/counts/stamps rejected |}]
;;

let%expect_test "search request and reply envelopes match independent bytes" =
  let module B = Gpuio_protocol.Wire in
  Eio_main.run (fun env ->
    let load name = Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip in
    let decode_hex hex =
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    let window = window 0L
    and node = node 2L in
    let commands : W.Command.t list =
      [ Read
      ; Open false
      ; Close
      ; Set_query ("é", Sensitive)
      ; Next
      ; Previous
      ; Replace_current (observation.stamp, "é")
      ; Replace_all (observation.stamp, "")
      ; Close_and_focus observation.activation_revision
      ; Set_query_text "é"
      ; Set_case Sensitive
      ; Toggle_case
      ]
    in
    List.iteri
      (List.zip_exn commands (String.split_lines (load "editor-search-commands.hex")))
      ~f:(fun index (command, hex) ->
        let message =
          B.Message.Editor_command (Int64.of_int (7 + index), window, node, Search command)
        in
        assert (String.equal (B.Message.encode message |> ok) (decode_hex hex)));
    let op =
      B.Message.Apply
        { window
        ; base = 0L
        ; revision = 1L
        ; operations =
            [ Set_editor_searchable (node, false); Set_editor_searchable (node, true) ]
        }
    in
    assert (
      String.equal
        (B.Message.encode op |> ok)
        (decode_hex (load "editor-search-operation.hex")));
    let editor : B.Editor.Snapshot.t =
      { revision = 8L
      ; text = "é é é"
      ; selection = { anchor = 5L; head = 5L }
      ; composition = None
      ; focused = true
      }
    in
    let after =
      { observation with stamp = { editor_revision = 8L; search_revision = 10L } }
    in
    let events =
      [ B.Event.Editor_result (10L, window, node, Search_observed observation)
      ; Editor_result (13L, window, node, Search_replaced (editor, after, 1L))
      ; Editor_result (14L, window, node, Failed Stale_search)
      ]
    in
    let encode events =
      Bin_prot.Utils.bin_dump [%bin_writer: B.Event.t list] events |> Bigstring.to_string
    in
    let bytes = decode_hex (load "editor-search-events.hex") in
    assert (String.equal (encode events) bytes);
    assert (List.equal B.Event.equal (B.Event.decode bytes |> ok) events);
    for count = 0 to String.length bytes - 1 do
      assert (Result.is_error (B.Event.decode (String.prefix bytes count)))
    done;
    assert (Result.is_error (B.Event.decode (bytes ^ "\000")));
    List.iter
      [ B.Editor.Result.Search_replaced (editor, observation, 1L)
      ; Search_replaced (editor, after, -1L)
      ; Search_observed { observation with query = "\000" }
      ]
      ~f:(fun result ->
        assert (
          Result.is_error
            (B.Event.decode (encode [ B.Event.Editor_result (1L, window, node, result) ])))));
  print_endline "commands, replies and opt-in pair; malformed replies rejected";
  [%expect {| commands, replies and opt-in pair; malformed replies rejected |}]
;;

let%expect_test "search events respect leases, opt-in and closed tombstones" =
  let open Gpuio in
  let module B = Gpuio_protocol.Wire in
  let reconciler = Reconciler.create (window 0L) in
  let render searchable =
    let config =
      Text_input.Config.create ~mode:Multiline ~searchable ~label:"Search" () |> ok
    in
    let view =
      View.text_input ~controller:(Key.of_string_exn "search") ~config ~on_event:Fn.id ()
      |> ok
    in
    let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
    let message = Reconciler.message update in
    Reconciler.accept reconciler update |> ok;
    message
  in
  let id, handler =
    match render true with
    | Some (B.Message.Apply tx) ->
      List.find_map_exn tx.operations ~f:(function
        | Create (id, Textarea, _, Some handler) -> Some (id, handler)
        | _ -> None)
    | _ -> assert false
  in
  let event
        ?(window = window 0L)
        ?(node = id)
        ?(handler = handler)
        ?(revision = Reconciler.revision reconciler)
        value
    =
    B.Event.Editor_search_observed (window, node, handler, revision, value)
  in
  let accepts event = Option.is_some (Reconciler.dispatch reconciler event) in
  assert (accepts (event observation));
  List.iter
    [ event ~window:(window 1L) observation
    ; event ~node:(node 9L) observation
    ; event
        ~handler:(Gpuio_protocol.Handler_id.create ~slot:99L ~generation:1L |> ok)
        observation
    ; event ~revision:999L observation
    ; event ~revision:(-1L) observation
    ; event { observation with query = "\000" }
    ]
    ~f:(fun event -> assert (not (accepts event)));
  ignore (render false : B.Message.t option);
  assert (not (accepts (event observation)));
  assert (accepts (event { observation with mode = Closed; can_replace = false }));
  Reconciler.close reconciler;
  assert (not (accepts (event { observation with mode = Closed; can_replace = false })));
  print_endline "current search only; disable admits closed metadata; close drops all";
  [%expect {| current search only; disable admits closed metadata; close drops all |}]
;;

let%expect_test "search observation envelope has an independent appended tag" =
  let module B = Gpuio_protocol.Wire in
  let handler = Gpuio_protocol.Handler_id.create ~slot:3L ~generation:1L |> ok in
  let event revision search =
    B.Event.Editor_search_observed (window 0L, node 2L, handler, revision, search)
  in
  let encode values =
    Bin_prot.Utils.bin_dump [%bin_writer: B.Event.t list] values |> Bigstring.to_string
  in
  Eio_main.run (fun env ->
    let hex =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "editor-search-observation.hex")
      |> String.strip
    in
    let bytes =
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    assert (String.equal bytes (encode [ event 4L observation ]));
    assert (List.equal B.Event.equal (B.Event.decode bytes |> ok) [ event 4L observation ]);
    for n = 0 to String.length bytes - 1 do
      assert (Result.is_error (B.Event.decode (String.prefix bytes n)))
    done;
    assert (Result.is_error (B.Event.decode (bytes ^ "\000"))));
  List.iter
    [ event (-1L) observation; event 4L { observation with query = "\000" } ]
    ~f:(fun value -> assert (Result.is_error (B.Event.decode (encode [ value ]))));
  print_endline "tag 70; bounded metadata; truncated and invalid envelopes rejected";
  [%expect {| tag 70; bounded metadata; truncated and invalid envelopes rejected |}]
;;
