open Core
open Gpuio
module T = Table_view
module A = Accessibility
module W = Gpuio_protocol.Accessibility_wire

let ok = Or_error.ok_exn
let key = Key.of_string_exn

let%expect_test
    "structural semantic roles have paired bytes and checked zero-based coordinates"
  =
  let cell ~row ~column ~column_span =
    A.Table_cell.create ~row ~column ~column_span () |> ok
  in
  let roles : A.Role.t list =
    [ Table (A.Table_info.create ~rows:3 ~columns:4 () |> ok)
    ; Row_group
    ; Table_row 1
    ; Table_cell (cell ~row:1 ~column:0 ~column_span:2)
    ; Column_header (cell ~row:0 ~column:0 ~column_span:4)
    ; Row_header (cell ~row:1 ~column:0 ~column_span:1)
    ; Caption
    ]
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "accessibility-structural-table.hex")
      |> String.split_lines
    in
    List.iter2_exn roles expected ~f:(fun role expected ->
      let config = A.create ~role () |> ok |> A.Expert.to_wire in
      let hex =
        Bin_prot.Utils.bin_dump W.Config.bin_writer_t config
        |> Bigstring.to_string
        |> String.to_list
        |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
        |> String.concat
      in
      assert (String.equal expected hex);
      assert (
        Result.is_error
          (View.with_accessibility (View.text "wrong owner") (A.create ~role () |> ok)))));
  List.iter [ -1; 1_000_000 ] ~f:(fun index ->
    assert (Result.is_error (A.create ~role:(Table_row index) ())));
  assert (Result.is_error (A.Table_info.create ~rows:(-1) ()));
  assert (Result.is_error (A.Table_info.create ~columns:1025 ()));
  assert (Result.is_error (A.Table_cell.create ~row:0 ~column:1023 ~column_span:2 ()));
  assert (Result.is_error (A.Table_cell.create ~row:0 ~column:0 ~column_span:0 ()));
  assert (Result.is_ok (A.Table_info.create ~rows:0 ~columns:0 ()));
  print_endline
    "7 paired roles; zero/unknown totals valid; invalid counts/indices/spans/owners \
     rejected";
  [%expect
    {| 7 paired roles; zero/unknown totals valid; invalid counts/indices/spans/owners rejected |}]
;;

let cell ?kind ?span name content =
  T.Cell.create ~key:(key name) ?kind ?span content |> ok
;;

let row name cells = T.Row.create ~key:(key name) cells |> ok
let section name rows = T.Section.create ~key:(key name) rows |> ok

let table ?(reverse = false) () =
  let rows =
    [ row
        "first"
        [ cell ~span:2 "name" [ View.text "Merged" ]
        ; cell "action" [ View.button ~key:(key "run") ~on_click:(fun () -> ()) "Run" ]
        ]
    ; row "second" [ cell ~span:3 "total" [ View.text "Total" ] ]
    ]
  in
  T.create
    ~columns:3
    ~label:"Results"
    ~header:
      (section
         "labels"
         [ row
             "header"
             [ cell ~kind:Column_header ~span:3 "title" [ View.text "Summary" ] ]
         ])
    ~footer:(section "foot" [ row "foot" [ cell ~span:3 "end" [ View.text "End" ] ] ])
    ~caption:(View.text "Caption")
    [ section "data" (if reverse then List.rev rows else rows) ]
  |> ok
;;

let rec roles view =
  let d = View.Expert.describe view in
  Option.to_list (Option.bind d.accessibility ~f:(fun a -> (A.Expert.to_wire a).role))
  @ List.concat_map d.children ~f:roles
;;

let%expect_test
    "table composition derives global indices and spanning geometry with keyed controls"
  =
  let view = table () in
  let rec check_geometry view =
    let d = View.Expert.describe view in
    (match Option.bind d.accessibility ~f:(fun a -> (A.Expert.to_wire a).role) with
     | Some (W.Role.Table_cell c | Column_header c | Row_header c) ->
       let fields =
         Style.Expert.to_wire d.style ~theme:Theme.default
         |> ok
         |> List.concat_map ~f:(function
           | Gpuio_protocol.Wire.Style.Fields fields -> fields
           | _ -> [])
       in
       assert (
         List.exists fields ~f:(function
           | Gpuio_protocol.Wire.Field.Grid_location
               { column = { start = Line start; end_ = Span span }; row = _ } ->
             Int64.equal start (Int64.of_int (c.column + 1))
             && Int64.equal span (Int64.of_int c.column_span)
           | _ -> false))
     | _ -> ());
    List.iter d.children ~f:check_geometry
  in
  check_geometry view;
  let cells =
    List.filter_map (roles view) ~f:(function
      | W.Role.Table_cell c | Column_header c -> Some (c.row, c.column, c.column_span)
      | _ -> None)
  in
  print_s [%sexp (cells : (int * int * int) list)];
  assert (
    List.exists (roles view) ~f:(function
      | W.Role.Table { rows = Some 4; columns = Some 3 } -> true
      | _ -> false));
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let first = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
  Reconciler.accept reconciler first |> ok;
  let second =
    Reconciler.prepare reconciler ~theme:Theme.default (Some (table ~reverse:true ()))
    |> ok
  in
  (match Reconciler.message second with
   | Some (Gpuio_protocol.Wire.Message.Apply tx) ->
     assert (
       not
         (List.exists tx.operations ~f:(function
            | Create _ | Remove _ -> true
            | _ -> false)))
   | _ -> assert false);
  let bad_row = section "bad" [ row "bad" [ cell "one" [ View.text "one" ] ] ] in
  assert (Result.is_error (T.create ~columns:2 ~label:"bad" [ bad_row ]));
  assert (Result.is_error (T.create ~columns:1 ~label:"bad" [ bad_row; bad_row ]));
  assert (Result.is_error (T.Row.create ~key:(key "dup") [ cell "a" []; cell "a" [] ]));
  let empty = T.create ~columns:3 ~label:"Empty" [] |> ok in
  assert (
    List.equal W.Role.equal (roles empty) [ Table { rows = Some 0; columns = Some 3 } ]);
  print_endline
    "header/body/footer indices; keyed reorder retains controls; invalid coverage/keys \
     rejected; empty table has no fake row";
  [%expect
    {|
    ((0 0 3) (1 0 2) (1 2 1) (2 0 3) (3 0 3))
    header/body/footer indices; keyed reorder retains controls; invalid coverage/keys rejected; empty table has no fake row
    |}]
;;
