open Core
open Gpuio
open Gpuio_protocol

let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let config
      ?(disabled = false)
      ?(open_state = Tooltip.Open_state.Managed { initially_open = false })
      ()
  =
  Tooltip.Config.create ~label:"Details" ~disabled ~open_state () |> Or_error.ok_exn
;;

let%expect_test "tooltip configuration rejects invalid labels, geometry and delays" =
  List.iter
    [ Tooltip.Config.create ~label:"" ()
    ; Tooltip.Config.create ~label:"Details" ~width:Float.nan ()
    ; Tooltip.Config.create ~label:"Details" ~show_delay:(Time_ns.Span.of_ms (-1.)) ()
    ; Tooltip.Config.create ~label:"Details" ~hide_delay:(Time_ns.Span.of_sec 61.) ()
    ; Tooltip.Config.create ~label:"Details" ~skip_delay:(Time_ns.Span.of_sec 61.) ()
    ; Tooltip.Config.create ~label:"Details" ~show_delay:Time_ns.Span.zero ()
    ]
    ~f:(fun result -> print_s [%sexp (Result.is_ok result : bool)]);
  [%expect
    {|
    false
    false
    false
    false
    false
    true
    |}]
;;

let%expect_test
    "tooltip keeps content on visibility changes and refreshes callbacks without wire \
     work"
  =
  let reconciler = Reconciler.create window in
  let anchor = View.button ~on_click:(fun () -> "anchor") "Details" in
  let content = View.text "Retained content" in
  let render ?disabled ?open_state callback =
    let view =
      View.tooltip
        ~config:(config ?disabled ?open_state ())
        ~anchor
        ~content
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
  let initial = render "first:" in
  let node, handler =
    List.find_map_exn initial ~f:(function
      | Wire.Op.Create (node, Tooltip, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let event open_ = Wire.Event.Tooltip_open_changed (window, node, handler, 1L, open_) in
  let show () =
    print_s [%sexp (Reconciler.dispatch reconciler (event true) : string option)]
  in
  show ();
  print_s [%sexp (render "latest:" : Wire.Op.t list)];
  show ();
  let changes = render ~open_state:(Controlled true) "controlled:" in
  print_s
    [%sexp
      (List.exists changes ~f:(function
         | Wire.Op.Create _ | Remove _ | Splice _ -> true
         | _ -> false)
       : bool)];
  ignore (render ~disabled:true "disabled:" : Wire.Op.t list);
  show ();
  ignore (render "enabled:" : Wire.Op.t list);
  show ();
  Reconciler.close reconciler;
  show ();
  [%expect
    {|
    (first:true)
    ()
    (latest:true)
    false
    ()
    ()
    ()
    |}]
;;

let%expect_test "tooltip ownership modes and open events match independent Rust fixtures" =
  let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let handler = Handler_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let config open_state =
    Tooltip.Config.create ~label:"Details" ~width:200. ~open_state ()
    |> Or_error.ok_exn
    |> Tooltip.Expert.to_wire
  in
  let request =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Tooltip, "", Some handler)
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
        (load "tooltip-v1-request.hex")
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
