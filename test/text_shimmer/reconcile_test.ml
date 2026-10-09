open Core
open Gpuio
module Wire = Gpuio_protocol.Wire
module W = Gpuio_protocol.Text_shimmer_wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let key = Key.of_string_exn "title"
let config = Text_shimmer.Config.default
let wire = Text_shimmer.Expert.to_wire config
let decorate view config = View.with_text_shimmer view config |> ok

let ops update =
  match Reconciler.message update with
  | Some (Wire.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let commit t view =
  let update = Reconciler.prepare t ~theme:Theme.default (Some view) |> ok in
  Reconciler.accept t update |> ok;
  ops update
;;

let%expect_test "op61 has independent native set/clear transaction fixtures" =
  let minimal : W.Config.t =
    { duration_ms = 1
    ; spread = Relative 0.5
    ; direction = Left_to_right
    ; repeat = Loop
    ; animated = true
    ; highlight = None
    ; appearance = None
    }
  in
  List.iter [ Some minimal; None ] ~f:(fun config ->
    let packet =
      Wire.Message.Apply
        { window
        ; base = 0L
        ; revision = 1L
        ; operations = [ Set_text_shimmer (node, config) ]
        }
    in
    Wire.Message.encode packet
    |> ok
    |> String.iter ~f:(fun c -> printf "%02x" (Char.to_int c));
    print_endline "");
  [%expect
    {|
    0300010001013d0001010100000000000000e03f0001010000
    0300010001013d000100
    |}]
;;

let%expect_test "decoration validates source and kind without changing text metadata" =
  let metadata = Accessibility.create ~label:"Working title" ~live:Off () |> ok in
  let base : unit View.t =
    View.with_accessibility (View.text ~key "Aé👩‍💻") metadata |> ok
  in
  let before = View.Expert.describe base in
  let after = View.Expert.describe (decorate base (Some config)) in
  assert (Option.equal Key.equal before.key after.key);
  assert (View.Expert.Kind.equal before.kind after.kind);
  assert (String.equal before.text after.text);
  assert (phys_equal before.style after.style);
  assert (phys_equal before.accessibility after.accessibility);
  assert (Option.is_none after.on_click && List.is_empty after.children);
  List.iter
    [ String.make (W.max_text_bytes + 1) 'x'; "\255" ]
    ~f:(fun source ->
      let text = View.text source in
      assert (Result.is_error (View.with_text_shimmer text (Some config)));
      assert (Result.is_ok (View.with_text_shimmer text None)));
  ignore
    (decorate (View.text (String.make W.max_text_bytes 'x')) (Some config) : unit View.t);
  List.iter
    [ View.row []; View.button ~on_click:(fun () -> ()) "Button" ]
    ~f:(fun view ->
      List.iter [ None; Some config ] ~f:(fun config ->
        assert (Result.is_error (View.with_text_shimmer view config))));
  let content = Text_content.create "Aé👩‍💻" |> ok in
  let styled = View.styled_text ~key content |> fun view -> decorate view (Some config) in
  assert (
    Option.equal
      Text_content.equal
      (View.Expert.describe styled).text_content
      (Some content));
  (* Compiles the public Bonsai API with precisely the same constructor contract. *)
  ignore
    (Gpuio_bonsai.View.with_text_shimmer (Gpuio_bonsai.View.text "Working") (Some config)
     |> ok
     : Gpuio_bonsai.View.t);
  print_endline
    "bounded UTF-8; ordinary text only; stable source/key/style/metadata; Core and Bonsai";
  [%expect
    {| bounded UTF-8; ordinary text only; stable source/key/style/metadata; Core and Bonsai |}]
;;

let%expect_test
    "set update clear and styled transitions preserve a single native identity"
  =
  let t = Reconciler.create window in
  let base = View.text ~key "Working" in
  assert (
    List.equal
      Wire.Op.equal
      (commit t base)
      [ Create (node, Text, "Working", None); Set_root (Some node) ]);
  let animated = decorate base (Some config) in
  assert (
    List.equal Wire.Op.equal (commit t animated) [ Set_text_shimmer (node, Some wire) ]);
  assert (List.is_empty (commit t animated));
  let paused = Text_shimmer.Config.create ~animated:false () |> ok in
  assert (
    List.equal
      Wire.Op.equal
      (commit t (decorate base (Some paused)))
      [ Set_text_shimmer (node, Some (Text_shimmer.Expert.to_wire paused)) ]);
  (* Discarded preparations must not replace acknowledged configuration. *)
  let before = Reconciler.revision t in
  let discarded = Reconciler.prepare t ~theme:Theme.default (Some animated) |> ok in
  assert (List.length (ops discarded) = 1);
  assert (Int64.equal before (Reconciler.revision t));
  assert (List.is_empty (commit t (decorate base (Some paused))));
  let content = Text_content.create "Aé世界" |> ok in
  let styled = decorate (View.styled_text ~key content) (Some paused) in
  assert (
    List.equal
      Wire.Op.equal
      (commit t styled)
      [ Set_styled_text
          (node, Text_content.Expert.to_wire content ~theme:Theme.default |> ok)
      ]);
  assert (
    List.equal
      Wire.Op.equal
      (commit t (decorate styled None))
      [ Set_text_shimmer (node, None) ]);
  assert (List.is_empty (commit t (decorate styled None)));
  assert (
    List.equal
      Wire.Op.equal
      (commit t animated)
      [ Set_text (node, "Working"); Set_text_shimmer (node, Some wire) ]);
  let larger = String.make (W.max_text_bytes + 1) 'x' in
  assert (
    List.equal
      Wire.Op.equal
      (commit t (View.text ~key larger))
      [ Set_text (node, larger); Set_text_shimmer (node, None) ]);
  print_endline
    "no replacement nodes/handlers; no-op reuse; pause; discarded prepare; styled \
     source; clear; grow after clearing";
  [%expect
    {| no replacement nodes/handlers; no-op reuse; pause; discarded prepare; styled source; clear; grow after clearing |}]
;;
