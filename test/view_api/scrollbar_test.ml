open Core
open Gpuio
module S = Scrollbar
module W = Gpuio_protocol.Wire.Scrollbar

let ok = Or_error.ok_exn
let wire t = S.Expert.to_wire t ~theme:Theme.default |> ok

let rgba n =
  Color.rgba
    ~red:((n lsr 24) land 255)
    ~green:((n lsr 16) land 255)
    ~blue:((n lsr 8) land 255)
    ~alpha:(n land 255)
  |> ok
;;

let fixture name value =
  assert (W.valid value);
  let bytes = Bin_prot.Utils.bin_dump W.bin_writer_t value |> Bigstring.to_string in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun ch -> sprintf "%02x" (Char.to_int ch))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip in
    assert (String.equal hex expected));
  print_s [%sexp (String.length bytes : int)]
;;

let%expect_test "scrollbar scalar fields and both gradient spaces match independent bytes"
  =
  fixture "scrollbar-default.hex" (S.create ~label:"Viewport" () |> ok |> wire);
  let track =
    S.Track.create ~background:(rgba 0x10203040) ~border:(rgba 0xa0b0c0d0) ~width:16. ()
    |> ok
  in
  let track_hover = S.Track.create ~border:(rgba 0) ~width:24. () |> ok in
  let track_pressed = S.Track.create ~background:(rgba 0) ~width:32. () |> ok in
  let thumb =
    S.Thumb.create
      ~background:(Background.solid (rgba 0x010203ff))
      ~width:6.
      ~inset:4.
      ~radius:3.
      ~min_length:48.
      ()
    |> ok
  in
  let hover =
    Background.linear_gradient
      ~angle:90.
      ~from:(rgba 0xffffffff, 0.)
      ~to_:(rgba 0x112233ff, 1.)
    |> ok
  in
  let pressed =
    Background.linear_gradient_in
      Oklab
      ~angle:180.
      ~from:(rgba 0x010203ff, 0.25)
      ~to_:(rgba 0xabcdef80, 0.75)
    |> ok
  in
  let thumb_hover = S.Thumb.create ~background:hover ~width:10. ~radius:5. () |> ok in
  let thumb_pressed =
    S.Thumb.create ~background:pressed ~width:12. ~inset:2. ~radius:6. ~min_length:64. ()
    |> ok
  in
  let appearance =
    S.Appearance.create
      ~track
      ~track_hover
      ~track_pressed
      ~thumb
      ~thumb_hover
      ~thumb_pressed
      ()
  in
  let motion =
    S.Motion.create
      ~idle:(Time_ns.Span.of_ms 1500.)
      ~enter:(Time_ns.Span.of_ms 125.)
      ~exit:(Time_ns.Span.of_ms 250.)
      ~expand:(Time_ns.Span.of_ms 175.)
      ~entrance:Slide_and_fade
      ()
    |> ok
  in
  fixture
    "scrollbar-custom.hex"
    (S.create ~label:"Transcript" ~mode:Hover ~appearance ~motion () |> ok |> wire);
  [%expect
    {|
    43
    259
    |}]
;;

let%expect_test "checked dimensions labels and duration rounding" =
  List.iter [ Float.nan; Float.infinity; Float.neg_infinity; -1.; 16385. ] ~f:(fun n ->
    assert (Result.is_error (S.Track.create ~width:n ()));
    List.iter
      [ (fun () -> S.Thumb.create ~width:n ())
      ; (fun () -> S.Thumb.create ~inset:n ())
      ; (fun () -> S.Thumb.create ~radius:n ())
      ; (fun () -> S.Thumb.create ~min_length:n ())
      ]
      ~f:(fun create -> assert (Result.is_error (create ()))));
  List.iter
    [ ""; " \t\r\n\011\012"; "bad\000name"; "\255"; String.make 1025 'x' ]
    ~f:(fun label -> assert (Result.is_error (S.create ~label ())));
  ignore (S.create ~label:(String.make 1024 'x') () |> ok : S.t);
  List.iter
    [ Time_ns.Span.of_ns (-1.); Time_ns.Span.of_sec 60.001 ]
    ~f:(fun duration ->
      List.iter
        [ (fun () -> S.Motion.create ~idle:duration ())
        ; (fun () -> S.Motion.create ~enter:duration ())
        ; (fun () -> S.Motion.create ~exit:duration ())
        ; (fun () -> S.Motion.create ~expand:duration ())
        ]
        ~f:(fun create -> assert (Result.is_error (create ()))));
  let motion =
    S.Motion.create
      ~idle:Time_ns.Span.zero
      ~enter:(Time_ns.Span.of_ns 1.)
      ~exit:(Time_ns.Span.of_sec 60.)
      ()
    |> ok
  in
  let geometry =
    S.Thumb.create ~width:0. ~inset:16384. ~radius:16384. ~min_length:16384. () |> ok
  in
  let value =
    S.create ~label:"日本語" ~motion ~appearance:(S.Appearance.create ~thumb:geometry ()) ()
    |> ok
    |> wire
  in
  print_s [%sexp (value.motion : W.Motion.t)];
  assert (W.valid value);
  [%expect
    {|
    ((idle_ms 0) (enter_ms 1) (exit_ms 60000) (expand_ms 0) (entrance Fade)
     (thumb_hover_entrance Fade))
    |}]
;;

let%expect_test "theme resolution includes hidden states and preserves sparse overrides" =
  let token = Color.token_exn "scrollbar-ink" in
  let appearance =
    S.Appearance.create
      ~track:(S.Track.create ~width:16. () |> ok)
      ~thumb_pressed:(S.Thumb.create ~background:(Background.solid token) () |> ok)
      ()
  in
  let config = S.create ~label:"Theme preview" ~appearance () |> ok in
  assert (Result.is_error (S.Expert.to_wire config ~theme:Theme.default));
  List.iter [ 0x112233; 0xabcdef ] ~f:(fun color ->
    let theme = Theme.create [ "scrollbar-ink", Color.rgb_exn color ] |> ok in
    let value = S.Expert.to_wire config ~theme |> ok in
    assert (W.valid value);
    assert (Option.is_none value.appearance.thumb.background);
    assert (Option.is_none value.appearance.thumb_hover.background);
    assert (Option.is_none value.appearance.thumb_pressed.width);
    print_s
      [%sexp
        (value.appearance.thumb_pressed.background : Gpuio_protocol.Wire.Fill.t option)]);
  [%expect
    {|
    ((Solid (Rgba 287454207)))
    ((Solid (Rgba 2882400255)))
    |}]
;;

let%expect_test "attach and reset operation match independently encoded bytes" =
  let module Wire = Gpuio_protocol.Wire in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let config = S.create ~label:"Viewport" () |> ok |> wire in
  let message : Wire.Message.t =
    Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations = [ Set_scrollbar (node, Some config); Set_scrollbar (node, None) ]
      }
  in
  let bytes = Wire.Message.encode message |> ok in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun ch -> sprintf "%02x" (Char.to_int ch))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "scrollbar-operation.hex")
      |> String.strip
    in
    assert (String.equal hex expected));
  [%expect {| |}]
;;

let%expect_test "metadata changes and theme resolution retain view identity" =
  let module Wire = Gpuio_protocol.Wire in
  let module Node = Gpuio_protocol.Node_id in
  let node = Node.create ~slot:0L ~generation:1L |> ok in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let root = View.column [ View.text ~key:(Key.of_string_exn "retained") "Content" ] in
  let theme color = Theme.create [ "thumb", Color.rgb_exn color ] |> ok in
  let thumb =
    S.Thumb.create ~background:(Background.solid (Color.token_exn "thumb")) () |> ok
  in
  let appearance = S.Appearance.create ~thumb () in
  let config = S.create ~label:"Content" ~appearance () |> ok in
  let update theme scrollbar =
    let view = View.with_scrollbar root scrollbar |> ok in
    let pending = Reconciler.prepare reconciler ~theme (Some view) |> ok in
    let message = Reconciler.message pending in
    Reconciler.accept reconciler pending |> ok;
    message
  in
  ignore (update (theme 0x123456) None : Wire.Message.t option);
  let attached =
    match update (theme 0x123456) (Some config) with
    | Some (Apply { operations = [ Set_scrollbar (id, Some config) ]; _ }) ->
      assert (Node.equal id node);
      config
    | _ -> assert false
  in
  assert (Option.is_none (update (theme 0x123456) (Some config)));
  (match update (theme 0xabcdef) (Some config) with
   | Some (Apply { operations = [ Set_scrollbar (id, Some changed) ]; _ }) ->
     assert (Node.equal id node);
     assert (not (W.equal attached changed))
   | _ -> assert false);
  let before =
    Reconciler.prepare
      reconciler
      ~theme:Theme.default
      (Some (View.with_scrollbar root (Some config) |> ok))
  in
  assert (Result.is_error before);
  (match update (theme 0xabcdef) None with
   | Some (Apply { operations = [ Set_scrollbar (id, None) ]; _ }) ->
     assert (Node.equal id node)
   | _ -> assert false);
  assert (Option.is_none (update (theme 0xabcdef) None));
  [%expect {| |}]
;;

let%expect_test "scrollbar attachment rejects non-viewport roots" =
  let config = S.create ~label:"Viewport" () |> ok in
  let list =
    View.virtual_list
      ~config:(Virtual_list.Config.create ~height:(Fixed 24.) ~scrollbar:false () |> ok)
      []
    |> ok
  in
  List.iter [ None; Some config ] ~f:(fun scrollbar ->
    assert (Result.is_error (View.with_scrollbar (View.text "Text") scrollbar));
    assert (Result.is_ok (View.with_scrollbar (View.column []) scrollbar));
    let described = View.with_scrollbar list scrollbar |> ok |> View.Expert.describe in
    assert (Option.equal S.equal described.scrollbar scrollbar);
    let described_list = Option.value_exn described.virtual_list in
    assert (
      Virtual_list.Config.equal
        described_list.config
        (Option.value_exn (View.Expert.describe list).virtual_list).config));
  [%expect {| |}]
;;
