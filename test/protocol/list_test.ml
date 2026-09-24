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
