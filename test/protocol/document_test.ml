open Core
open Gpuio_protocol
module D = Wire.Document

let%expect_test "document upload schema has independent Rust fixtures" =
  let id = Resource_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let requests : D.Request.t list =
    [ Create
    ; Begin
        { id
        ; base = 0L
        ; revision = 1L
        ; generation = 1L
        ; from_byte = 0L
        ; suffix_bytes = 2L
        ; status = Streaming
        }
    ; Chunk (id, 1L, 0L, "\206\187")
    ; Publish (id, 1L)
    ; Abort (id, 1L)
    ; Release id
    ]
  in
  List.iter requests ~f:(fun request ->
    let bytes = Wire.Message.encode (Document (7L, request)) |> Or_error.ok_exn in
    let hex =
      String.to_list bytes
      |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
      |> String.concat
    in
    print_s [%sexp (hex : string)]);
  [%expect
    {|
    0a0700
    0a07010001000101000200
    0a07020001010002cebb
    0a0703000101
    0a0704000101
    0a07050001
    |}]
;;

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "document presentation and navigation field order" =
  let source = Resource_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let handler = Handler_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let config : D.Config.t =
    { source = Some source
    ; mode = Markdown
    ; dark = false
    ; layout = Flow
    ; label = "d"
    ; path = None
    ; line_numbers = true
    ; initially_collapsed = false
    ; search = ""
    ; images = []
    }
  in
  List.iter
    [ config
    ; { config with
        mode = Code "ml"
      ; dark = true
      ; layout = Viewport 300.
      ; path = Some "a.ml"
      ; search = "λ"
      }
    ; { config with source = None; mode = Diff; initially_collapsed = true }
    ]
    ~f:(fun config ->
      Wire.Message.encode
        (Apply
           { window
           ; base = 0L
           ; revision = 1L
           ; operations =
               [ Create (node, Document_view, "", Some handler)
               ; Set_document (node, config)
               ; Set_root (Some node)
               ]
           })
      |> Or_error.ok_exn
      |> hex
      |> print_endline);
  let encode generation navigation =
    Bin_prot.Utils.bin_dump
      [%bin_writer: Wire.Event.t list]
      [ Document_navigation (window, node, handler, 1L, source, generation, navigation) ]
    |> Bigstring.to_string
  in
  List.iter
    [ D.Navigation.Link "https://example.test"; Line (Some "a.ml", Before, 9L) ]
    ~f:(fun navigation ->
      let bytes = encode 3L navigation in
      assert (Or_error.is_ok (Wire.Event.decode bytes));
      print_endline (hex bytes));
  assert (Or_error.is_error (Wire.Event.decode (encode 0L (Link "x"))));
  assert (Or_error.is_error (Wire.Event.decode (encode 1L (Line (None, After, 0L)))));
  [%expect
    {|
    0300010001030000011a000100012100010100010000000164000100000006010001
    0300010001030000011a0001000121000101000101026d6c01010000000000c0724001640104612e6d6c010002cebb0006010001
    0300010001030000011a00010001210001000200000164000101000006010001
    011e00010001000101000103001468747470733a2f2f6578616d706c652e74657374
    011e00010001000101000103010104612e6d6c0009
    |}]
;;
