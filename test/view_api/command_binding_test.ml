open Core
open Gpuio
module B = Command_binding
module W = Gpuio_protocol.Command_binding_wire
module C = Gpuio_protocol.Command_wire

let ok = Or_error.ok_exn
let id text = Command.Id.of_string text |> ok
let config = B.Config.create [ Command (id "run"); Native_action Copy ] |> ok

let shortcut =
  Shortcut.create ~key:"k" ~modifiers:[ Primary ] ~priority:Override ()
  |> ok
  |> Shortcut.Expert.to_wire
;;

let observation : W.Observation.t =
  { epoch = 1L
  ; state =
      Ready
        [ Registry
            { enabled = true; candidates = [ { shortcut; disposition = Override } ] }
        ; Native_binding
            { strokes =
                [ { key = "k"; modifiers = 24L }; { key = "enter"; modifiers = 0L } ]
            ; disposition = Widget
            }
        ]
  }
;;

let hex writer value =
  let bytes = Bin_prot.Utils.bin_dump writer value in
  String.concat
    (List.init (Bigstring.length bytes) ~f:(fun i ->
       sprintf "%02x" (Char.to_int (Bigstring.get bytes i))))
;;

let fixture name =
  Eio_main.run (fun env ->
    Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip)
;;

let decode reader bytes =
  try
    let buffer = Bigstring.of_string bytes in
    let pos_ref = ref 0 in
    let value = reader buffer ~pos_ref in
    Option.some_if (!pos_ref = Bigstring.length buffer) value
  with
  | W.Invalid_wire_binding
  | Bin_prot.Common.Buffer_short
  | Bin_prot.Common.Read_error _
  | Gpuio_protocol.Generational_id.Invalid_wire_handle -> None
;;

let bytes writer value = Bin_prot.Utils.bin_dump writer value |> Bigstring.to_string

let%expect_test "independent bytes and typed target/native-stroke projection" =
  let wire = B.Expert.to_wire config in
  assert (
    String.equal (hex W.Config.bin_writer_t wire) (fixture "command-binding-config.hex"));
  assert (
    String.equal
      (hex W.Observation.bin_writer_t observation)
      (fixture "command-binding-observation.hex"));
  let decoded =
    decode W.Observation.bin_read_t (bytes W.Observation.bin_writer_t observation)
    |> Option.value_exn
  in
  let public = B.Expert.of_wire config decoded |> Option.value_exn in
  assert (Int64.equal (B.Observation.epoch public) 1L);
  (match B.Observation.state public with
   | Ready
       [ (Command command, Registry { enabled = true; candidates = [ candidate ] })
       ; (Native_action Copy, Native_binding { strokes; disposition = Widget })
       ] ->
     assert (Command.Id.equal command (id "run"));
     printf "%s\n" (Shortcut.format candidate.shortcut ~platform:Macos);
     List.iter strokes ~f:(fun stroke ->
       printf
         "%s | %s | %s\n"
         (B.Stroke.format stroke ~platform:Macos)
         (B.Stroke.format stroke ~platform:Linux)
         (B.Stroke.accessible_label stroke ~platform:Macos))
   | _ -> assert false);
  [%expect
    {|
    ⌘K
    fn⌘K | Fn+Super+K | Function + Command + K
    ⏎ | Enter | Enter
    |}]
;;

let%expect_test "query bounds and contexts validate before use" =
  assert (Result.is_error (B.Config.create []));
  assert (Result.is_error (B.Config.create [ Command (id "run"); Command (id "run") ]));
  assert (Result.is_error (B.Config.create [ Native_action Copy; Native_action Copy ]));
  let targets = List.init 64 ~f:(fun i -> B.Target.Command (id (Int.to_string i))) in
  assert (Result.is_ok (B.Config.create targets));
  assert (Result.is_error (B.Config.create (Command (id "extra") :: targets)));
  let native = B.Context.native_context "Input" |> ok in
  assert (Result.is_error (B.Config.create ~context:native [ Command (id "run") ]));
  assert (Result.is_error (B.Config.create ~context:B.Context.here [ Native_action Copy ]));
  assert (Result.is_ok (B.Config.create ~context:native [ Native_action Copy ]));
  List.iter
    [ ""; "  "; "\255"; "bad\000context"; String.make 1025 'x' ]
    ~f:(fun text -> assert (Result.is_error (B.Context.native_context text)));
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:7L ~generation:9L |> ok in
  let snapshot =
    Text_input.Expert.snapshot
      ~window
      ~node
      ~revision:(Text_input.Revision.of_int64 0L |> ok)
      ~text:"private draft"
      ~selection:(Text_input.Selection.create ~anchor:0 ~head:0 |> ok)
      ~composition:None
      ~focused:false
    |> ok
  in
  let editor =
    B.Config.create
      ~context:(B.Context.editor snapshot)
      [ Command (id "run"); Native_action Copy ]
    |> ok
  in
  assert (B.Expert.valid_window editor window);
  assert (
    not
      (B.Expert.valid_window
         editor
         (Gpuio_protocol.Window_id.create ~slot:0L ~generation:2L |> ok)));
  let wire = B.Expert.to_wire editor in
  assert (W.Context.equal wire.context (Editor (window, node)));
  assert (
    not
      (String.is_substring (bytes W.Config.bin_writer_t wire) ~substring:"private draft"));
  print_endline
    "64 ordered unique targets; context compatibility and bounded text; exact editor \
     lease without draft retention";
  [%expect
    {| 64 ordered unique targets; context compatibility and bounded text; exact editor lease without draft retention |}]
;;

let%expect_test "result provenance, disposition and context fencing" =
  let accepts config state =
    Option.is_some (B.Expert.of_wire config { epoch = 1L; state })
  in
  let command = B.Config.create [ Command (id "run") ] |> ok in
  let native = B.Config.create [ Native_action Copy ] |> ok in
  let scope = B.Config.create ~context:B.Context.here [ Command (id "run") ] |> ok in
  let entry disposition enabled =
    W.Entry.Registry { enabled; candidates = [ { shortcut; disposition } ] }
  in
  assert (accepts command (Ready [ Missing_command ]));
  assert (not (accepts command (Ready [ Native_unbound ])));
  assert (not (accepts native (Ready [ Missing_command ])));
  assert (not (accepts command (Ready [])));
  assert (accepts native (Ready [ Native_unsupported Sequence_too_long ]));
  List.iter
    [ W.Disposition.Override, true
    ; Unavailable Disabled, false
    ; Unavailable (Conflict "other"), true
    ]
    ~f:(fun (disposition, enabled) ->
      assert (accepts command (Ready [ entry disposition enabled ])));
  List.iter
    [ W.Disposition.Declared, true
    ; Widget, true
    ; Native_first, true
    ; Override, false
    ; Unavailable Disabled, true
    ; Unavailable (Conflict "run"), true
    ]
    ~f:(fun (disposition, enabled) ->
      assert (not (accepts command (Ready [ entry disposition enabled ]))));
  assert (accepts scope (Ready [ entry Declared false ]));
  assert (not (accepts scope (Ready [ entry Override true ])));
  assert (not (accepts command Context_gone));
  assert (not (accepts native Invalid_context));
  assert (accepts command Suspended && accepts native Epoch_exhausted);
  assert (Option.is_none (B.Expert.of_wire config { observation with epoch = 0L }));
  print_endline
    "target families, result count, scope-vs-focused disposition, priority, \
     disabled/conflict and context states fenced";
  [%expect
    {| target families, result count, scope-vs-focused disposition, priority, disabled/conflict and context states fenced |}]
;;

let%expect_test "bounded readers reject malformed framing and nested payloads" =
  let check reader writer value =
    let encoded = bytes writer value in
    for length = 0 to String.length encoded - 1 do
      assert (Option.is_none (decode reader (String.prefix encoded length)))
    done;
    assert (Option.is_none (decode reader (encoded ^ "\000")))
  in
  check W.Config.bin_read_t W.Config.bin_writer_t (B.Expert.to_wire config);
  check W.Observation.bin_read_t W.Observation.bin_writer_t observation;
  let rejects value =
    assert (
      Option.is_none
        (decode W.Observation.bin_read_t (bytes W.Observation.bin_writer_t value)))
  in
  let ready entry = { W.Observation.epoch = 1L; state = Ready [ entry ] } in
  List.iter [ -1L; 32L; Int64.max_value ] ~f:(fun modifiers ->
    rejects
      (ready
         (Native_binding { strokes = [ { key = "k"; modifiers } ]; disposition = Widget })));
  List.iter
    [ ""; "\255"; "\001"; "\127"; "\194\128"; "bad\000key"; String.make 257 'x' ]
    ~f:(fun key ->
      rejects
        (ready
           (Native_binding { strokes = [ { key; modifiers = 0L } ]; disposition = Widget })));
  List.iter [ 0; 9 ] ~f:(fun length ->
    rejects
      (ready
         (Native_binding
            { strokes =
                List.init length ~f:(fun _ -> { W.Stroke.key = "k"; modifiers = 0L })
            ; disposition = Widget
            })));
  let candidate = { W.Candidate.shortcut; disposition = Override } in
  rejects
    (ready
       (Registry { enabled = true; candidates = List.init 5 ~f:(fun _ -> candidate) }));
  rejects
    { W.Observation.epoch = 1L
    ; state = Ready (List.init 65 ~f:(fun _ -> W.Entry.Native_unbound))
    };
  List.iter [ "\001\000\255"; "\001\255"; "\001\000\001\003\255" ] ~f:(fun bytes ->
    assert (Option.is_none (decode W.Observation.bin_read_t bytes)));
  print_endline
    "independent frames truncated/trailing; unknown tags; bounded \
     candidates/entries/strokes/key bytes and modifier masks";
  [%expect
    {| independent frames truncated/trailing; unknown tags; bounded candidates/entries/strokes/key bytes and modifier masks |}]
;;

let%expect_test "independent disposition and state tags, exact sequence bounds" =
  let roundtrip value expected =
    assert (String.equal (hex W.Observation.bin_writer_t value) expected);
    let decoded =
      decode W.Observation.bin_read_t (bytes W.Observation.bin_writer_t value)
      |> Option.value_exn
    in
    assert (W.Observation.equal value decoded)
  in
  List.iter
    [ W.State.Suspended, "0101"
    ; Context_gone, "0102"
    ; Invalid_context, "0103"
    ; Epoch_exhausted, "0104"
    ; Capacity, "0105"
    ]
    ~f:(fun (state, expected) -> roundtrip { epoch = 1L; state } expected);
  List.iter
    [ W.Disposition.Declared, "00"
    ; Override, "01"
    ; Native_first, "02"
    ; Widget, "03"
    ; Unavailable Disabled, "0400"
    ; Unavailable Scope_blocked, "0401"
    ; Unavailable Native_unavailable, "0402"
    ; Unavailable Composition, "0403"
    ; Unavailable Text_input, "0404"
    ; Unavailable Native_navigation, "0405"
    ; Unavailable (Conflict "other"), "0406056f74686572"
    ]
    ~f:(fun (disposition, suffix) ->
      roundtrip
        { epoch = 1L
        ; state =
            Ready
              [ Native_binding
                  { strokes = [ { key = "k"; modifiers = 0L } ]; disposition }
              ]
        }
        ("0100010301016b00" ^ suffix));
  let exact : W.Observation.t =
    { epoch = Int64.max_value
    ; state =
        Ready
          (List.init 64 ~f:(fun _ ->
             W.Entry.Native_binding
               { strokes =
                   List.init 8 ~f:(fun _ -> { W.Stroke.key = "k"; modifiers = 31L })
               ; disposition = Widget
               }))
    }
  in
  assert (
    Option.is_some
      (decode W.Observation.bin_read_t (bytes W.Observation.bin_writer_t exact)));
  List.iter
    [ [ C.Shortcut_modifier.Primary; Primary ]; [ Super; Control ] ]
    ~f:(fun modifiers ->
      let bad : W.Observation.t =
        { epoch = 1L
        ; state =
            Ready
              [ Registry
                  { enabled = true
                  ; candidates =
                      [ { shortcut = { shortcut with modifiers }; disposition = Override }
                      ]
                  }
              ]
        }
      in
      assert (
        Option.is_none
          (decode W.Observation.bin_read_t (bytes W.Observation.bin_writer_t bad))));
  let native = B.Config.create [ Native_action Copy ] |> ok in
  assert (Option.is_some (B.Expert.of_wire native { epoch = 1L; state = Capacity }));
  let text_context = B.Context.native_context "Input mode=visible" |> ok in
  let hypothetical = B.Config.create ~context:text_context [ Native_action Copy ] |> ok in
  assert (
    Option.is_some (B.Expert.of_wire hypothetical { epoch = 1L; state = Invalid_context }));
  let public =
    B.Expert.of_wire
      native
      { epoch = 1L
      ; state =
          Ready
            [ Native_binding
                { strokes =
                    [ { key = "ß"; modifiers = 16L }
                    ; { key = "media-play"; modifiers = 0L }
                    ]
                ; disposition = Widget
                }
            ]
      }
    |> Option.value_exn
  in
  (match B.Observation.state public with
   | Ready [ (_, Native_binding { strokes; disposition = Widget }) ] ->
     List.iter strokes ~f:(fun stroke ->
       printf
         "%s | %s\n"
         (B.Stroke.format stroke ~platform:Macos)
         (B.Stroke.accessible_label stroke ~platform:Linux))
   | _ -> assert false);
  [%expect
    {| 
    fnSS | Function + SS
    Media-play | Media-play
  |}]
;;

let%expect_test "mounted binding owner fences config, epoch, callback and retirement" =
  let module Wire = Gpuio_protocol.Wire in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let handler generation = Gpuio_protocol.Handler_id.create ~slot:0L ~generation |> ok in
  let reconciler = Reconciler.create window in
  let config = B.Config.create ~context:B.Context.here [ Command (id "run") ] |> ok in
  let view config label =
    View.command_binding_scope ~config ~on_update:(fun _ -> label) [ View.text "child" ]
  in
  let prepare view = Reconciler.prepare reconciler ~theme:Theme.default view |> ok in
  let commit update = Reconciler.accept reconciler update |> ok in
  let event ?(generation = 1L) ?(revision = 1L) epoch state =
    Wire.Event.Command_binding_observed
      (window, node, handler generation, revision, { epoch; state })
  in
  let dispatch event = Reconciler.dispatch reconciler event in
  let ready = W.State.Ready [ Missing_command ] in
  commit (prepare (Some (view config "first")));
  assert (Option.equal String.equal (dispatch (event 1L ready)) (Some "first"));
  assert (Option.is_none (dispatch (event 1L ready)));
  assert (Option.is_none (dispatch (event 0L ready)));
  let callback = prepare (Some (view config "latest")) in
  assert (Option.is_none (Reconciler.message callback));
  commit callback;
  assert (Option.equal String.equal (dispatch (event 2L ready)) (Some "latest"));
  let changed_config = B.Config.create [ Command (id "run") ] |> ok in
  let changed = prepare (Some (view changed_config "changed")) in
  (match Reconciler.message changed with
   | Some
       (Apply
          { operations =
              [ Bind (bound, Some next); Set_command_binding (configured, Some _) ]
          ; _
          }) ->
     assert (Gpuio_protocol.Node_id.equal bound node);
     assert (Gpuio_protocol.Node_id.equal configured node);
     assert (Gpuio_protocol.Handler_id.equal next (handler 2L))
   | _ -> assert false);
  (* Merely preparing cannot retire an accepted callback or reset its epoch. *)
  assert (Option.equal String.equal (dispatch (event 3L ready)) (Some "latest"));
  commit changed;
  assert (Option.is_none (dispatch (event 4L ready)));
  assert (
    Option.equal
      String.equal
      (dispatch (event ~generation:2L ~revision:2L 1L ready))
      (Some "changed"));
  assert (
    Option.is_none
      (dispatch (event ~generation:2L ~revision:2L 2L (Ready [ Native_unbound ]))));
  assert (
    Option.equal
      String.equal
      (dispatch (event ~generation:2L ~revision:2L 2L ready))
      (Some "changed"));
  let plain = prepare (Some (View.column [ View.text "child" ])) in
  (match Reconciler.message plain with
   | Some (Apply { operations = [ Bind (_, None); Set_command_binding (_, None) ]; _ }) ->
     ()
   | _ -> assert false);
  commit plain;
  assert (Option.is_none (dispatch (event ~generation:2L ~revision:2L 3L ready)));
  commit (prepare None);
  print_endline
    "stable child and latest callback; config handler and monotonic epoch fencing; \
     prepare isolation; observer removal";
  [%expect
    {| stable child and latest callback; config handler and monotonic epoch fencing; prepare isolation; observer removal |}]
;;

let%expect_test "binding transport appends independently specified envelope tags" =
  let module Wire = Gpuio_protocol.Wire in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok in
  let event =
    Wire.Event.Command_binding_observed
      (window, node, handler, 1L, { epoch = 1L; state = Suspended })
  in
  let encoded = bytes [%bin_writer: Wire.Event.t list] [ event ] in
  assert (
    String.equal (hex [%bin_writer: Wire.Event.t list] [ event ]) "0142000100010001010101");
  assert (List.equal Wire.Event.equal (Wire.Event.decode encoded |> ok) [ event ]);
  let op = Wire.Op.Set_command_binding (node, Some (B.Expert.to_wire config)) in
  assert (String.equal (hex Wire.Op.bin_writer_t op) "3e0001010002000372756e0100");
  assert (
    String.equal (hex Wire.Op.bin_writer_t (Set_command_binding (node, None))) "3e000100");
  print_endline
    "operation 62; event 66; set and clear; bounded observation reader inside event \
     envelope";
  [%expect
    {| operation 62; event 66; set and clear; bounded observation reader inside event envelope |}]
;;
