open Core
open Gpuio
open Gpuio_protocol
module C = Carousel_track
module W = Carousel_track_wire

let ok = Or_error.ok_exn
let window = Window_id.create ~slot:0L ~generation:1L |> ok
let item name = C.Item.create ~id:(C.Id.of_string name |> ok) ~label:name () |> ok
let values = List.map [ "a"; "β"; "c" ] ~f:item
let bytes writer value = Bin_prot.Utils.bin_dump writer value |> Bigstring.to_string

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let fixture name =
  Eio_main.run (fun env ->
    Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip)
;;

let observation ?(epoch = 0L) ?(lineage = 0L) () =
  W.Request.Layout
    { lineage; epoch; stops = Some { canonical = [ 0L; 1L; 1L ]; looping = Finite } }
;;

let%expect_test "track operation and event envelopes are paired and strictly decoded" =
  let node = Node_id.create ~slot:1L ~generation:2L |> ok in
  let handler = Handler_id.create ~slot:3L ~generation:4L |> ok in
  let config : W.Config.t =
    { lineage = 3L
    ; carousel =
        { revision = 7L
        ; ids = [ "a"; "β"; "c" ]
        ; selected = Some 1L
        ; looping = true
        ; disabled = false
        ; axis = Vertical
        ; auto_advance_ms = Some 5000L
        ; direction = Next
        }
    }
  in
  let message =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations = [ Set_carousel_track (node, config) ]
      }
  in
  assert (
    String.equal
      (hex (bytes Wire.Message.bin_writer_t message))
      (fixture "carousel-track-operation.hex"));
  let events =
    List.map
      W.Request.
        [ Layout
            { lineage = 3L
            ; epoch = 9L
            ; stops = Some { canonical = [ 0L; 1L; 1L ]; looping = Jump }
            }
        ; Auto_next { revision = 7L; geometry_epoch = 9L; from = "β"; target = "c" }
        ; Next
        ]
      ~f:(fun request ->
        Wire.Event.Carousel_track_requested (window, node, handler, 5L, request))
  in
  let encoded = bytes [%bin_writer: Wire.Event.t list] events in
  assert (String.equal (hex encoded) (fixture "carousel-track-events.hex"));
  assert (List.equal Wire.Event.equal (Wire.Event.decode encoded |> ok) events);
  for count = 0 to String.length encoded - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix encoded count)))
  done;
  assert (Result.is_error (Wire.Event.decode (encoded ^ "\000")));
  let malformed =
    Wire.Event.Carousel_track_requested
      ( window
      , node
      , handler
      , 5L
      , Layout
          { lineage = 0L
          ; epoch = 0L
          ; stops = Some { canonical = [ 1L ]; looping = Finite }
          } )
  in
  assert (
    Result.is_error
      (Wire.Event.decode (bytes [%bin_writer: Wire.Event.t list] [ malformed ])));
  assert (String.equal (hex (bytes Wire.Kind.bin_writer_t Carousel_track)) "36");
  assert (String.equal (hex (bytes Wire.Kind.bin_writer_t Carousel_track_group)) "37");
  print_endline "paired tags; strict envelopes; malformed layout rejected";
  [%expect {| paired tags; strict envelopes; malformed layout rejected |}]
;;

let%expect_test "track view keeps item owners and fences events before model conversion" =
  let model = C.create values |> ok in
  let r = Reconciler.create window in
  let view model =
    View.carousel_track
      model
      ~key:(Key.of_string_exn "gallery")
      ~label:"Cards"
      ~item_style:(fun item ->
        Style.create_exn
          [ Width
              (Length.px_exn (if String.equal (C.Item.label item) "a" then 80. else 120.))
          ])
      ~on_request:Fn.id
      ~content:(fun item -> [ View.text (C.Item.label item) ])
      ()
  in
  let prepare model =
    Reconciler.prepare r ~theme:Theme.default (Option.map model ~f:view)
  in
  let accept model =
    let update = prepare model |> ok in
    Reconciler.accept r update |> ok;
    match Reconciler.message update with
    | Some (Apply tx) -> tx.operations
    | _ -> []
  in
  let owner ops =
    List.find_map_exn ops ~f:(function
      | Wire.Op.Create (n, Carousel_track, _, Some h) -> Some (n, h)
      | _ -> None)
  in
  let initial = accept (Some model) in
  let node, handler = owner initial in
  assert (
    List.count initial ~f:(function
      | Wire.Op.Create (_, Carousel_track_group, "", None) -> true
      | _ -> false)
    = 1);
  let event
        ?(node = node)
        ?(handler = handler)
        ?(revision = 1L)
        ?(window = window)
        request
    =
    Wire.Event.Carousel_track_requested (window, node, handler, revision, request)
  in
  let dispatch ?node ?handler ?revision ?window request =
    Reconciler.dispatch r (event ?node ?handler ?revision ?window request)
  in
  assert (
    List.count initial ~f:(function
      | Wire.Op.Create (_, Panel, _, _) -> true
      | _ -> false)
    = 3);
  assert (
    List.exists initial ~f:(function
      | Wire.Op.Set_carousel_track (_, _) -> true
      | _ -> false));
  let measured =
    C.apply_request model (dispatch (observation ()) |> Option.value_exn) |> ok
  in
  assert (C.has_layout measured && C.can_next measured);
  let update = accept (Some measured) in
  assert (
    not
      (List.exists update ~f:(function
         | Wire.Op.Set_carousel_track _ -> true
         | _ -> false)));
  assert (
    not
      (List.exists update ~f:(function
         | Wire.Op.Create (_, Panel, _, _) | Remove _ -> true
         | _ -> false)));
  let selected = C.apply_request measured C.Request.next |> ok in
  let selected_ops = accept (Some selected) in
  assert (
    List.exists selected_ops ~f:(function
      | Wire.Op.Set_carousel_track _ -> true
      | _ -> false));
  assert (
    not
      (List.exists selected_ops ~f:(function
         | Wire.Op.Create (_, Panel, _, _) | Remove _ -> true
         | _ -> false)));
  assert (Result.is_error (prepare (Some model)));
  let disabled = C.with_disabled selected true |> ok in
  ignore (accept (Some disabled) : Wire.Op.t list);
  assert (Option.is_some (dispatch (observation ~epoch:1L ())));
  assert (Option.is_none (dispatch Next));
  assert (Option.is_none (dispatch (observation ~lineage:1L ())));
  assert (Option.is_none (dispatch ~revision:1000L (observation ())));
  assert (Option.is_none (dispatch ~revision:(-1L) (observation ())));
  assert (
    Option.is_none
      (dispatch
         ~node:(Node_id.create ~slot:(Node_id.slot node) ~generation:1000L |> ok)
         (observation ())));
  assert (
    Option.is_none
      (dispatch
         ~handler:
           (Handler_id.create ~slot:(Handler_id.slot handler) ~generation:1000L |> ok)
         (observation ())));
  assert (
    Option.is_none
      (dispatch ~window:(Window_id.create ~slot:0L ~generation:2L |> ok) (observation ())));
  ignore (accept None : Wire.Op.t list);
  assert (Option.is_none (dispatch (observation ())));
  let node2, handler2 = owner (accept (Some model)) in
  assert (Option.is_none (dispatch (observation ())));
  let current request =
    dispatch ~node:node2 ~handler:handler2 ~revision:(Reconciler.revision r) request
  in
  let remounted =
    C.apply_request measured (current (observation ()) |> Option.value_exn) |> ok
  in
  assert (C.has_layout remounted);
  Reconciler.close r;
  assert (Option.is_none (current (observation ())));
  print_endline
    "retained keys; layout-only updates; revision checks; disabled observations; retired \
     sources rejected";
  [%expect
    {| retained keys; layout-only updates; revision checks; disabled observations; retired sources rejected |}]
;;

let%expect_test "motion is checked presentation without selection revisions or remounts" =
  List.iter [ 0.; -1.; 10_001. ] ~f:(fun ms ->
    assert (Or_error.is_error (C.Motion.create (Time_ns.Span.of_ms ms))));
  let fractional = C.Motion.create (Time_ns.Span.of_ns 1.) |> ok in
  assert (
    Int64.equal (C.Motion.Expert.to_wire fractional |> Option.value_exn).duration_ms 1L);
  let model = C.create values |> ok in
  let reconciler = Reconciler.create window in
  let update motion =
    let view =
      View.carousel_track
        model
        ~motion
        ~label:"Cards"
        ~show_controls:false
        ~on_request:Fn.id
        ~content:(fun _ -> [ View.text "Retained" ])
        ()
    in
    let prepared = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept reconciler prepared |> ok;
    match Reconciler.message prepared with
    | Some (Wire.Message.Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let initial = update C.Motion.default in
  let owner =
    List.find_map_exn initial ~f:(function
      | Wire.Op.Set_carousel_track_motion (id, Some _) -> Some id
      | _ -> None)
  in
  List.iter [ C.Motion.immediate; fractional; C.Motion.default ] ~f:(fun motion ->
    let ops = update motion in
    assert (List.length ops = 1);
    match List.hd_exn ops with
    | Wire.Op.Set_carousel_track_motion (id, value) ->
      assert (Node_id.equal owner id);
      assert (Option.equal W.Motion.equal value (C.Motion.Expert.to_wire motion))
    | _ -> assert false);
  assert (List.is_empty (update C.Motion.default));
  print_endline "checked duration; motion changes retain model, owner and children";
  [%expect {| checked duration; motion changes retain model, owner and children |}]
;;

let%expect_test "track motion has independent paired operation bytes" =
  let node = Node_id.create ~slot:0L ~generation:1L |> ok in
  let message =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_carousel_track_motion (node, C.Motion.Expert.to_wire C.Motion.default)
          ; Set_carousel_track_motion (node, None)
          ]
      }
  in
  assert (
    String.equal
      (hex (Wire.Message.encode message |> ok))
      (fixture "carousel-track-motion.hex"));
  print_endline "motion operation 97 matches independent bytes";
  [%expect {| motion operation 97 matches independent bytes |}]
;;
