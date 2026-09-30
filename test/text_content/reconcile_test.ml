open Core
open Gpuio
module Wire = Gpuio_protocol.Wire
module W = Gpuio_protocol.Text_content_wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let key = Key.of_string_exn "label"

let ops update =
  match Reconciler.message update with
  | Some (Wire.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let prepare t theme view = Reconciler.prepare t ~theme (Some view) |> ok

let commit t theme view =
  let update = prepare t theme view in
  Reconciler.accept t update |> ok;
  ops update
;;

let content () =
  Text_content.create
    "Aé世界"
    ~spans:
      [ Text_content.Span.create
          ~start_byte:1
          ~end_byte:3
          ~foreground:(Color.token_exn "label-match")
        |> ok
      ]
  |> ok
;;

let theme color = Theme.create [ "label-match", Color.rgb_exn color ] |> ok

let%expect_test "op59 independently matches the native transaction fixture" =
  let packet =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_styled_text
              ( node
              , { text = "Aé世界"
                ; spans =
                    [ { start_byte = 1L; end_byte = 3L; foreground = 0x11223344L }
                    ; { start_byte = 3L; end_byte = 9L; foreground = 0xaabbccddL }
                    ]
                } )
          ]
      }
  in
  Wire.Message.encode packet
  |> ok
  |> String.iter ~f:(fun c -> printf "%02x" (Char.to_int c));
  print_endline "";
  [%expect
    {| 0300010001013b00010941c3a9e4b896e7958c020103fd443322110309fcddccbbaa00000000 |}]
;;

let%expect_test
    "resolved runs are retained, theme-sensitive and atomic through plain transitions"
  =
  let t = Reconciler.create window in
  let content = content () in
  let view = View.styled_text ~key content in
  let red = theme 0xff0000
  and blue = theme 0x0000ff in
  let wire theme = Text_content.Expert.to_wire content ~theme |> ok in
  let initial = commit t red view in
  assert (
    List.equal
      Wire.Op.equal
      initial
      [ Create (node, Text, "", None)
      ; Set_styled_text (node, wire red)
      ; Set_root (Some node)
      ]);
  assert (List.is_empty (commit t red view));
  (* A physically shared view still resolves changed tokens. *)
  assert (
    List.equal Wire.Op.equal (commit t blue view) [ Set_styled_text (node, wire blue) ]);
  let plain = View.text ~key "Aé世界" in
  assert (List.equal Wire.Op.equal (commit t blue plain) [ Set_text (node, "Aé世界") ]);
  assert (List.is_empty (commit t blue plain));
  assert (
    List.equal Wire.Op.equal (commit t blue view) [ Set_styled_text (node, wire blue) ]);
  let empty_runs = Text_content.create "Aé世界" |> ok |> View.styled_text ~key in
  assert (
    List.equal
      Wire.Op.equal
      (commit t blue empty_runs)
      [ Set_styled_text (node, { W.text = "Aé世界"; spans = [] }) ]);
  let empty = Text_content.create "" |> ok |> View.styled_text ~key in
  assert (
    List.equal
      Wire.Op.equal
      (commit t blue empty)
      [ Set_styled_text (node, { W.text = ""; spans = [] }) ]);
  print_endline
    "one initial payload; stable identity; no-op reuse; recolor; clear/restore runs; \
     empty source";
  [%expect
    {| one initial payload; stable identity; no-op reuse; recolor; clear/restore runs; empty source |}]
;;

let%expect_test
    "failed or discarded preparation cannot publish text or resolved theme state"
  =
  let t = Reconciler.create window in
  let view = View.styled_text ~key (content ()) in
  let red = theme 0xff0000
  and blue = theme 0x0000ff in
  ignore (commit t red view : Wire.Op.t list);
  let before = Reconciler.revision t in
  let missing = Theme.create [] |> ok in
  assert (Or_error.is_error (Reconciler.prepare t ~theme:missing (Some view)));
  assert (Int64.equal before (Reconciler.revision t));
  let discarded = prepare t blue view in
  assert (List.length (ops discarded) = 1);
  assert (Int64.equal before (Reconciler.revision t));
  assert (List.is_empty (commit t red view));
  assert (List.length (commit t blue view) = 1);
  print_endline
    "failed/discarded preparation leaves acknowledged runs and revision unchanged";
  [%expect
    {| failed/discarded preparation leaves acknowledged runs and revision unchanged |}]
;;

let%expect_test "styled text requires a paired host capability" =
  let capability = Int64.shift_left 1L 47 in
  assert (Int64.equal (Int64.bit_and Wire.capabilities capability) capability);
  let packet = Wire.Message.Hello (Wire.version, Wire.capabilities) in
  Wire.Message.encode packet
  |> ok
  |> String.iter ~f:(fun c -> printf "%02x" (Char.to_int c));
  print_endline "";
  [%expect {| 0001fcffffffffffff0100 |}]
;;
