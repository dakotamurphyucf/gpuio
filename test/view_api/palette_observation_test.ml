open Core
open Gpuio
open Gpuio_protocol
module Snapshot = Command_palette.Snapshot

let ok = Or_error.ok_exn
let window = Window_id.create ~slot:0L ~generation:1L |> ok
let node = Node_id.create ~slot:1L ~generation:1L |> ok
let handler = Handler_id.create ~slot:2L ~generation:1L |> ok

let state : Palette_state_wire.t =
  { sequence = 3L
  ; query_revision = 2L
  ; query = "λ"
  ; composing = false
  ; selected = Some "run"
  ; matched_count = 2
  ; loading = false
  }
;;

let encode events =
  Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events |> Bigstring.to_string
;;

let%expect_test
    "palette snapshots have independent Rust fixture bytes and reject malformed state"
  =
  let message =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations = [ Set_palette_observed (node, true) ]
      }
  in
  assert (
    String.equal
      (Wire.Message.encode message |> ok)
      "\003\000\001\000\001\001\127\001\001\001");
  let event s = Wire.Event.Palette_observed (window, node, handler, 7L, s) in
  let bytes = encode [ event state ] in
  assert (
    String.equal
      bytes
      "\001\079\000\001\001\001\002\001\007\003\002\002\206\187\000\001\003run\002\000");
  assert (List.equal Wire.Event.equal (Wire.Event.decode bytes |> ok) [ event state ]);
  for length = 0 to String.length bytes - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix bytes length)))
  done;
  assert (Result.is_error (Wire.Event.decode (bytes ^ "\000")));
  List.iter
    [ { state with sequence = 0L }
    ; { state with query_revision = 4L }
    ; { state with query = "a\nb" }
    ; { state with query = String.make 4097 'x' }
    ; { state with selected = Some " " }
    ; { state with matched_count = 0 }
    ; { state with matched_count = 1025 }
    ]
    ~f:(fun state ->
      assert (Result.is_error (Wire.Event.decode (encode [ event state ]))));
  [%expect {| |}]
;;

let%expect_test "query identity distinguishes composition ABA and observer lifetimes" =
  let make ?(observer = handler) wire =
    Command_palette.Expert.snapshot_of_wire ~window ~node ~observer wire |> ok
  in
  let first = make state in
  assert (String.equal (Snapshot.query first) "λ");
  assert (not (Snapshot.composing first));
  assert (Snapshot.matched_count first = 2);
  assert (Snapshot.same_query first (make { state with sequence = 4L; selected = None }));
  assert (
    not
      (Snapshot.same_query first (make { state with sequence = 4L; query_revision = 4L })));
  assert (
    not
      (Snapshot.same_query
         first
         (make ~observer:(Handler_id.create ~slot:2L ~generation:2L |> ok) state)));
  [%expect {| |}]
;;

let%expect_test
    "palette observation dispatch fences accepted lifetimes and ordered snapshots"
  =
  let r = Reconciler.create window in
  let config = Command_palette.Config.create ~label:"Actions" ~commands:[] () |> ok in
  let view ?on_change () =
    View.command_palette ~config ?on_change ~on_dismiss:(fun _ -> "dismiss") ()
  in
  let prepare v = Reconciler.prepare r ~theme:Theme.default v |> ok in
  let accept p =
    Reconciler.accept r p |> ok;
    match Reconciler.message p with
    | Some (Wire.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  let ops =
    accept (prepare (Some (view ~on_change:(fun s -> "first:" ^ Snapshot.query s) ())))
  in
  let node, handler =
    List.find_map_exn ops ~f:(function
      | Wire.Op.Create (n, Command_palette, _, Some h) -> Some (n, h)
      | _ -> None)
  in
  assert (
    List.exists ops ~f:(function
      | Wire.Op.Set_palette_observed (_, true) -> true
      | _ -> false));
  let event ?(handler = handler) ?(revision = 1L) sequence =
    Wire.Event.Palette_observed
      ( window
      , node
      , handler
      , revision
      , { state with sequence; selected = None; matched_count = 0 } )
  in
  assert (Option.equal String.equal (Reconciler.dispatch r (event 3L)) (Some "first:λ"));
  assert (Option.is_none (Reconciler.dispatch r (event 3L)));
  let replacement =
    prepare (Some (view ~on_change:(fun s -> "latest:" ^ Snapshot.query s) ()))
  in
  assert (Option.is_none (Reconciler.message replacement));
  ignore (accept replacement : Wire.Op.t list);
  assert (Option.equal String.equal (Reconciler.dispatch r (event 4L)) (Some "latest:λ"));
  let detached = prepare (Some (view ())) in
  assert (Option.is_some (Reconciler.dispatch r (event 5L)));
  ignore (accept detached : Wire.Op.t list);
  assert (Option.is_none (Reconciler.dispatch r (event 6L)));
  let ops = accept (prepare (Some (view ~on_change:(fun _ -> "reattached") ()))) in
  let fresh =
    List.find_map_exn ops ~f:(function
      | Wire.Op.Bind (_, Some h) -> Some h
      | _ -> None)
  in
  assert (not (Handler_id.equal fresh handler));
  assert (Option.is_none (Reconciler.dispatch r (event 7L)));
  assert (
    Option.equal
      String.equal
      (Reconciler.dispatch r (event ~handler:fresh ~revision:3L 7L))
      (Some "reattached"));
  assert (Option.is_none (Reconciler.dispatch r (event ~handler:fresh ~revision:99L 8L)));
  ignore (accept (prepare None) : Wire.Op.t list);
  assert (Option.is_none (Reconciler.dispatch r (event ~handler:fresh ~revision:3L 8L)));
  [%expect {| |}]
;;

let%expect_test "palette command wire request/reply and malformed boundaries" =
  let command =
    Wire.Message.Palette_command (9L, window, node, handler, Some 2L, Set_query "λ")
  in
  assert (
    String.equal
      (Wire.Message.encode command |> ok)
      "\022\009\000\001\001\001\002\001\001\002\002\002\206\187");
  let reply state =
    Wire.Event.Palette_result (9L, window, node, handler, Applied state)
  in
  let bytes = encode [ reply state ] in
  assert (
    String.equal
      bytes
      "\001\080\009\000\001\001\001\002\001\000\003\002\002\206\187\000\001\003run\002\000");
  assert (List.equal Wire.Event.equal (Wire.Event.decode bytes |> ok) [ reply state ]);
  List.iter
    [ Some 0L, Palette_command_wire.Command.Focus
    ; None, Set_query "x\n"
    ; None, Set_query (String.make 4097 'x')
    ; None, Highlight (Some " ")
    ]
    ~f:(fun (expected, command) ->
      assert (
        Result.is_error
          (Wire.Message.encode
             (Palette_command (9L, window, node, handler, expected, command)))));
  assert (
    Result.is_error (Wire.Event.decode (encode [ reply { state with sequence = 0L } ])));
  for length = 0 to String.length bytes - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix bytes length)))
  done;
  [%expect {| |}]
;;

let%expect_test "loading is observed without changing query identity" =
  let make wire =
    Command_palette.Expert.snapshot_of_wire ~window ~node ~observer:handler wire |> ok
  in
  let initial = make state in
  let loading = make { state with sequence = 4L; loading = true } in
  assert (not (Snapshot.loading initial));
  assert (Snapshot.loading loading);
  assert (Snapshot.same_query initial loading);
  let message =
    Wire.Message.Palette_command (9L, window, node, handler, Some 2L, Set_loading true)
  in
  assert (
    String.equal
      (Wire.Message.encode message |> ok)
      "\022\009\000\001\001\001\002\001\001\002\004\001");
  let event =
    Wire.Event.Palette_observed (window, node, handler, 7L, { state with loading = true })
  in
  let bytes = encode [ event ] in
  assert (
    String.equal
      bytes
      "\001\079\000\001\001\001\002\001\007\003\002\002\206\187\000\001\003run\002\001");
  assert (List.equal Wire.Event.equal (Wire.Event.decode bytes |> ok) [ event ]);
  let invalid = String.drop_suffix bytes 1 ^ "\002" in
  assert (Result.is_error (Wire.Event.decode invalid));
  [%expect {| |}]
;;

let%expect_test
    "external result metadata is bounded and publication requires a native query fence"
  =
  let command = Command.Id.of_string "run" |> ok in
  let results = Command_palette.Results.create ~commands:[ command ] () |> ok in
  assert (
    Result.is_error (Command_palette.Results.create ~commands:[ command; command ] ()));
  let wire = Command_palette.Expert.command_to_wire (Publish_results results) in
  let message expected =
    Wire.Message.Palette_command (9L, window, node, handler, expected, wire)
  in
  assert (Result.is_error (Wire.Message.encode (message None)));
  assert (
    String.equal
      (Wire.Message.encode (message (Some 2L)) |> ok)
      "\022\009\000\001\001\001\002\001\001\002\005\001\003run\000");
  let config =
    Command_palette.Config.create
      ~label:"Remote"
      ~commands:[ command ]
      ~search:External
      ()
    |> ok
  in
  assert (
    Option.value_exn (Command_palette.Expert.options config)
    |> fun o ->
    Gpuio_protocol.Palette_options_wire.Search.equal
      o.Gpuio_protocol.Palette_options_wire.search
      External);
  [%expect {| |}]
;;
