open Core
open Gpuio
module S = Slider
module W = Gpuio_protocol.Slider_wire
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let domain = Numeric.Domain.create ~min:(-2.) ~max:8. ~step:0.5 |> ok

let config =
  S.Config.create
    ~domain
    ~label:"Temperature"
    ~lower_label:"Minimum"
    ~upper_label:"Maximum"
    ~axis:Vertical
    ()
  |> ok
;;

let range lower upper = S.Value.range ~lower ~upper |> ok

let snapshot =
  { W.Snapshot.revision = 7L
  ; value = Range { lower = 1.5; upper = 7. }
  ; committed = Range { lower = 2.; upper = 7. }
  ; dragging = Some Lower
  }
;;

let%expect_test "slider contracts validate labels, logarithmic domain and ordered values" =
  List.iter [ Float.nan; Float.infinity ] ~f:(fun v ->
    assert (Result.is_error (S.Value.single v)));
  assert (Result.is_error (S.Value.range ~lower:3. ~upper:2.));
  List.iter
    [ ""; " \t\011"; "bad\000label"; "\255"; String.make 4097 'a' ]
    ~f:(fun label -> assert (Result.is_error (S.Config.create ~domain ~label ())));
  assert (Result.is_error (S.Config.create ~domain ~label:"Log" ~scale:Logarithmic ()));
  assert (Numeric.Domain.equal domain (S.Config.domain config));
  assert (Result.is_error (S.Revision.of_int64 (-1L)));
  let observed = S.Expert.snapshot_of_wire ~window ~node snapshot |> ok in
  assert (Gpuio_protocol.Window_id.equal window (S.Expert.window observed));
  assert (Gpuio_protocol.Node_id.equal node (S.Expert.node observed));
  assert (S.Value.equal (S.Snapshot.value observed) (range 1.5 7.));
  assert (S.Value.equal (S.Snapshot.committed observed) (range 2. 7.));
  assert (Option.equal S.Thumb.equal (S.Snapshot.dragging observed) (Some Lower));
  print_endline
    "finite ordered values, bounded labels, positive logarithmic minimum, distinct \
     preview/commit";
  [%expect
    {| finite ordered values, bounded labels, positive logarithmic minimum, distinct preview/commit |}]
;;

let%expect_test "malformed snapshots and lifecycle phases cannot enter public types" =
  List.iter
    [ { snapshot with revision = -1L }
    ; { snapshot with revision = 0L }
    ; { snapshot with dragging = None }
    ; { snapshot with dragging = Some Single }
    ; { snapshot with committed = Single 1. }
    ; { snapshot with value = Range { lower = 1.5; upper = 6. } }
    ]
    ~f:(fun s -> assert (Result.is_error (S.Expert.snapshot_of_wire ~window ~node s)));
  List.iter
    [ W.Event.Drag_started snapshot
    ; Committed (Pointer, snapshot)
    ; Cancelled (Escape, snapshot)
    ]
    ~f:(fun e -> assert (Result.is_error (S.Expert.event_of_wire ~window ~node e)));
  ignore (S.Expert.event_of_wire ~window ~node (Preview snapshot) |> ok : S.Event.t);
  print_endline "revision, mode, stationary thumb and lifecycle invariants enforced";
  [%expect {| revision, mode, stationary thumb and lifecycle invariants enforced |}]
;;

let hex s =
  String.to_list s
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "independent slider config, preview and guarded replacement fixtures" =
  let command =
    S.Command.Replace
      { value = range 0. 6.; if_revision = Some (S.Revision.of_int64 7L |> ok) }
  in
  let values =
    [ ( "slider-config.hex"
      , Bin_prot.Utils.bin_dump W.Config.bin_writer_t (S.Expert.config_to_wire config) )
    ; ( "slider-preview.hex"
      , Bin_prot.Utils.bin_dump W.Event.bin_writer_t (Preview snapshot) )
    ; ( "slider-replace.hex"
      , Bin_prot.Utils.bin_dump W.Command.bin_writer_t (S.Expert.command_to_wire command)
      )
    ]
  in
  Eio_main.run (fun env ->
    List.iter values ~f:(fun (name, bytes) ->
      assert (
        String.equal
          (hex (Bigstring.to_string bytes))
          (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip))));
  print_endline "three independently authored OCaml/Rust slider fixtures agree";
  [%expect {| three independently authored OCaml/Rust slider fixtures agree |}]
;;

let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok

let events_bytes events =
  Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events |> Bigstring.to_string
;;

let%expect_test "slider retained request and event envelopes have independent fixtures" =
  let request =
    Wire.Message.encode
      (Apply
         { window
         ; base = 0L
         ; revision = 1L
         ; operations =
             [ Create (node, Slider, "", Some handler)
             ; Set_slider
                 (node, S.Expert.config_to_wire config, Range { lower = 2.; upper = 7. })
             ; Set_root (Some node)
             ]
         })
    |> ok
  in
  let events =
    [ Wire.Event.Slider_event (window, node, handler, 1L, Preview snapshot) ]
  in
  let bytes = events_bytes events in
  Eio_main.run (fun env ->
    List.iter
      [ "slider-request.hex", request; "slider-events.hex", bytes ]
      ~f:(fun (name, bytes) ->
        assert (
          String.equal
            (hex bytes)
            (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip))));
  assert (List.equal Wire.Event.equal events (Wire.Event.decode bytes |> ok));
  for length = 0 to String.length bytes - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix bytes length)))
  done;
  assert (Result.is_error (Wire.Event.decode (bytes ^ "\000")));
  List.iter
    [ -1L, W.Event.Preview snapshot
    ; 1L, Preview { snapshot with revision = 0L }
    ; 1L, Preview { snapshot with dragging = None }
    ; 1L, Committed (Pointer, snapshot)
    ]
    ~f:(fun (revision, event) ->
      assert (
        Result.is_error
          (Wire.Event.decode
             (events_bytes [ Slider_event (window, node, handler, revision, event) ]))));
  print_endline
    "independent envelopes, strict consumption and invalid lifecycle rejection";
  [%expect
    {| independent envelopes, strict consumption and invalid lifecycle rejection |}]
;;

let%expect_test
    "slider observers retain identity, latest callbacks and native revision fences"
  =
  let reconciler = Reconciler.create window in
  let controller = Key.of_string_exn "range" in
  let view ?(controller = controller) ?(config = config) ?(initial = range 2. 7.) callback
    =
    View.slider ~controller ~config ~initial ~on_event:callback ()
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
      | Wire.Op.Create (node, Slider, "", Some handler) -> Some (node, handler)
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
    Wire.Event.Slider_event
      (window, node, handler, tree_revision, Preview { snapshot with revision })
  in
  let dispatch = Reconciler.dispatch reconciler in
  assert (Option.equal Int.equal (dispatch (event 1L)) (Some 1));
  assert (List.is_empty (commit (Some (view (fun _ -> 2)))));
  assert (Option.is_none (dispatch (event 1L)));
  assert (Option.equal Int.equal (dispatch (event 2L)) (Some 2));
  List.iter
    [ event ~tree_revision:99L 3L
    ; event ~tree_revision:(-1L) 3L
    ; event ~window:(Gpuio_protocol.Window_id.create ~slot:0L ~generation:2L |> ok) 3L
    ; event ~node:(Gpuio_protocol.Node_id.create ~slot:0L ~generation:2L |> ok) 3L
    ; event ~handler:(Gpuio_protocol.Handler_id.create ~slot:0L ~generation:2L |> ok) 3L
    ; Wire.Event.Slider_event
        ( window
        , node
        , handler
        , 1L
        , Preview { snapshot with revision = 99L; dragging = None } )
    ; Wire.Event.Slider_event
        ( window
        , node
        , handler
        , 1L
        , Observed
            { revision = 99L; value = Single 1.; committed = Single 1.; dragging = None }
        )
    ]
    ~f:(fun event -> assert (Option.is_none (dispatch event)));
  (* Invalid messages must not poison the revision fence. *)
  assert (Option.equal Int.equal (dispatch (event 3L)) (Some 2));
  let new_config =
    S.Config.create
      ~domain:(Numeric.Domain.create ~min:0. ~max:1. ~step:0.1 |> ok)
      ~label:"Updated"
      ~disabled:true
      ~read_only:true
      ()
    |> ok
  in
  let ops = commit (Some (view ~config:new_config (fun _ -> 3))) in
  assert (
    List.for_all ops ~f:(function
      | Wire.Op.Set_slider _ -> true
      | _ -> false));
  let idle =
    { snapshot with revision = 4L; value = snapshot.committed; dragging = None }
  in
  (* A queued cancellation belongs to the previous domain, even after disabling. *)
  assert (
    Option.equal
      Int.equal
      (dispatch
         (Slider_event (window, node, handler, 1L, Cancelled (Configuration_changed, idle))))
      (Some 3));
  assert (
    Result.is_error
      (Reconciler.prepare
         reconciler
         ~theme:Theme.default
         (Some (view ~initial:(S.Value.single 0. |> ok) (fun _ -> 99)))));
  assert (Option.equal Int.equal (dispatch (event 5L)) (Some 3));
  ignore (commit None : Wire.Op.t list);
  assert (Option.is_none (dispatch (event 6L)));
  let next_node, next_handler = identity (commit (Some (view (fun _ -> 4)))) in
  assert (not (Gpuio_protocol.Node_id.equal node next_node));
  assert (Option.is_none (dispatch (event 7L)));
  assert (
    Option.equal
      Int.equal
      (dispatch (event ~node:next_node ~handler:next_handler ~tree_revision:4L 1L))
      (Some 4));
  print_endline
    "latest callback, monotonic observations, historical cancellation, atomic failure \
     and remount fences";
  [%expect
    {| latest callback, monotonic observations, historical cancellation, atomic failure and remount fences |}]
;;

let%expect_test
    "slider pending updates preserve observation fences and duplicate controllers fail"
  =
  let reconciler = Reconciler.create window in
  let view callback =
    View.slider
      ~controller:(Key.of_string_exn "stable")
      ~config
      ~initial:(range 2. 7.)
      ~on_event:callback
      ()
  in
  let prepare view = Reconciler.prepare reconciler ~theme:Theme.default (Some view) in
  let update = prepare (view (fun _ -> 1)) |> ok in
  Reconciler.accept reconciler update |> ok;
  let node, handler =
    match Reconciler.message update with
    | Some (Apply { operations; _ }) ->
      List.find_map_exn operations ~f:(function
        | Wire.Op.Create (node, Slider, "", Some handler) -> Some (node, handler)
        | _ -> None)
    | None | Some _ -> assert false
  in
  let dispatch revision =
    Reconciler.dispatch
      reconciler
      (Slider_event (window, node, handler, 1L, Preview { snapshot with revision }))
  in
  let pending = prepare (view (fun _ -> 2)) |> ok in
  assert (Option.equal Int.equal (dispatch 1L) (Some 1));
  Reconciler.accept reconciler pending |> ok;
  assert (Option.is_none (dispatch 1L));
  assert (Option.equal Int.equal (dispatch 2L) (Some 2));
  (* Different parent keys avoid the sibling-key validator and exercise the
     per-window controller registry. A failed prepare cannot publish callbacks. *)
  let duplicate =
    View.column
      [ View.column ~key:(Key.of_int 1) [ view (fun _ -> 99) ]
      ; View.column ~key:(Key.of_int 2) [ view (fun _ -> 99) ]
      ]
  in
  assert (Result.is_error (prepare duplicate));
  assert (Option.equal Int.equal (dispatch 3L) (Some 2));
  Reconciler.close reconciler;
  assert (Option.is_none (dispatch 4L));
  print_endline
    "pending updates retain delivered revisions; duplicate ownership and close are fenced";
  [%expect
    {| pending updates retain delivered revisions; duplicate ownership and close are fenced |}]
;;
