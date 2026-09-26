open Core
open Gpuio
module C = Color_input
module W = Gpuio_protocol.Color_input_wire
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok
let initial = Color_value.Value.Color (Color_value.Rgba.of_hex "#00FF00" |> ok)
let config = Color_input_test.config ()

let snapshot =
  let s = Color_input_test.snapshot () in
  { s with
    revision = 0L
  ; value = s.committed
  ; interaction = None
  ; draft = None
  ; channels = { hue_degrees = 120.; saturation = 1.; lightness = 0.5; alpha = 1. }
  }
;;

let event_bytes events =
  Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events |> Bigstring.to_string
;;

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test
    "retained color envelopes agree with independent tags and reject malformed events"
  =
  let request =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Color_input, "", Some handler)
          ; Set_color_input
              (node, C.Expert.config_to_wire config, C.Expert.value_to_wire initial)
          ; Set_root (Some node)
          ]
      }
  in
  let preview =
    Wire.Event.Color_input_event
      (window, node, handler, 1L, Preview (Color_input_test.snapshot ()))
  in
  let bytes = event_bytes [ preview ] in
  Eio_main.run (fun env ->
    let fs = Eio.Stdenv.fs env in
    assert (
      String.equal
        (Wire.Message.encode request |> ok |> hex)
        (Eio.Path.load Eio.Path.(fs / "color-request.hex") |> String.strip));
    assert (
      String.equal
        (hex bytes)
        (Eio.Path.load Eio.Path.(fs / "color-events.hex") |> String.strip)));
  assert (List.equal Wire.Event.equal (Wire.Event.decode bytes |> ok) [ preview ]);
  for length = 0 to String.length bytes - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix bytes length)))
  done;
  assert (Result.is_error (Wire.Event.decode (bytes ^ "\000")));
  List.iter
    [ -1L, W.Event.Observed snapshot
    ; 1L, Preview snapshot
    ; 1L, Observed { snapshot with value = Color (-1L) }
    ; ( 1L
      , Observed
          { snapshot with channels = { snapshot.channels with alpha = Float.infinity } } )
    ]
    ~f:(fun (revision, event) ->
      assert (
        Result.is_error
          (Wire.Event.decode
             (event_bytes [ Color_input_event (window, node, handler, revision, event) ]))));
  print_endline
    "Kind 41 / operation 47 / event 52: paired envelopes and validated bounded \
     observations";
  [%expect
    {| Kind 41 / operation 47 / event 52: paired envelopes and validated bounded observations |}]
;;

let%expect_test
    "color owners retain native values and callback revisions across configuration"
  =
  let reconciler = Reconciler.create window in
  let controller = Key.of_string_exn "accent" in
  let view ?(config = config) ?(initial = initial) callback =
    View.color_input ~controller ~config ~initial ~on_event:callback ()
  in
  let commit view =
    let update = Reconciler.prepare reconciler ~theme:Theme.default view |> ok in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply { operations; _ }) -> operations
    | None -> []
    | Some _ -> assert false
  in
  let identity ops =
    List.find_map_exn ops ~f:(function
      | Wire.Op.Create (node, Color_input, "", Some handler) -> Some (node, handler)
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
    Wire.Event.Color_input_event
      (window, node, handler, tree_revision, Observed { snapshot with revision })
  in
  let dispatch = Reconciler.dispatch reconciler in
  assert (Option.equal Int.equal (dispatch (event 0L)) (Some 1));
  assert (List.is_empty (commit (Some (view ~initial:Empty (fun _ -> 2)))));
  assert (Option.is_none (dispatch (event 0L)));
  assert (Option.equal Int.equal (dispatch (event 1L)) (Some 2));
  List.iter
    [ event ~tree_revision:99L 2L
    ; event ~tree_revision:(-1L) 2L
    ; event ~window:(Gpuio_protocol.Window_id.create ~slot:0L ~generation:2L |> ok) 2L
    ; event ~node:(Gpuio_protocol.Node_id.create ~slot:0L ~generation:2L |> ok) 2L
    ; event ~handler:(Gpuio_protocol.Handler_id.create ~slot:0L ~generation:2L |> ok) 2L
    ; Color_input_event
        ( window
        , node
        , handler
        , 1L
        , Observed
            { snapshot with
              revision = 99L
            ; channels = { snapshot.channels with hue_degrees = 300. }
            } )
    ]
    ~f:(fun event -> assert (Option.is_none (dispatch event)));
  assert (Option.equal Int.equal (dispatch (event 2L)) (Some 2));
  let labels = C.Labels.english ~control:"Updated" |> ok in
  let updated =
    C.Config.create
      ~labels
      ~alpha_policy:Opaque_only
      ~allow_empty:false
      ~disabled:true
      ~read_only:true
      ()
    |> ok
  in
  let ops = commit (Some (view ~config:updated ~initial:Empty (fun _ -> 3))) in
  assert (
    List.length ops = 1
    && List.for_all ops ~f:(function
      | Wire.Op.Set_color_input _ -> true
      | _ -> false));
  let pending =
    Reconciler.prepare
      reconciler
      ~theme:Theme.default
      (Some (view ~config:updated (fun _ -> 4)))
    |> ok
  in
  assert (Option.equal Int.equal (dispatch (event 3L)) (Some 3));
  Reconciler.accept reconciler pending |> ok;
  assert (Option.is_none (dispatch (event 3L)));
  assert (Option.equal Int.equal (dispatch (event 4L)) (Some 4));
  let duplicate =
    View.column
      [ View.column ~key:(Key.of_int 1) [ view (fun _ -> 9) ]
      ; View.column ~key:(Key.of_int 2) [ view (fun _ -> 9) ]
      ]
  in
  assert (
    Result.is_error (Reconciler.prepare reconciler ~theme:Theme.default (Some duplicate)));
  assert (Option.equal Int.equal (dispatch (event 5L)) (Some 4));
  ignore (commit None : Wire.Op.t list);
  assert (Option.is_none (dispatch (event 6L)));
  assert (
    Result.is_error
      (Reconciler.prepare
         reconciler
         ~theme:Theme.default
         (Some (view ~config:updated ~initial:Empty (fun _ -> 9)))));
  let fresh, new_handler = identity (commit (Some (view (fun _ -> 5)))) in
  assert (not (Gpuio_protocol.Node_id.equal fresh node));
  assert (Option.is_none (dispatch (event 7L)));
  assert (
    Option.equal
      Int.equal
      (dispatch
         (event
            ~node:fresh
            ~handler:new_handler
            ~tree_revision:(Reconciler.revision reconciler)
            0L))
      (Some 5));
  Reconciler.close reconciler;
  assert (Option.is_none (dispatch (event 8L)));
  print_endline
    "stable seed, callback-only refresh, pending updates, duplicate owners and stale \
     leases fenced";
  [%expect
    {| stable seed, callback-only refresh, pending updates, duplicate owners and stale leases fenced |}]
;;

let%expect_test "color input form metadata uses the retained owner's native role" =
  let view =
    View.color_input
      ~controller:(Key.of_string_exn "accent")
      ~config
      ~initial
      ~on_event:Fn.id
      ()
  in
  let field =
    Accessibility.Field.create ~label:"Accent" ~help:"Choose a color" ~required:true ()
    |> ok
  in
  let view = View.with_accessibility view (Accessibility.field field) |> ok in
  let reconciler = Reconciler.create window in
  let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
  assert (Option.is_some (Reconciler.message update));
  let invalid = Accessibility.create ~role:Accessibility.Role.Link () |> ok in
  assert (Result.is_error (View.with_accessibility view invalid));
  print_endline "Form metadata accepted; native color role cannot be overridden";
  [%expect {| Form metadata accepted; native color role cannot be overridden |}]
;;
