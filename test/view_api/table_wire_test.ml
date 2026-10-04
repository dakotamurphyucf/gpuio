open Core
module W = Gpuio_protocol.Table_wire
module C = Gpuio.Table_column

let column id pin : W.Column.t =
  { id
  ; label = "Name"
  ; width = 160.
  ; min_width = 40.
  ; max_width = 4096.
  ; pin
  ; alignment = Left
  ; resizable = true
  ; movable = true
  ; sortable = true
  }
;;

let config () : W.Config.t =
  { schema_revision = 7L
  ; query_generation = 3L
  ; schema =
      { columns =
          [ column "a" Left
          ; { (column "β" Unpinned) with label = "Value"; alignment = Right }
          ]
      ; headers =
          [ [ { label = "Identity"; columns = [ "a" ] }
            ; { label = "Payload"; columns = [ "β" ] }
            ]
          ]
      }
  ; sort = Some { column = "β"; direction = Descending }
  ; row_height = 32.
  ; overscan = 64.
  ; max_active_rows = 32L
  ; max_active_cells = 64L
  ; selection_mode = Rows_and_cells
  ; column_selection = true
  ; disabled = false
  ; scrollbar = true
  ; label = "Results"
  }
;;

let hex writer value =
  Bin_prot.Utils.bin_dump writer value
  |> Bigstring.to_string
  |> String.to_list
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "paired table schema, cell, command and native request bytes" =
  let config = config () in
  let cell : W.Cell.t = { column = "β"; copy_text = "日本語👨‍👩‍👧‍👦" } in
  let command : W.Command.t =
    { serial = 9L; query_generation = 3L; target = Set_selection (Cell (42L, "β")) }
  in
  let request = W.Request.Resize [ "a", 160.; "β", 240. ] in
  Eio_main.run (fun env ->
    List.iter
      [ "table-config.hex", hex W.Config.bin_writer_t config
      ; "table-cell.hex", hex W.Cell.bin_writer_t cell
      ; "table-command.hex", hex W.Command.bin_writer_t command
      ; "table-request.hex", hex W.Request.bin_writer_t request
      ]
      ~f:(fun (file, bytes) ->
        let expected =
          Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / file) |> String.strip
        in
        assert (String.equal expected bytes)));
  assert (
    W.Config.valid config
    && W.Cell.valid cell
    && W.Command.valid command
    && W.Request.valid request);
  let columns = C.Expert.of_wire config.schema |> Or_error.ok_exn in
  assert (W.Schema.equal config.schema (C.Expert.to_wire columns));
  print_endline "4 independent wire fixtures; validated Core schema roundtrip";
  [%expect {| 4 independent wire fixtures; validated Core schema roundtrip |}]
;;

let%expect_test "schema limits, grouped pin partitions and active cell product" =
  let base = config () in
  List.iter
    [ { base with schema_revision = 0L }
    ; { base with query_generation = -1L }
    ; { base with max_active_rows = Int64.max_value }
    ; { base with max_active_cells = 63L }
    ; { base with row_height = Float.nan }
    ; { base with label = "\000" }
    ; { base with
        schema = { base.schema with columns = [ column "a" Left; column "a" Unpinned ] }
      }
    ; { base with
        schema =
          { base.schema with
            headers = [ [ { label = "cross-pin"; columns = [ "a"; "β" ] } ] ]
          }
      }
    ; { base with schema = { columns = List.rev base.schema.columns; headers = [] } }
    ; { base with schema = { base.schema with headers = [ [] ] } }
    ; { base with
        schema =
          { base.schema with
            columns =
              List.map base.schema.columns ~f:(fun c -> { c with sortable = false })
          }
      }
    ]
    ~f:(fun config -> assert (not (W.Config.valid config)));
  let schema : W.Schema.t =
    { columns = List.init 64 ~f:(fun i -> column (Int.to_string i) Unpinned)
    ; headers = []
    }
  in
  let maximum =
    { base with schema; sort = None; max_active_rows = 256L; max_active_cells = 16384L }
  in
  assert (W.Config.valid maximum);
  assert (not (W.Config.valid { maximum with max_active_rows = 257L }));
  assert (not (W.Config.valid { maximum with max_active_cells = 16385L }));
  let invalid_schemas =
    [ { schema with columns = schema.columns @ [ column "65" Unpinned ] }
    ; { schema with
        columns =
          List.map schema.columns ~f:(fun column ->
            { column with label = String.make 4096 'a' })
      }
    ; { schema with
        headers =
          List.init 5 ~f:(fun _ ->
            [ { W.Group.label = "all"
              ; columns = List.map schema.columns ~f:(fun column -> column.id)
              }
            ])
      }
    ]
  in
  List.iter invalid_schemas ~f:(fun schema ->
    assert (not (W.Schema.valid schema));
    assert (Or_error.is_error (C.Expert.of_wire schema)));
  let group columns : W.Group.t = { label = "group"; columns } in
  let nested : W.Schema.t =
    { columns = List.take schema.columns 4
    ; headers =
        [ [ group [ "0"; "1" ]; group [ "2"; "3" ] ]
        ; List.map [ "0"; "1"; "2"; "3" ] ~f:(fun id -> group [ id ])
        ]
    }
  in
  assert (W.Schema.valid nested);
  assert (Or_error.is_ok (C.Expert.of_wire nested));
  let invalid = { nested with headers = List.rev nested.headers } in
  assert (not (W.Schema.valid invalid));
  assert (Or_error.is_error (C.Expert.of_wire invalid));
  assert (W.Schema.valid { columns = []; headers = [] });
  print_endline "64 columns × 256 rows bounded at 16384 cells; schema invariants agree";
  [%expect {| 64 columns × 256 rows bounded at 16384 cells; schema invariants agree |}]
;;

let%expect_test "all table intent families and bounded Unicode copy text" =
  let selections : W.Selection.t list = [ Empty; Row 1L; Column "β"; Cell (42L, "β") ] in
  let targets : W.Target.t list =
    [ W.Target.Reveal (42L, None)
    ; Reveal (42L, Some "β")
    ; Scroll_to (42L, 7.5)
    ; Scroll_to_column "β"
    ; Scroll_to_end
    ; Reset_columns
    ]
    @ List.map selections ~f:(fun selection -> W.Target.Set_selection selection)
  in
  List.iter targets ~f:(fun target ->
    assert (W.Command.valid { serial = 1L; query_generation = 0L; target }));
  let requests : W.Request.t list =
    [ W.Request.Activate (42L, None)
    ; Activate (42L, Some "β")
    ; Move ("β", None)
    ; Move ("β", Some "a")
    ; Sort ("β", None)
    ; Sort ("β", Some Ascending)
    ; Sort ("β", Some Descending)
    ]
    @ List.concat_map selections ~f:(fun selection ->
      [ W.Request.Select selection; Context selection; Copy selection ])
  in
  List.iter requests ~f:(fun request -> assert (W.Request.valid request));
  let commands =
    List.map targets ~f:(fun target ->
      { W.Command.serial = 1L; query_generation = 0L; target })
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "table-intents.hex") |> String.strip
    in
    assert (
      String.equal
        expected
        (hex [%bin_writer: W.Command.t list * W.Request.t list] (commands, requests))));
  List.iter
    [ W.Request.Select (Row 0L)
    ; Select (Cell (-1L, "a"))
    ; Select (Column "")
    ; Resize []
    ; Resize [ "a", 40.; "a", 50. ]
    ; Resize [ "a", Float.infinity ]
    ; Move ("a", Some "a")
    ]
    ~f:(fun request -> assert (not (W.Request.valid request)));
  List.iter
    [ ""; String.make W.max_copy_bytes 'x' ]
    ~f:(fun copy_text -> assert (W.Cell.valid { column = "a"; copy_text }));
  List.iter
    [ "\000"; "\255"; String.make (W.max_copy_bytes + 1) 'x' ]
    ~f:(fun copy_text -> assert (not (W.Cell.valid { column = "a"; copy_text })));
  print_s [%sexp (List.length targets : int), (List.length requests : int)];
  [%expect {| (10 19) |}]
;;

let%expect_test
    "public table configuration owns budgets and accepted sort without bridge revisions"
  =
  let module T = Gpuio.Table in
  let columns = C.Expert.of_wire (config ()).schema |> Or_error.ok_exn in
  let config = T.Config.create ~columns ~label:"Results" () |> Or_error.ok_exn in
  assert (T.Config.max_active_rows config = 64);
  assert (T.Config.max_active_cells config = 4096);
  assert (not (T.Config.column_selection config));
  let sort : T.Sort.t =
    { column = C.Id.of_string "β" |> Or_error.ok_exn; direction = Descending }
  in
  let sorted = T.Config.with_sort config (Some sort) |> Or_error.ok_exn in
  let wire =
    T.Expert.to_wire sorted ~schema_revision:7L ~query_generation:3L |> Or_error.ok_exn
  in
  assert (W.Config.valid wire);
  assert (Option.is_none (T.Config.sort config));
  let empty = C.Collection.create [] |> Or_error.ok_exn in
  assert (Or_error.is_error (T.Config.with_columns sorted empty));
  assert (Or_error.is_ok (T.Config.with_columns config empty));
  let bounded =
    T.Config.create ~columns ~label:"Results" ~max_active_cells:16 () |> Or_error.ok_exn
  in
  assert (T.Config.max_active_rows bounded = 8);
  assert (
    Or_error.is_error
      (T.Config.create
         ~columns
         ~label:"Results"
         ~max_active_cells:16
         ~max_active_rows:9
         ()));
  assert (Or_error.is_error (T.Config.create ~columns ~label:"" ()));
  assert (
    Or_error.is_error (T.Config.create ~columns ~label:"Results" ~row_height:Float.nan ()));
  assert (
    Or_error.is_error
      (T.Config.create ~columns ~label:"Results" ~max_active_cells:(-1) ()));
  assert (
    Or_error.is_error (T.Expert.to_wire config ~schema_revision:0L ~query_generation:0L));
  assert (
    Or_error.is_error
      (T.Expert.to_wire config ~schema_revision:1L ~query_generation:(-1L)));
  print_endline "pure config, bounded defaults, immutable sort/schema validation";
  [%expect {| pure config, bounded defaults, immutable sort/schema validation |}]
;;

let%expect_test "table transaction and fenced event envelopes match Rust" =
  let module Wire = Gpuio_protocol.Wire in
  let ok = Or_error.ok_exn in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node slot = Gpuio_protocol.Node_id.create ~slot ~generation:1L |> ok in
  let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok in
  let message =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_table (node 0L, config ())
          ; Set_table_cell (node 1L, { column = "β"; copy_text = "日本語👨‍👩‍👧‍👦" })
          ; Table_command
              ( node 0L
              , { serial = 9L
                ; query_generation = 3L
                ; target = Set_selection (Cell (42L, "β"))
                } )
          ]
      }
  in
  let input : W.Input.t =
    { schema_revision = 7L
    ; query_generation = 3L
    ; request = Resize [ "a", 160.; "β", 240. ]
    }
  in
  let event input = Wire.Event.Table_input (window, node 0L, handler, 1L, input) in
  Eio_main.run (fun env ->
    List.iter
      [ "table-transaction.hex", hex Wire.Message.bin_writer_t message
      ; "table-event.hex", hex Wire.Event.bin_writer_t (event input)
      ]
      ~f:(fun (file, actual) ->
        let expected =
          Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / file) |> String.strip
        in
        assert (String.equal actual expected)));
  let bytes =
    Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] [ event input ]
    |> Bigstring.to_string
  in
  assert (List.equal Wire.Event.equal [ event input ] (Wire.Event.decode bytes |> ok));
  for length = 0 to String.length bytes - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix bytes length)))
  done;
  assert (Result.is_error (Wire.Event.decode (bytes ^ "\000")));
  List.iter
    [ { input with schema_revision = 0L }
    ; { input with query_generation = -1L }
    ; { input with request = Select (Row 0L) }
    ]
    ~f:(fun input ->
      let bytes =
        Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] [ event input ]
        |> Bigstring.to_string
      in
      assert (Result.is_error (Wire.Event.decode bytes)));
  [%expect {| |}]
;;

let%expect_test "managed tables require a distinct negotiated capability" =
  let module Wire = Gpuio_protocol.Wire in
  let capability = 1099511627776L in
  assert (Int64.equal (Int64.bit_and Wire.capabilities capability) capability);
  let bytes = Wire.Message.encode (Hello (Wire.version, capability)) |> Or_error.ok_exn in
  String.iter bytes ~f:(fun byte -> printf "%02x" (Char.to_int byte));
  print_endline "";
  [%expect {| 0003fc0000000000010000 |}]
;;

let%expect_test "paired table behavior extension preserves original schema encoding" =
  let module Wire = Gpuio_protocol.Wire in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let behavior : W.Behavior.t =
    { row_header = false; boundary = Stop; selectable_headers = Some [ "a"; "β" ] }
  in
  assert (W.Behavior.valid behavior);
  Eio_main.run (fun env ->
    List.iter
      [ "table-behavior-operation.hex", Wire.Op.Set_table_behavior (node, Some behavior)
      ; "table-behavior-clear.hex", Wire.Op.Set_table_behavior (node, None)
      ]
      ~f:(fun (file, operation) ->
        let expected =
          Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / file) |> String.strip
        in
        assert (String.equal expected (hex Wire.Op.bin_writer_t operation))));
  let module T = Gpuio.Table in
  let columns = C.Expert.of_wire (config ()).schema |> Or_error.ok_exn in
  let create ?selectable_headers () =
    T.Config.create
      ~columns
      ~label:"Results"
      ~column_selection:true
      ?selectable_headers
      ()
  in
  let base = create () |> Or_error.ok_exn in
  assert (T.Config.row_header base);
  assert (T.Boundary.equal (T.Config.boundary base) Wrap);
  assert (Option.is_none (T.Expert.behavior_to_wire base));
  let a = C.Id.of_string "a" |> Or_error.ok_exn in
  let absent = C.Id.of_string "absent" |> Or_error.ok_exn in
  assert (Or_error.is_error (create ~selectable_headers:[ a; a ] ()));
  assert (Or_error.is_error (create ~selectable_headers:[ absent ] ()));
  let limited = create ~selectable_headers:[ a ] () |> Or_error.ok_exn in
  assert (T.Config.header_selectable limited a);
  assert (not (T.Config.header_selectable limited (C.Id.of_string "β" |> Or_error.ok_exn)));
  assert (
    Or_error.is_error
      (T.Config.with_columns limited (C.Collection.create [] |> Or_error.ok_exn)));
  let cleared = T.Config.with_selectable_headers limited None |> Or_error.ok_exn in
  assert (T.Config.equal base cleared);
  print_endline
    "paired extension; preserved defaults; unique existing headers; explicit reset";
  [%expect
    {| paired extension; preserved defaults; unique existing headers; explicit reset |}]
;;

let%expect_test "table appearance validates padding, colors, schema and paired bytes" =
  let module A = Gpuio.Table.Appearance in
  let module Wire = Gpuio_protocol.Wire in
  let ok = Or_error.ok_exn in
  let pad = A.Padding.create ~top:1. ~right:2. ~bottom:3. ~left:4. |> ok in
  let id = C.Id.of_string "name" |> ok in
  let appearance =
    A.create
      ~striped:true
      ~colors:
        [ ( Header_background
          , Gpuio.Color.rgba ~red:0x33 ~green:0x66 ~blue:0x99 ~alpha:0x80 |> ok )
        ; Header_foreground, Gpuio.Color.rgb_exn 0xffffff
        ]
      ~padding:pad
      ~column_padding:[ id, A.Padding.zero ]
      ()
    |> ok
  in
  let wire = A.Expert.to_wire appearance ~theme:Gpuio.Theme.default |> ok in
  assert (W.Appearance.valid wire);
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  Eio_main.run (fun env ->
    List.iter
      [ "table-appearance-operation.hex", Wire.Op.Set_table_appearance (node, Some wire)
      ; "table-appearance-clear.hex", Wire.Op.Set_table_appearance (node, None)
      ]
      ~f:(fun (file, op) ->
        assert (
          String.equal
            (Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / file) |> String.strip)
            (hex Wire.Op.bin_writer_t op))));
  List.iter [ Float.nan; Float.infinity; -1.; 4097. ] ~f:(fun n ->
    assert (Or_error.is_error (A.Padding.all n)));
  assert (
    Or_error.is_error
      (A.create
         ~colors:[ Row_border, Gpuio.Color.rgb_exn 0; Row_border, Gpuio.Color.rgb_exn 0 ]
         ()));
  assert (Or_error.is_error (A.create ~column_padding:[ id, pad; id, pad ] ()));
  let missing =
    A.create ~colors:[ Stripe_background, Gpuio.Color.token_exn "missing" ] () |> ok
  in
  assert (Or_error.is_error (A.Expert.to_wire missing ~theme:Gpuio.Theme.default));
  let columns = C.Expert.of_wire (config ()).schema |> ok in
  assert (
    Or_error.is_error (Gpuio.Table.Config.create ~columns ~label:"Table" ~appearance ()));
  print_endline
    "paired appearance and reset; bounded padding; unique parts/columns; eager theme \
     resolution";
  [%expect
    {| paired appearance and reset; bounded padding; unique parts/columns; eager theme resolution |}]
;;

let%expect_test "column viewport event fixture and strict envelope validation" =
  let module P = Gpuio_protocol in
  let viewport : W.Column_viewport.t =
    { schema_revision = 7L
    ; query_generation = 3L
    ; columns = [ "a", Left, true; "β", Unpinned, false ]
    }
  in
  let event value =
    P.Wire.Event.Table_columns_observed
      ( P.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
      , P.Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
      , P.Handler_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
      , 1L
      , value )
  in
  let encode value =
    "\001"
    ^ (Bin_prot.Utils.bin_dump P.Wire.Event.bin_writer_t (event value)
       |> Bigstring.to_string)
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "table-columns-event.hex")
      |> String.strip
    in
    assert (String.equal expected (hex P.Wire.Event.bin_writer_t (event viewport))));
  let bytes = encode viewport in
  assert (Result.is_ok (P.Wire.Event.decode bytes));
  for len = 0 to String.length bytes - 1 do
    assert (Result.is_error (P.Wire.Event.decode (String.sub bytes ~pos:0 ~len)))
  done;
  assert (Result.is_error (P.Wire.Event.decode (bytes ^ "\000")));
  List.iter
    [ { viewport with schema_revision = 0L }
    ; { viewport with query_generation = -1L }
    ; { viewport with columns = [ "", Left, true ] }
    ; { viewport with columns = [ "a", Left, true; "a", Left, false ] }
    ; { viewport with columns = List.rev viewport.columns }
    ; { viewport with
        columns = List.init 65 ~f:(fun i -> Int.to_string i, W.Pin.Unpinned, true)
      }
    ]
    ~f:(fun value -> assert (Result.is_error (P.Wire.Event.decode (encode value))));
  print_endline
    "paired Event75; truncation, trailing bytes, identity, pin order and bounds rejected";
  [%expect
    {| paired Event75; truncation, trailing bytes, identity, pin order and bounds rejected |}]
;;
