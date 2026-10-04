open Core
module P = Gpuio.Choice_picker
module C = Gpuio.Choice

let ok = Or_error.ok_exn
let id name = C.Id.of_string name |> ok
let item ?disabled name = C.create ~id:(id name) ~label:"Same label" ?disabled () |> ok
let items values = C.Collection.create values |> ok

let group name values =
  P.Group.create ~id:(P.Group.Id.of_string name |> ok) ~label:"Section" (items values)
  |> ok
;;

let multiple names = P.Selection.multiple (List.map names ~f:id) |> ok
let show result = print_s [%sexp (Result.map result ~f:(fun _ -> ()) : unit Or_error.t)]

let%expect_test "group identity, choice identity and order are separate invariants" =
  let a = item "a" in
  let b = item ~disabled:true "b" in
  let groups = [ group "a" [ b ]; group "empty" []; group "last" [ a ] ] in
  let collection = P.Collection.grouped groups |> ok in
  print_s [%sexp (List.map (P.Collection.items collection) ~f:C.id : C.Id.t list)];
  print_s
    [%sexp
      (List.map (P.Collection.groups collection |> Option.value_exn) ~f:P.Group.id
       : P.Group.Id.t list)];
  show (P.Collection.grouped [ group "same" [ a ]; group "same" [ b ] ]);
  show (P.Collection.grouped [ group "first" [ a ]; group "second" [ a ] ]);
  show
    (P.Config.create ~label:"Picker" ~options:collection ~selected:(multiple [ "b" ]) ());
  show
    (P.Config.create
       ~label:"Picker"
       ~options:collection
       ~selected:(multiple [ "gone" ])
       ());
  show (P.Selection.multiple [ id "a"; id "a" ]);
  print_s
    [%sexp
      (P.Collection.equal (P.Collection.flat (items [])) (P.Collection.grouped [] |> ok)
       : bool)];
  [%expect
    {|
    (b a)
    (a empty last)
    (Error "duplicate picker group id: same")
    (Error "duplicate choice id: a")
    (Ok ())
    (Error "selected picker ID is absent: gone")
    (Error "duplicate picker selection ID")
    false
    |}]
;;

let%expect_test "requests reduce in order against current mode, catalog and policy" =
  let options =
    P.Collection.flat (items [ item "a"; item "b"; item ~disabled:true "locked" ])
  in
  let config ?(disabled = false) ?(clearable = false) selected =
    P.Config.create ~label:"Picker" ~options ~selected ~disabled ~clearable () |> ok
  in
  let start = multiple [ "locked"; "a" ] in
  let reduce selected request = P.Config.apply_request (config selected) request in
  let once = reduce start (Toggle (id "b")) in
  let twice = reduce once (Toggle (id "b")) in
  print_s [%sexp (once : P.Selection.t), (twice : P.Selection.t)];
  List.iter
    [ P.Request.Toggle (id "locked"); Select (id "b"); Toggle (id "gone"); Clear ]
    ~f:(fun request -> assert (P.Selection.equal start (reduce start request)));
  assert (
    P.Selection.equal
      start
      (P.Config.apply_request (config ~disabled:true ~clearable:true start) Clear));
  print_s
    [%sexp (P.Config.apply_request (config ~clearable:true start) Clear : P.Selection.t)];
  let single = P.Selection.single (Some (id "locked")) in
  assert (
    P.Selection.equal single (P.Config.apply_request (config single) (Toggle (id "b"))));
  print_s
    [%sexp (P.Config.apply_request (config single) (Select (id "b")) : P.Selection.t)];
  print_s
    [%sexp (P.Config.apply_request (config ~clearable:true single) Clear : P.Selection.t)];
  let reordered =
    P.Collection.grouped
      [ group "renamed" [ item "b"; item "a"; item ~disabled:true "locked" ] ]
    |> ok
  in
  let updated =
    P.Config.create ~label:"Changed label" ~options:reordered ~selected:once () |> ok
  in
  assert (P.Selection.equal twice (P.Config.apply_request updated (Toggle (id "b"))));
  print_endline "stale/disabled/wrong-mode requests do not change current selection";
  [%expect
    {|
    ((Multiple (locked a b)) (Multiple (locked a)))
    (Multiple ())
    (Single (b))
    (Single ())
    stale/disabled/wrong-mode requests do not change current selection
    |}]
;;

let%expect_test "group metadata participates in aggregate budgets before flattening" =
  show (P.Collection.grouped (List.init 256 ~f:(fun i -> group (Int.to_string i) [])));
  show (P.Collection.grouped (List.init 257 ~f:(fun i -> group (Int.to_string i) [])));
  let big_group n =
    P.Group.create
      ~id:(P.Group.Id.of_string (Int.to_string n) |> ok)
      ~label:(String.make 1024 'g')
      (items [])
    |> ok
  in
  show (P.Collection.grouped (List.init 256 ~f:big_group));
  let half start = List.init 2048 ~f:(fun i -> item (Int.to_string (start + i))) in
  let full = P.Collection.grouped [ group "one" (half 0); group "two" (half 2048) ] in
  show full;
  show
    (P.Collection.grouped
       [ group "one" (half 0); group "two" (half 2048); group "extra" [ item "extra" ] ]);
  show (P.Selection.multiple (List.init 4097 ~f:(fun i -> id (Int.to_string i))));
  let options = full |> ok in
  let selected =
    P.Selection.multiple (List.map (P.Collection.items options) ~f:C.id) |> ok
  in
  show (P.Config.create ~label:"Maximum" ~options ~selected ());
  [%expect
    {|
    (Ok ())
    (Error "picker exceeds 256 groups")
    (Error "picker exceeds 262144 catalog text bytes")
    (Ok ())
    (Error "picker exceeds 4096 items")
    (Error "picker selection exceeds 4096 IDs")
    (Ok ())
    |}]
;;

let%expect_test "text boundaries and inactive popup preferences remain explicit" =
  List.iter
    [ ""; "a\000"; "\255"; String.make 257 'a' ]
    ~f:(fun text -> show (P.Group.Id.of_string text));
  let options = P.Collection.flat (items []) in
  let selected = P.Selection.single None in
  let config =
    P.Config.create
      ~label:"Choose"
      ~options
      ~selected
      ~search:Application
      ~disabled:true
      ~open_state:(Controlled true)
      ()
    |> ok
  in
  print_s
    [%sexp
      (P.Config.search config : P.Search.t)
    , (P.Config.open_state config : P.Open_state.t)
    , (P.Config.placeholder config : string)];
  show (P.Config.create ~label:"Choose" ~options ~selected ~placeholder:"\255" ());
  show
    (P.Config.create
       ~label:"Choose"
       ~options
       ~selected
       ~search_placeholder:(String.make 1025 'x')
       ());
  [%expect
    {|
    (Error "picker group id must contain 1..256 bytes")
    (Error "picker group id must be UTF-8 without NUL")
    (Error "picker group id must be UTF-8 without NUL")
    (Error "picker group id must contain 1..256 bytes")
    (Application (Controlled true) "")
    (Error "picker placeholder must be UTF-8 without NUL")
    (Error "picker search placeholder must contain 0..1024 bytes")
    |}]
;;

let%expect_test "picker configuration bytes are independent and conversion revalidates" =
  let module W = Gpuio_protocol.Choice_picker_wire in
  let a = C.create ~id:(id "a") ~label:"Alpha" () |> ok in
  let b = C.create ~id:(id "b") ~label:"β" ~disabled:true () |> ok in
  let group =
    P.Group.create ~id:(P.Group.Id.of_string "g" |> ok) ~label:"Group" (items [ a; b ])
    |> ok
  in
  let grouped =
    P.Config.create
      ~label:"Pick"
      ~options:(P.Collection.grouped [ group ] |> ok)
      ~selected:(multiple [ "b"; "a" ])
      ~search:Substring
      ~clearable:true
      ~open_state:(Controlled true)
      ~placeholder:"Choose"
      ~search_placeholder:"Find"
      ()
    |> ok
  in
  let empty =
    P.Config.create
      ~label:"Pick"
      ~options:(P.Collection.flat (items []))
      ~selected:(P.Selection.single None)
      ~disabled:true
      ~search:Application
      ~open_state:(Managed { initially_open = true })
      ()
    |> ok
  in
  Eio_main.run (fun env ->
    List.iter
      [ grouped, "choice-picker-grouped.hex"; empty, "choice-picker-empty.hex" ]
      ~f:(fun (config, name) ->
        let wire = P.Expert.to_wire config in
        assert (W.Config.valid wire);
        let encoded = Bin_prot.Utils.bin_dump W.Config.bin_writer_t wire in
        let hex =
          Bigstring.to_string encoded
          |> String.to_list
          |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
          |> String.concat
        in
        assert (
          String.equal
            hex
            (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip));
        let pos_ref = ref 0 in
        let decoded = W.Config.bin_read_t encoded ~pos_ref in
        assert (!pos_ref = Bigstring.length encoded);
        assert (P.Config.equal config (P.Expert.of_wire decoded |> ok))));
  let raw = P.Expert.to_wire grouped in
  List.iter
    [ { raw with selected = Multiple [ "a"; "a" ] }
    ; { raw with selected = Single (Some "missing") }
    ; { raw with placeholder = "\255" }
    ; { raw with options = Grouped [] }
    ]
    ~f:(fun invalid ->
      assert (not (W.Config.valid invalid));
      assert (Result.is_error (P.Expert.of_wire invalid)));
  print_endline
    "grouped Unicode/multiple and flat empty/single fixtures; invalid conversions \
     rejected";
  [%expect
    {| grouped Unicode/multiple and flat empty/single fixtures; invalid conversions rejected |}]
;;

let%expect_test
    "picker event fixtures preserve intent versus visibility and reject malformed domain \
     values"
  =
  let module W = Gpuio_protocol.Choice_picker_wire in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let events : W.Event.t list =
    [ Selection_requested (Select "α", None)
    ; Selection_requested (Toggle "b", None)
    ; Selection_requested (Clear, None)
    ; Open_requested (true, Trigger)
    ; Open_requested (true, Keyboard)
    ; Open_requested (false, Escape)
    ; Open_requested (false, Outside_pointer)
    ; Open_requested (false, Focus_left)
    ; Open_requested (false, Selection)
    ; Visibility (Snapshot false)
    ; Visibility (Snapshot true)
    ; Visibility (Changed (true, Interaction Trigger))
    ; Visibility (Changed (false, Interaction Escape))
    ; Visibility (Changed (true, Application))
    ; Visibility (Changed (false, Unavailable))
    ; Visibility (Changed (false, Application))
    ]
  in
  Eio_main.run (fun env ->
    let lines =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "choice-picker-events.hex")
      |> String.split_lines
    in
    List.iter2_exn events lines ~f:(fun event expected ->
      assert (W.Event.valid event);
      let encoded = Bin_prot.Utils.bin_dump W.Event.bin_writer_t event in
      let hex =
        Bigstring.to_string encoded
        |> String.to_list
        |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
        |> String.concat
      in
      assert (String.equal hex expected);
      let pos_ref = ref 0 in
      let decoded = W.Event.bin_read_t encoded ~pos_ref in
      assert (!pos_ref = Bigstring.length encoded);
      assert (W.Event.equal event decoded);
      print_s
        [%sexp
          (P.Expert.event_of_wire decoded ~window ~query_node:None |> ok : P.Event.t)]));
  List.iter
    [ W.Event.Selection_requested (Select "", None)
    ; Selection_requested (Toggle "\255", None)
    ; Selection_requested (Select "a\000b", None)
    ; Selection_requested (Toggle (String.make 257 'x'), None)
    ; Open_requested (false, Keyboard)
    ; Open_requested (true, Escape)
    ; Open_requested (true, Outside_pointer)
    ; Open_requested (true, Focus_left)
    ; Open_requested (true, Selection)
    ; Visibility (Changed (true, Unavailable))
    ; Visibility (Changed (false, Interaction Keyboard))
    ]
    ~f:(fun invalid ->
      assert (not (W.Event.valid invalid));
      assert (Result.is_error (P.Expert.event_of_wire invalid ~window ~query_node:None)));
  [%expect
    {|
    (Selection_requested ((request (Select "\206\177")) (query ())))
    (Selection_requested ((request (Toggle b)) (query ())))
    (Selection_requested ((request Clear) (query ())))
    (Open_requested true Trigger)
    (Open_requested true Keyboard)
    (Open_requested false Escape)
    (Open_requested false Outside_pointer)
    (Open_requested false Focus_left)
    (Open_requested false Selection)
    (Visibility (Snapshot false))
    (Visibility (Snapshot true))
    (Visibility (Changed true (Interaction Trigger)))
    (Visibility (Changed false (Interaction Escape)))
    (Visibility (Changed true Application))
    (Visibility (Changed false Unavailable))
    (Visibility (Changed false Application))
    |}]
;;

let%expect_test "query events retain exact editor leases and validate snapshots" =
  let module W = Gpuio_protocol.Choice_picker_wire in
  let module E = Gpuio_protocol.Editor_wire in
  let module N = Gpuio_protocol.Node_id in
  let window = Gpuio_protocol.Window_id.create ~slot:4L ~generation:1L |> ok in
  let node = N.create ~slot:7L ~generation:2L |> ok in
  let replacement = N.create ~slot:7L ~generation:3L |> ok in
  let query : W.Query.t =
    { node
    ; snapshot =
        { revision = 9L
        ; text = "λx"
        ; selection = { anchor = 3L; head = 3L }
        ; composition = None
        ; focused = true
        }
    }
  in
  let composing =
    { query with
      snapshot = { query.snapshot with composition = Some { anchor = 0L; head = 2L } }
    }
  in
  let events : W.Event.t list =
    [ Selection_requested (Select "α", Some query)
    ; Query_changed query
    ; Query_changed composing
    ; Selection_requested (Clear, Some query)
    ]
  in
  Eio_main.run (fun env ->
    let lines =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "choice-picker-query-events.hex")
      |> String.split_lines
    in
    List.iter2_exn events lines ~f:(fun event expected ->
      assert (W.Event.valid event);
      let encoded = Bin_prot.Utils.bin_dump W.Event.bin_writer_t event in
      let hex =
        Bigstring.to_string encoded
        |> String.to_list
        |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
        |> String.concat
      in
      assert (String.equal hex expected);
      let pos_ref = ref 0 in
      let decoded = W.Event.bin_read_t encoded ~pos_ref in
      assert (W.Event.equal event decoded);
      assert (!pos_ref = Bigstring.length encoded);
      let domain = P.Expert.event_of_wire decoded ~window ~query_node:(Some node) |> ok in
      let snapshot =
        match domain with
        | Selection_requested request ->
          print_s [%sexp (P.Selection_request.request request : P.Request.t)];
          P.Selection_request.query request |> Option.value_exn
        | Query_changed query -> query
        | Open_requested _ | Visibility _ -> assert false
      in
      assert (N.equal (Gpuio.Text_input.Expert.node snapshot) node);
      assert (
        Gpuio_protocol.Window_id.equal (Gpuio.Text_input.Expert.window snapshot) window);
      assert (String.equal (Gpuio.Text_input.Snapshot.text snapshot) "λx");
      assert (
        Int64.equal
          (Gpuio.Text_input.Snapshot.revision snapshot
           |> Gpuio.Text_input.Revision.to_int64)
          9L);
      assert (
        Result.is_error
          (P.Expert.event_of_wire decoded ~window ~query_node:(Some replacement)));
      assert (Result.is_error (P.Expert.event_of_wire decoded ~window ~query_node:None))));
  assert (
    Result.is_error
      (P.Expert.event_of_wire
         (Selection_requested (Clear, None))
         ~window
         ~query_node:(Some node)));
  assert (not (W.Event.valid (Selection_requested (Clear, Some composing))));
  let invalid : E.Snapshot.t list =
    [ { query.snapshot with text = "x\ny" }
    ; { query.snapshot with text = "x\ry" }
    ; { query.snapshot with text = "x\000y" }
    ; { query.snapshot with text = "\255xx" }
    ; { query.snapshot with text = String.make (W.max_query_bytes + 1) 'x' }
    ; { query.snapshot with revision = -1L }
    ; { query.snapshot with selection = { anchor = 0L; head = 1L } }
    ; { query.snapshot with selection = { anchor = 0L; head = 4L } }
    ; { query.snapshot with composition = Some { anchor = 2L; head = 0L } }
    ; { query.snapshot with composition = Some { anchor = 0L; head = 1L } }
    ]
  in
  List.iter invalid ~f:(fun snapshot ->
    let invalid = W.Event.Query_changed { query with snapshot } in
    assert (not (W.Event.valid invalid));
    assert (
      Result.is_error (P.Expert.event_of_wire invalid ~window ~query_node:(Some node))));
  print_endline
    "query identity/revision preserved; remount, missing and malformed snapshots rejected";
  [%expect
    {|
    (Select "\206\177")
    Clear
    query identity/revision preserved; remount, missing and malformed snapshots rejected
    |}]
;;

let%expect_test
    "picker descriptions validate slot identity, query mode and custom indicators"
  =
  let config =
    P.Config.create
      ~label:"Pick"
      ~options:(P.Collection.grouped [ group "g" [ item "a"; item "b" ] ] |> ok)
      ~selected:(multiple [ "b" ])
      ~search:Substring
      ()
    |> ok
  in
  let query =
    P.Query.create ~controller:(Gpuio.Key.of_string_exn "query") ~initial_text:"λ" ()
    |> ok
  in
  let a = P.Option_content.create ~checkmark:Custom "rich a" in
  let b = P.Option_content.create "rich b" in
  let description =
    P.Description.create
      ~config
      ~query
      ~trigger:"trigger"
      ~empty:"empty"
      ~footer:"footer"
      ~groups:[ P.Group.Id.of_string "g" |> ok, "header" ]
      ~options:[ id "a", a; id "b", b ]
      ()
    |> ok
  in
  assert (P.Config.equal (P.Description.config description) config);
  assert (P.Query.equal (P.Description.query description |> Option.value_exn) query);
  assert (String.equal (P.Query.initial_text query) "λ");
  assert (String.equal (Gpuio.Key.to_string (P.Query.controller query)) "query");
  assert (Option.equal String.equal (P.Description.trigger description) (Some "trigger"));
  assert (Option.equal String.equal (P.Description.empty description) (Some "empty"));
  assert (Option.equal String.equal (P.Description.footer description) (Some "footer"));
  assert (List.length (P.Description.groups description) = 1);
  assert (String.equal (P.Option_content.content a) "rich a");
  List.iter (P.Description.options description) ~f:(fun (id, content) ->
    print_s [%sexp (id : C.Id.t), (P.Option_content.checkmark content : P.Checkmark.t)]);
  List.iter
    [ P.Description.create ~config ()
    ; P.Description.create ~config ~query ~options:[ id "a", a; id "a", b ] ()
    ; P.Description.create ~config ~query ~options:[ id "missing", a ] ()
    ; P.Description.create
        ~config
        ~query
        ~groups:[ P.Group.Id.of_string "missing" |> ok, "bad" ]
        ()
    ; P.Description.create
        ~config
        ~query
        ~groups:
          [ P.Group.Id.of_string "g" |> ok, "x"; P.Group.Id.of_string "g" |> ok, "y" ]
        ()
    ]
    ~f:(fun value -> assert (Result.is_error value));
  let plain =
    P.Config.create
      ~label:"Pick"
      ~options:(P.Collection.flat (items []))
      ~selected:(P.Selection.single None)
      ()
    |> ok
  in
  assert (Result.is_ok (P.Description.create ~config:plain ()));
  assert (Result.is_error (P.Description.create ~config:plain ~query ()));
  assert (
    Result.is_error
      (P.Query.create ~controller:(Gpuio.Key.of_string_exn "q") ~initial_text:"x\ny" ()));
  print_endline "duplicate/absent slots and mismatched search placement rejected";
  [%expect
    {|
    (a Custom)
    (b Native)
    duplicate/absent slots and mismatched search placement rejected
    |}]
;;

let%expect_test "picker presentation bytes preserve geometry, styles and slot order" =
  let module W = Gpuio_protocol.Wire.Choice_picker_presentation in
  let module S = Gpuio.Style in
  let config =
    P.Config.create
      ~label:"Pick"
      ~options:
        (P.Collection.grouped
           [ P.Group.create
               ~id:(P.Group.Id.of_string "g" |> ok)
               ~label:"Group"
               (items
                  [ C.create ~id:(id "a") ~label:"Alpha" () |> ok
                  ; C.create ~id:(id "b") ~label:"β" ~disabled:true () |> ok
                  ])
             |> ok
           ]
         |> ok)
      ~selected:(multiple [ "b"; "a" ])
      ~search:Substring
      ~clearable:true
      ~open_state:(Controlled true)
      ~placeholder:"Choose"
      ~search_placeholder:"Find"
      ()
    |> ok
  in
  let appearance =
    P.Appearance.create
      ~max_height:240.
      ~empty_label:"No matches"
      ~popup_style:(S.create_exn [ Opacity 0.8 ])
      ~option_style:(S.with_state_exn S.empty Selected [ Opacity 0.9 ])
      ~header_style:(S.create_exn [ Opacity 0.6 ])
      ~empty_style:(S.create_exn [ Opacity 0.5 ])
      ()
    |> ok
  in
  let query = P.Query.create ~controller:(Gpuio.Key.of_string_exn "q") () |> ok in
  let description =
    P.Description.create
      ~config
      ~appearance
      ~query
      ~trigger:()
      ~empty:()
      ~footer:()
      ~groups:[ P.Group.Id.of_string "g" |> ok, () ]
      ~options:
        [ id "a", P.Option_content.create ~checkmark:Custom ()
        ; id "b", P.Option_content.create ()
        ]
      ()
    |> ok
  in
  assert (P.Appearance.equal (P.Description.appearance description) appearance);
  let wire = P.Expert.description_to_wire description ~theme:Gpuio.Theme.default |> ok in
  assert (Gpuio_protocol.Choice_picker_wire.valid_slots wire.config wire.slots);
  let encoded = Bin_prot.Utils.bin_dump W.bin_writer_t wire in
  let hex =
    Bigstring.to_string encoded
    |> String.to_list
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    assert (
      String.equal
        hex
        (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "choice-picker-presentation.hex")
         |> String.strip)));
  let pos_ref = ref 0 in
  assert (W.equal wire (W.bin_read_t encoded ~pos_ref));
  assert (!pos_ref = Bigstring.length encoded);
  let module Wire = Gpuio_protocol.Wire in
  let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:2L |> ok in
  let handler = Gpuio_protocol.Handler_id.create ~slot:3L ~generation:4L |> ok in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let message =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Choice_picker, "", Some handler)
          ; Set_choice_picker (node, wire)
          ]
      }
  in
  let bytes =
    Bin_prot.Utils.bin_dump Wire.Message.bin_writer_t message |> Bigstring.to_string
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    assert (
      String.equal
        hex
        (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "choice-picker-operation.hex")
         |> String.strip)));
  List.iter
    [ wire.slots @ [ Trigger ]
    ; [ Query; Group "missing" ]
    ; [ Query; Option ("missing", Native) ]
    ; []
    ]
    ~f:(fun slots ->
      assert (not (Gpuio_protocol.Choice_picker_wire.valid_slots wire.config slots)));
  print_endline "independent config/geometry/four style parts/seven slot roles match";
  [%expect {| independent config/geometry/four style parts/seven slot roles match |}]
;;

let%expect_test "picker appearance bounds geometry and rejects structural part styles" =
  let module S = Gpuio.Style in
  List.iter
    [ P.Appearance.create ~max_height:0. ()
    ; P.Appearance.create ~popup_width:Float.nan ()
    ; P.Appearance.create ~estimated_row_height:0.5 ()
    ; P.Appearance.create ~overscan:4097. ()
    ; P.Appearance.create
        ~header_style:(S.create_exn [ Width (Gpuio.Length.px_exn 20.) ])
        ()
    ; P.Appearance.create
        ~header_style:(S.with_state_exn S.empty Hovered [ Opacity 0.5 ])
        ()
    ; P.Appearance.create ~option_style:(S.create_exn [ Disabled true ]) ()
    ]
    ~f:(fun value -> assert (Result.is_error value));
  assert (Result.is_ok (P.Appearance.create ~estimated_row_height:1. ~overscan:0. ()));
  print_endline "invalid geometry, header interaction and layout/input overrides rejected";
  [%expect {| invalid geometry, header interaction and layout/input overrides rejected |}]
;;

let%expect_test "picker bridge envelopes preserve identity and validate nested events" =
  let module Wire = Gpuio_protocol.Wire in
  let module W = Gpuio_protocol.Choice_picker_wire in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:2L |> ok in
  let handler = Gpuio_protocol.Handler_id.create ~slot:3L ~generation:4L |> ok in
  let child = Gpuio_protocol.Node_id.create ~slot:7L ~generation:2L |> ok in
  let query : W.Query.t =
    { node = child
    ; snapshot =
        { revision = 9L
        ; text = "λx"
        ; selection = { anchor = 3L; head = 3L }
        ; composition = None
        ; focused = true
        }
    }
  in
  let composing =
    { query with
      snapshot = { query.snapshot with composition = Some { anchor = 0L; head = 2L } }
    }
  in
  let events : W.Event.t list =
    [ Selection_requested (Select "α", Some query)
    ; Query_changed query
    ; Query_changed composing
    ; Selection_requested (Clear, Some query)
    ]
  in
  let encode event revision =
    let event = Wire.Event.Choice_picker_event (window, node, handler, revision, event) in
    ( event
    , Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] [ event ]
      |> Bigstring.to_string )
  in
  Eio_main.run (fun env ->
    let lines =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "choice-picker-envelope.hex")
      |> String.split_lines
    in
    List.iter2_exn events lines ~f:(fun event expected ->
      let event, bytes = encode event 5L in
      let hex =
        String.to_list bytes
        |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
        |> String.concat
      in
      assert (String.equal hex expected);
      assert (List.equal Wire.Event.equal (Wire.Event.decode bytes |> ok) [ event ]);
      for length = 0 to String.length bytes - 1 do
        assert (Result.is_error (Wire.Event.decode (String.prefix bytes length)))
      done;
      assert (Result.is_error (Wire.Event.decode (bytes ^ "\000")))));
  List.iter
    [ W.Event.Open_requested (true, Escape)
    ; Selection_requested (Clear, Some composing)
    ; Query_changed
        { query with
          snapshot = { query.snapshot with selection = { anchor = 1L; head = 3L } }
        }
    ; Query_changed { query with snapshot = { query.snapshot with text = "x\ny" } }
    ]
    ~f:(fun event ->
      let _, bytes = encode event 5L in
      assert (Result.is_error (Wire.Event.decode bytes)));
  let _, bytes = encode (Visibility (Snapshot false)) (-1L) in
  assert (Result.is_error (Wire.Event.decode bytes));
  print_endline "event69 envelopes, truncation, composition, UTF-8 and direction checked";
  [%expect {| event69 envelopes, truncation, composition, UTF-8 and direction checked |}]
;;
