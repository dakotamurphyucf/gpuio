open Core
open Gpuio
open Gpuio_protocol

let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let%expect_test
    "hover card kind, ownership and open events match independent Rust fixtures"
  =
  let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let handler = Handler_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let config open_state =
    Hover_card.Config.create ~label:"Details" ~width:200. ~open_state ()
    |> Or_error.ok_exn
    |> Hover_card.Expert.tooltip
    |> Tooltip.Expert.to_wire
  in
  let request =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Hover_card, "", Some handler)
          ; Set_tooltip (node, config (Managed { initially_open = false }))
          ; Set_tooltip (node, config (Controlled true))
          ; Set_placement (node, Some { side = Top; align = Center; offset = 6. })
          ]
      }
  in
  let events =
    List.map [ true; false ] ~f:(fun open_ ->
      Wire.Event.Tooltip_open_changed (window, node, handler, 1L, open_))
  in
  Eio_main.run (fun env ->
    let load file =
      let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / file) |> String.strip in
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    assert (
      String.equal
        (load "hover-card-request.hex")
        (Wire.Message.encode request |> Or_error.ok_exn));
    let bytes = load "tooltip-v1-events.hex" in
    assert (
      String.equal
        bytes
        (Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events
         |> Bigstring.to_string));
    assert (List.equal Wire.Event.equal events (Wire.Event.decode bytes |> Or_error.ok_exn)));
  [%expect {| |}]
;;

let%expect_test "cards validate timing/labels and always permit interaction without grace"
  =
  List.iter
    [ ""; "\255"; "a\000b"; String.make 4097 'x' ]
    ~f:(fun label -> assert (Result.is_error (Hover_card.Config.create ~label ())));
  List.iter [ Float.nan; 0.; 16385. ] ~f:(fun width ->
    assert (Result.is_error (Hover_card.Config.create ~label:"Details" ~width ())));
  List.iter
    [ Time_ns.Span.of_ns (-1.); Time_ns.Span.of_sec 61. ]
    ~f:(fun delay ->
      assert (
        Result.is_error (Hover_card.Config.create ~label:"Details" ~show_delay:delay ()));
      assert (
        Result.is_error (Hover_card.Config.create ~label:"Details" ~hide_delay:delay ())));
  let config =
    Hover_card.Config.create ~label:"Details" ()
    |> Or_error.ok_exn
    |> Hover_card.Expert.tooltip
    |> Tooltip.Expert.to_wire
  in
  print_s
    [%sexp
      (config.hoverable : bool)
    , (config.show_delay_ns : int64)
    , (config.hide_delay_ns : int64)
    , (config.skip_delay_ns : int64)];
  [%expect {| (true 600000000 300000000 0) |}]
;;

let%expect_test
    "controlled card closure keeps identities; callback replacement and retirement"
  =
  let reconciler = Reconciler.create window in
  let render ?(disabled = false) ~open_state callback =
    let view =
      View.hover_card
        ~config:
          (Hover_card.Config.create ~label:"Details" ~open_state ~disabled ()
           |> Or_error.ok_exn)
        ~anchor:(View.button ~on_click:(fun () -> "anchor") "Details")
        ~content:(View.button ~on_click:(fun () -> "content") "Open profile")
        ~on_open_change:(fun open_ -> callback ^ Bool.to_string open_)
        ()
    in
    let update =
      Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> Or_error.ok_exn
    in
    Reconciler.accept reconciler update |> Or_error.ok_exn;
    match Reconciler.message update with
    | Some (Wire.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  let initial = render ~open_state:(Controlled true) "first:" in
  let node, handler =
    List.find_map_exn initial ~f:(function
      | Wire.Op.Create (id, Hover_card, _, Some handler) -> Some (id, handler)
      | _ -> None)
  in
  let dispatch open_ =
    Reconciler.dispatch
      reconciler
      (Wire.Event.Tooltip_open_changed (window, node, handler, 1L, open_))
  in
  print_s [%sexp (dispatch false : string option)];
  assert (List.is_empty (render ~open_state:(Controlled true) "latest:"));
  print_s [%sexp (dispatch false : string option)];
  let changed = render ~open_state:(Controlled false) "closed:" in
  assert (
    not
      (List.exists changed ~f:(function
         | Wire.Op.Create _ | Remove _ | Splice _ -> true
         | _ -> false)));
  ignore
    (render ~disabled:true ~open_state:(Controlled true) "disabled:" : Wire.Op.t list);
  print_s [%sexp (dispatch true : string option)];
  Reconciler.close reconciler;
  print_s [%sexp (dispatch false : string option)];
  [%expect
    {|
    (first:false)
    (latest:false)
    ()
    ()
    |}]
;;
