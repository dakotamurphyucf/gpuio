open Core
module L = Gpuio_protocol.List_wire

let%expect_test "logical row runs are compact and validated before expansion" =
  let order : L.Order.t =
    { revision = 1L; runs = [ { first = 1L; count = 100_000L } ] }
  in
  L.Order.validate order |> Or_error.ok_exn;
  let bytes = Bin_prot.Utils.bin_dump L.Order.bin_writer_t order |> Bigstring.to_string in
  let hex =
    String.to_list bytes |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  in
  print_s [%sexp (String.concat hex : string)];
  List.iter
    [ [ { L.Id_run.first = Int64.max_value; count = 2L } ]
    ; [ { first = 1L; count = Int64.max_value } ]
    ; [ { first = 0L; count = 1L } ]
    ; [ { first = 1L; count = 0L } ]
    ; [ { first = 1L; count = 3L }; { first = 3L; count = 2L } ]
    ]
    ~f:(fun runs -> assert (Or_error.is_error (L.Order.validate { revision = 1L; runs })));
  L.Order.validate
    { revision = 1L; runs = [ { first = 100L; count = 3L }; { first = 1L; count = 3L } ] }
  |> Or_error.ok_exn;
  [%expect {| 010101fda0860100 |}]
;;

let%expect_test "list transaction encoding fixes operation tags and field order" =
  let open Gpuio_protocol in
  let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let root = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let row = Node_id.create ~slot:1L ~generation:1L |> Or_error.ok_exn in
  let message =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (root, Virtual_list, "", None)
          ; Create (row, Text, "row", None)
          ; Set_list_config
              ( root
              , { L.Config.estimated_height = 100.
                ; overscan = 200.
                ; max_active = 32L
                ; scroll_policy = Follow_tail_when_at_end
                ; scrollbar = true
                ; managed = true
                } )
          ; Set_list_order
              ( root
              , { L.Order.revision = 1L; runs = [ { first = 1L; count = 100_000L } ] } )
          ; Set_list_rows (root, [ { L.Row.id = 1L; node = row } ])
          ; Splice (root, 0L, 0L, [ row ])
          ; Set_root (Some root)
          ; Invalidate_list_rows (root, [ 1L; 3L ])
          ; Scroll_list
              (root, { L.Scroll_request.serial = 17L; target = Offset (3L, 37.5) })
          ]
      }
  in
  let bytes = Wire.Message.encode message |> Or_error.ok_exn in
  let hex =
    String.to_list bytes |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  in
  print_endline (String.concat hex);
  [%expect
    {| 0300010001090000011900000001010103726f77001c000100000000000059400000000000006940200101011d0001010101fda08601001e0001010101010500010000010101060100011f00010201032000011100030000000000c04240 |}]
;;

let%expect_test "list viewport events validate the data generation and bounded identities"
  =
  let open Gpuio_protocol in
  let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let handler = Handler_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let viewport : L.Viewport.t =
    { order_revision = 1L
    ; visible_first = 0L
    ; visible_last = 3L
    ; requested = [ 1L; 2L; 3L ]
    ; pinned = [ 1L ]
    ; anchor = Some (1L, 4.)
    ; following_tail = false
    ; at_start = false
    ; at_end = false
    ; budget_exhausted = false
    }
  in
  let encode viewport =
    Bin_prot.Utils.bin_dump
      [%bin_writer: Wire.Event.t list]
      [ List_viewport (window, node, handler, 1L, viewport) ]
    |> Bigstring.to_string
  in
  let bytes = encode viewport in
  let decoded = Wire.Event.decode bytes |> Or_error.ok_exn in
  assert (List.length decoded = 1);
  print_endline
    (String.to_list bytes
     |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
     |> String.concat);
  List.iter
    [ { viewport with order_revision = 0L }
    ; { viewport with requested = [ 1L; 1L ] }
    ; { viewport with pinned = [ -1L ] }
    ; { viewport with visible_first = 4L; visible_last = 3L }
    ; { viewport with anchor = Some (1L, Float.nan) }
    ]
    ~f:(fun bad -> assert (Or_error.is_error (Wire.Event.decode (encode bad))));
  [%expect {| 011b000100010001010100030301020301010101000000000000104000000000 |}]
;;
