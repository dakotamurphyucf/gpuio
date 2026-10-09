open Core
open Gpuio
module W = Gpuio_protocol.Spinner_wire

let ok = Or_error.ok_exn
let owner = Asset.Expert.Owner.create ()
let id = Gpuio_protocol.Resource_id.create ~slot:7L ~generation:3L |> ok
let icon format = Asset.Expert.handle ~owner ~id ~format

let encoded config =
  Bin_prot.Utils.bin_dump W.Config.bin_writer_t config
  |> Bigstring.to_string
  |> String.to_list
  |> List.map ~f:(fun char -> sprintf "%02x" (Char.to_int char))
  |> String.concat
;;

let fixture file config =
  Eio_main.run (fun env ->
    let expected = Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / file) |> String.strip in
    assert (String.equal (encoded config) expected))
;;

let%expect_test "spinner defaults and custom source match the Rust configuration" =
  let config = Spinner.Config.create ~label:"Loading" () |> ok in
  let wire = Spinner.Expert.to_wire config ~owner:(Some owner) in
  fixture "spinner-default.hex" wire;
  print_s [%sexp (wire : W.Config.t)];
  let easing = Animation.Easing.cubic_bezier ~x1:0.25 ~y1:(-2.) ~x2:0.75 ~y2:3. |> ok in
  let custom =
    Spinner.Config.create
      ~label:"Indexing"
      ~icon:(icon Svg)
      ~period:(Time_ns.Span.of_ms 100.5)
      ~animated:false
      ~easing
      ()
    |> ok
  in
  fixture "spinner-custom.hex" (Spinner.Expert.to_wire custom ~owner:(Some owner));
  let image = Spinner.Expert.image custom |> Option.value_exn in
  assert (Image.Fit.equal (Image.Config.fit image) Contain);
  assert (
    Image.Description.equal (Image.Config.description image) Image.Description.decorative);
  let legacy = Loading.Config.create ~kind:Spinner ~label:"Loading" () |> ok in
  assert ((Loading.Expert.to_wire legacy).period_ms = 1200);
  print_endline "custom SVG is decorative/contained; legacy period remains 1200ms";
  [%expect
    {|
    ((label Loading) (animated true) (period_ms 800) (easing Ease_in_out)
     (source ()))
    custom SVG is decorative/contained; legacy period remains 1200ms
    |}]
;;

let%expect_test "spinner validation shares loading bounds and requires SVG" =
  List.iter
    [ ""; " \t"; "\255"; "a\000b"; String.make 4097 'x' ]
    ~f:(fun label -> assert (Result.is_error (Spinner.Config.create ~label ())));
  List.iter
    [ Time_ns.Span.min_value_representable
    ; Time_ns.Span.max_value_representable
    ; Time_ns.Span.of_ms 99.
    ; Time_ns.Span.of_ms 60001.
    ]
    ~f:(fun period ->
      assert (Result.is_error (Spinner.Config.create ~label:"Loading" ~period ())));
  List.iter [ 100.; 60000. ] ~f:(fun ms ->
    assert (
      Result.is_ok
        (Spinner.Config.create
           ~label:(String.make 4096 'x')
           ~period:(Time_ns.Span.of_ms ms)
           ())));
  List.iter [ Asset.Format.Png; Jpeg; Webp; Gif; Bmp; Tiff; Ico; Pnm ] ~f:(fun format ->
    assert (
      Result.is_error (Spinner.Config.create ~label:"Loading" ~icon:(icon format) ())));
  print_endline "labels, time bounds, rounding and SVG format checked before mounting";
  [%expect {| labels, time bounds, rounding and SVG format checked before mounting |}]
;;

let%expect_test "spinner source provenance is preserved without acquiring an asset" =
  let config = Spinner.Config.create ~label:"Loading" ~icon:(icon Svg) () |> ok in
  List.iter
    [ None; Some (Asset.Expert.Owner.create ()) ]
    ~f:(fun owner ->
      let wire = Spinner.Expert.to_wire config ~owner in
      match wire.source with
      | Some (Unavailable Wrong_application) -> ()
      | _ -> assert false);
  let wire = Spinner.Expert.to_wire config ~owner:(Some owner) in
  (match wire.source with
   | Some (Reference same) -> assert (Gpuio_protocol.Resource_id.equal same id)
   | _ -> assert false);
  assert (Spinner.Config.equal config config);
  let foreign =
    Asset.Expert.handle ~owner:(Asset.Expert.Owner.create ()) ~id ~format:Svg
  in
  let foreign = Spinner.Config.create ~label:"Loading" ~icon:foreign () |> ok in
  assert (not (Spinner.Config.equal config foreign));
  print_endline
    "equal slot/generation does not alias registrations from different applications";
  [%expect
    {| equal slot/generation does not alias registrations from different applications |}]
;;

let%expect_test "spinner reconciles atomically and legacy transitions clear icon state" =
  let module W = Gpuio_protocol.Wire in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create ~asset_owner:owner window in
  let config = Spinner.Config.create ~label:"Loading" ~icon:(icon Svg) () |> ok in
  let commit view =
    let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply { operations; _ }) -> operations
    | None -> []
    | Some _ -> assert false
  in
  let view callback = View.spinner ~config ~on_icon_change:callback () in
  let initial = commit (view (fun _ -> `First)) in
  let node, handler =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (node, Loading, "", Some handler) -> Some (node, handler)
      | _ -> None)
  in
  assert (
    List.exists initial ~f:(function
      | Set_spinner _ -> true
      | _ -> false));
  assert (
    not
      (List.exists initial ~f:(function
         | Set_image _ | Set_loading _ -> true
         | _ -> false)));
  assert (List.is_empty (commit (view (fun _ -> `Latest))));
  let event = W.Event.Image_state (window, node, handler, 1L, Loading) in
  assert (
    match Reconciler.dispatch reconciler event with
    | Some `Latest -> true
    | _ -> false);
  let legacy = Spinner.Expert.loading config in
  let reset = commit (View.loading ~config:legacy ()) in
  assert (
    List.exists reset ~f:(function
      | Set_loading (id, _) -> Gpuio_protocol.Node_id.equal id node
      | _ -> false));
  assert (
    not
      (List.exists reset ~f:(function
         | Create _ | Remove _ -> true
         | _ -> false)));
  assert (Option.is_none (Reconciler.dispatch reconciler event));
  let rebound = commit (view (fun _ -> `Latest)) in
  assert (
    List.exists rebound ~f:(function
      | Set_spinner (id, _) -> Gpuio_protocol.Node_id.equal id node
      | _ -> false));
  assert (Option.is_none (Reconciler.dispatch reconciler event));
  print_endline
    "atomic spinner setter; latest callback; same-key legacy reset retires the icon \
     handler";
  [%expect
    {| atomic spinner setter; latest callback; same-key legacy reset retires the icon handler |}]
;;

let%expect_test "spinner operation and negotiated capability match Rust" =
  let module W = Gpuio_protocol.Wire in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let config = Spinner.Config.create ~label:"Loading" () |> ok in
  let wire = Spinner.Expert.to_wire config ~owner:None in
  let bytes =
    W.Message.encode
      (Apply
         { window; base = 0L; revision = 1L; operations = [ Set_spinner (node, wire) ] })
    |> ok
  in
  let hex text =
    String.to_list text
    |> List.map ~f:(fun char -> sprintf "%02x" (Char.to_int char))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "spinner-operation.hex") |> String.strip
    in
    assert (String.equal (hex bytes) expected));
  assert (
    String.equal
      (W.Message.encode (Hello (W.version, 288230376151711744L)) |> ok |> hex)
      "0003fc0000000000000004");
  print_endline "operation 65; spinner capability 58";
  [%expect {| operation 65; spinner capability 58 |}]
;;
