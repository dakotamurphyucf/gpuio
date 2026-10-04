open Core
open Gpuio
module H = Table_header
module C = Table_column
module W = Gpuio_protocol.Table_header_wire

let ok = Or_error.ok_exn
let id value = C.Id.of_string value |> ok
let key = Key.of_string_exn
let column name = C.create ~id:(id name) ~label:"Repeated label" () |> ok

let group names =
  C.Group.create ~label:"Repeated label" ~columns:(List.map names ~f:id) |> ok
;;

let target level names = H.Target.group ~level ~columns:(List.map names ~f:id) |> ok
let slot name target = H.create ~key:(key name) ~target (fun () -> name)

let schema () =
  C.Collection.create
    ~header_groups:
      [ [ group [ "a"; "β" ]; group [ "c" ] ]
      ; [ group [ "a" ]; group [ "β" ]; group [ "c" ] ]
      ]
    (List.map [ "a"; "β"; "c" ] ~f:column)
  |> ok
;;

let%expect_test
    "header addresses have paired canonical bytes and reject invalid raw metadata"
  =
  let targets = [ H.Target.column (id "a"); target 0 [ "β"; "a" ] ] in
  Eio_main.run (fun env ->
    let fixtures =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "table-header-targets.hex")
      |> String.split_lines
    in
    List.iter2_exn targets fixtures ~f:(fun target expected ->
      let wire = H.Expert.target_to_wire target in
      let bytes = Bin_prot.Utils.bin_dump W.bin_writer_t wire |> Bigstring.to_string in
      let hex =
        String.to_list bytes
        |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
        |> String.concat
      in
      assert (String.equal hex expected);
      assert (H.Target.equal target (H.Expert.target_of_wire wire |> ok))));
  List.iter
    [ W.Column ""
    ; Group { level = -1; columns = [ "a" ] }
    ; Group { level = 4; columns = [ "a" ] }
    ; Group { level = 0; columns = [] }
    ; Group { level = 0; columns = [ "a"; "a" ] }
    ; Group { level = 0; columns = [ "β"; "a" ] }
    ]
    ~f:(fun wire -> assert (Result.is_error (H.Expert.target_of_wire wire)));
  assert (Result.is_error (H.Target.group ~level:0 ~columns:[ id "a"; id "a" ]));
  assert (
    Result.is_error
      (H.Target.group ~level:0 ~columns:(List.init 65 ~f:(fun n -> id (Int.to_string n)))));
  print_endline
    "Paired Unicode targets; public construction canonicalizes; raw noncanonical or \
     invalid metadata rejected";
  [%expect
    {| Paired Unicode targets; public construction canonicalizes; raw noncanonical or invalid metadata rejected |}]
;;

let%expect_test
    "content keys survive reorder while target membership and levels remain exact"
  =
  let columns = schema () in
  let member = target 0 [ "a"; "β" ] in
  let header = slot "stable-control" member in
  let moved = C.Collection.move columns ~column:(id "β") ~before:(Some (id "a")) |> ok in
  assert (H.Target.equal member (target 0 [ "β"; "a" ]));
  List.iter [ columns; moved ] ~f:(fun columns ->
    assert (Result.is_ok (H.validate_all [ header ] ~columns)));
  assert (String.equal (H.content header ()) "stable-control");
  assert (not (H.Target.matches (target 0 [ "a" ]) columns));
  assert (H.Target.matches (target 1 [ "a" ]) columns);
  assert (not (H.Target.matches (target 2 [ "a" ]) columns));
  assert (not (H.Target.matches (target 0 [ "a"; "c" ]) columns));
  List.iter
    [ [ header; slot "another-key" member ]
    ; [ header; slot "stable-control" (H.Target.column (id "a")) ]
    ; [ slot "missing" (H.Target.column (id "missing")) ]
    ; [ slot "partial-group" (target 0 [ "a" ]) ]
    ]
    ~f:(fun headers -> assert (Result.is_error (H.validate_all headers ~columns)));
  print_endline
    "Reorder retains exact group address and caller key; repeated labels do not alias \
     groups or levels";
  [%expect
    {| Reorder retains exact group address and caller key; repeated labels do not alias groups or levels |}]
;;

let%expect_test "maximum headers are bounded separately from active body cells" =
  let names = List.init 64 ~f:(fun n -> sprintf "c%02d" n) in
  let columns =
    C.Collection.create
      ~header_groups:
        (List.init 4 ~f:(fun _ -> List.map names ~f:(fun name -> group [ name ])))
      (List.map names ~f:column)
    |> ok
  in
  let targets =
    List.map names ~f:(fun name -> H.Target.column (id name))
    @ List.concat_map (List.init 4 ~f:Fn.id) ~f:(fun level ->
      List.map names ~f:(fun name -> target level [ name ]))
  in
  let headers = List.mapi targets ~f:(fun i target -> slot (Int.to_string i) target) in
  assert (Result.is_ok (H.validate_all headers ~columns));
  assert (Result.is_error (H.validate_all (List.hd_exn headers :: headers) ~columns));
  print_s [%sexp (List.length headers : int)];
  [%expect {| 320 |}]
;;

let%expect_test "appended header operation has independent paired bytes" =
  let module Wire = Gpuio_protocol.Wire in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  Eio_main.run (fun env ->
    List.iter
      [ ( "table-header-operation.hex"
        , Some (H.Expert.target_to_wire (target 0 [ "a"; "β" ])) )
      ; "table-header-clear.hex", None
      ]
      ~f:(fun (file, target) ->
        let operation = Wire.Op.Set_table_header (node, target) in
        let bytes =
          Bin_prot.Utils.bin_dump Wire.Op.bin_writer_t operation |> Bigstring.to_string
        in
        let hex =
          String.to_list bytes
          |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
          |> String.concat
        in
        let expected =
          Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / file) |> String.strip
        in
        assert (String.equal hex expected)));
  print_endline "Set and clear retain operation tag 113 and generation-checked identity";
  [%expect {| Set and clear retain operation tag 113 and generation-checked identity |}]
;;
