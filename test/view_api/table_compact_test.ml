open Core
open Gpuio
module W = Gpuio_protocol.Wire
module R = Reconciler

let ok = Or_error.ok_exn
let key = Key.of_string_exn
let column n = Table_column.Id.of_string (Int.to_string n) |> ok
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

let operations update =
  match R.message update with
  | Some (W.Message.Apply transaction) -> transaction.operations
  | None -> []
  | Some _ -> assert false
;;

let%expect_test "64-column text viewport replaces and streams within atomic limits" =
  let columns =
    List.init 64 ~f:(fun n ->
      Table_column.create ~id:(column n) ~label:(Int.to_string n) () |> ok)
    |> Table_column.Collection.create
    |> ok
  in
  let config =
    Table.Config.create
      ~columns
      ~label:"Wide"
      ~max_active_rows:32
      ~max_active_cells:2048
      ()
    |> ok
  in
  let order =
    List.init 100_000 ~f:(fun n -> key (Int.to_string n))
    |> Virtual_list.Order.create
    |> ok
  in
  let view ~first ~suffix ~rich =
    View.Expert.managed_table
      ~config
      ~query_generation:0L
      ~order
      ~on_input:(fun _ -> ())
      ~on_viewport:(fun _ -> ())
      ~on_retain:(fun _ -> ())
      (List.init 28 ~f:(fun row ->
         ( key (Int.to_string (first + row))
         , List.init 64 ~f:(fun n ->
             let copy_text = sprintf "%d/%d 日本語%s" (first + row) n suffix in
             let cell = Table.Cell.create ~column:(column n) ~copy_text |> ok in
             let view =
               if rich && row = 0 && n = 0
               then View.text copy_text
               else View.Expert.table_text cell
             in
             cell, view) )))
    |> ok
  in
  let reconciler = R.create window in
  let apply ~first ~suffix ~rich =
    let update =
      R.prepare reconciler ~theme:Theme.default (Some (view ~first ~suffix ~rich)) |> ok
    in
    let ops = operations update in
    assert (List.length ops <= 4096);
    R.accept reconciler update |> ok;
    ops
  in
  let initial = apply ~first:0 ~suffix:"" ~rich:false in
  let replacement = apply ~first:50_000 ~suffix:"" ~rich:false in
  let streamed = apply ~first:50_000 ~suffix:"!" ~rich:false in
  let count ops predicate = List.count ops ~f:predicate in
  print_s
    [%sexp
      (count initial (function
         | W.Op.Create_table_text _ -> true
         | _ -> false)
       : int)
    , (count replacement (function
         | W.Op.Create_table_text _ -> true
         | _ -> false)
       : int)
    , (List.length streamed : int)
    , (List.for_all streamed ~f:(function
         | W.Op.Set_table_text _ -> true
         | _ -> false)
       : bool)];
  [%expect {| (1792 1792 1792 true) |}];
  (* Rich/plain replacement retires only that cell's incompatible owner. *)
  let rich = apply ~first:50_000 ~suffix:"!" ~rich:true in
  let plain = apply ~first:50_000 ~suffix:"!" ~rich:false in
  print_s
    [%sexp
      (count rich (function
         | W.Op.Set_table_cell _ -> true
         | _ -> false)
       : int)
    , (count plain (function
         | W.Op.Create_table_text _ -> true
         | _ -> false)
       : int)];
  [%expect {| (1 1) |}]
;;

let%expect_test "compact text operations have fixed paired bytes" =
  let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:1L |> ok in
  let cell : Gpuio_protocol.Table_wire.Cell.t = { column = "v"; copy_text = "λ" } in
  List.iter
    [ W.Op.Create_table_text (node, cell); Set_table_text (node, cell) ]
    ~f:(fun op ->
      let bytes = Bin_prot.Utils.bin_dump W.Op.bin_writer_t op |> Bigstring.to_string in
      print_endline (String.concat_map bytes ~f:(fun c -> sprintf "%02x" (Char.to_int c))));
  [%expect
    {|
    7b0101017602cebb
    7c0101017602cebb
    |}]
;;
