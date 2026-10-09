open Core
module D = Gpuio.Document.Diff
module W = Gpuio_protocol.Document_diff_wire

let ok = Or_error.ok_exn
let dump writer value = Bin_prot.Utils.bin_dump writer value |> Bigstring.to_string

let hex text =
  String.to_list text
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let decode reader text =
  Or_error.try_with (fun () ->
    let buffer = Bigstring.of_string text in
    let pos_ref = ref 0 in
    let t = reader buffer ~pos_ref in
    if !pos_ref <> Bigstring.length buffer then failwith "trailing bytes";
    t)
;;

let reject_prefixes reader text =
  for length = 0 to String.length text - 1 do
    assert (Result.is_error (decode reader (String.prefix text length)))
  done;
  assert (Result.is_error (decode reader (text ^ "\000")))
;;

let configs =
  [ D.Config.default
  ; D.Config.create
      ~collapse:(Controlled [ Path "src/λ.ml"; Unnamed ])
      ~line_limit:(Controlled (Some 0))
      ~word_diff:false
      ()
    |> ok
  ; D.Config.create
      ~collapse:(Managed { initially_collapsed = [ Path "a" ] })
      ~line_limit:(Managed { initial = Some 130; step = 129 })
      ()
    |> ok
  ]
;;

let%expect_test "standalone config fixtures pair with Rust" =
  List.iter configs ~f:(fun config ->
    let wire = D.Expert.to_wire config in
    let bytes = dump W.Config.bin_writer_t wire in
    let decoded = decode W.Config.bin_read_t bytes |> ok in
    assert (D.Config.equal config (D.Expert.of_wire decoded |> ok));
    reject_prefixes W.Config.bin_read_t bytes;
    print_endline (hex bytes));
  [%expect
    {|
    00000000fec80001
    010200097372632fcebb2e6d6c0101010000
    00010001610001fe8200fe810001
    |}]
;;

let file : W.File.t =
  { index = 1L; key = Path "λ"; before_path = Some "old"; after_path = Some "λ" }
;;

let event observation : W.Event.t =
  { config_epoch = 2L; source_revision = 7L; source_generation = 3L; observation }
;;

let observations : W.Observation.t list =
  [ Toggle_file { file; collapsed = true; applied = false }
  ; Show_more { visible = 0L; hidden = 7L; applied_limit = None }
  ; Show_more { visible = 130L; hidden = 9L; applied_limit = Some 330L }
  ; Line
      { file
      ; before = Some 10L
      ; after = Some 20L
      ; start_byte = 50L
      ; end_byte = 52L
      ; text = "λ"
      }
  ; Line
      { file = { index = 0L; key = Unnamed; before_path = None; after_path = None }
      ; before = None
      ; after = None
      ; start_byte = 1L
      ; end_byte = 1L
      ; text = ""
      }
  ]
;;

let%expect_test "revisioned event fixtures preserve payload and paired lines" =
  List.iter observations ~f:(fun observation ->
    let value = event observation in
    let bytes = dump W.Event.bin_writer_t value in
    assert (W.Event.equal value (decode W.Event.bin_read_t bytes |> ok));
    reject_prefixes W.Event.bin_read_t bytes;
    print_endline (hex bytes));
  [%expect
    {|
    02070300010002cebb01036f6c640102cebb0100
    02070301000700
    02070301fe82000901fe4a01
    02070302010002cebb01036f6c640102cebb010a0114323402cebb
    02070302000100000000010100
    |}]
;;

let%expect_test "configuration bounds and exact encoded-size limit" =
  List.iter
    [ ""; "\255"; "bad\000path"; String.make 4097 'x' ]
    ~f:(fun path ->
      assert (Result.is_error (D.Config.create ~collapse:(Controlled [ Path path ]) ())));
  assert (Result.is_error (D.Config.create ~collapse:(Controlled [ Unnamed; Unnamed ]) ()));
  List.iter [ -1; 8193 ] ~f:(fun n ->
    assert (Result.is_error (D.Config.create ~line_limit:(Controlled (Some n)) ())));
  List.iter [ 0; 8193 ] ~f:(fun step ->
    assert (
      Result.is_error (D.Config.create ~line_limit:(Managed { initial = None; step }) ())));
  List.iter [ 0; 8192 ] ~f:(fun n ->
    assert (Result.is_ok (D.Config.create ~line_limit:(Controlled (Some n)) ())));
  let keys = List.init 8192 ~f:(fun i -> D.File_key.Path (Int.to_string i)) in
  assert (Result.is_ok (D.Config.create ~collapse:(Controlled keys) ()));
  assert (Result.is_error (D.Config.create ~collapse:(Controlled (Unnamed :: keys)) ()));
  let prefix =
    List.init 63 ~f:(fun i -> D.File_key.Path (sprintf "%04d%s" i (String.make 4092 'x')))
  in
  let create length =
    D.Config.create
      ~collapse:(Controlled (prefix @ [ Path (String.make length 'y') ]))
      ~line_limit:(Controlled None)
      ()
  in
  let exact = create 3835 |> ok |> D.Expert.to_wire in
  let bytes = dump W.Config.bin_writer_t exact in
  assert (String.length bytes = 262144);
  assert (Result.is_ok (decode W.Config.bin_read_t bytes));
  assert (Result.is_error (create 3836));
  let oversized : W.Config.t =
    { exact with collapse = Controlled (W.Collapse.keys exact.collapse @ [ Unnamed ]) }
  in
  assert (
    Result.is_error (decode W.Config.bin_read_t (dump W.Config.bin_writer_t oversized)));
  (* An impossible declared count is rejected before reading/allocating keys. *)
  assert (
    Result.is_error
      (decode W.Config.bin_read_t "\000\252\255\255\255\255\255\255\255\127"));
  print_endline "UTF-8, unique keys,8192 keys/rows, steps and exact256KiB bound";
  [%expect {| UTF-8, unique keys,8192 keys/rows, steps and exact256KiB bound |}]
;;

let%expect_test "event invariants and managed/controlled intent semantics" =
  let invalid observation =
    assert (
      Result.is_error
        (decode W.Event.bin_read_t (dump W.Event.bin_writer_t (event observation))))
  in
  invalid
    (Toggle_file
       { file = { file with key = Path "wrong" }; collapsed = true; applied = false });
  invalid (Show_more { visible = 0L; hidden = 0L; applied_limit = None });
  invalid (Show_more { visible = 8191L; hidden = 2L; applied_limit = None });
  invalid (Show_more { visible = 10L; hidden = 2L; applied_limit = Some 10L });
  let line : W.Line.t =
    { file
    ; before = Some 10L
    ; after = Some 20L
    ; start_byte = 50L
    ; end_byte = 52L
    ; text = "λ"
    }
  in
  List.iter
    [ { line with before = Some 0L }
    ; { line with after = Some 2147483648L }
    ; { line with text = "a" }
    ; { line with text = "\255x" }
    ; { line with text = "a\n" }
    ; { line with start_byte = -1L }
    ; { line with end_byte = 262145L }
    ]
    ~f:(fun line -> invalid (Line line));
  List.iter [ 0L; -1L ] ~f:(fun n ->
    let value = event (Line line) in
    List.iter
      [ { value with config_epoch = n }
      ; { value with source_revision = n }
      ; { value with source_generation = n }
      ]
      ~f:(fun value ->
        assert (
          Result.is_error (decode W.Event.bin_read_t (dump W.Event.bin_writer_t value)))));
  let controlled =
    D.Config.create ~collapse:(Controlled []) ~line_limit:(Controlled (Some 0)) () |> ok
  in
  let toggle applied collapsed = event (Toggle_file { file; collapsed; applied }) in
  assert (Result.is_ok (D.Expert.event_of_wire ~config:controlled (toggle false true)));
  assert (Result.is_error (D.Expert.event_of_wire ~config:controlled (toggle true true)));
  assert (Result.is_error (D.Expert.event_of_wire ~config:controlled (toggle false false)));
  assert (
    Result.is_ok (D.Expert.event_of_wire ~config:D.Config.default (toggle true false)));
  assert (
    Result.is_error (D.Expert.event_of_wire ~config:D.Config.default (toggle false false)));
  let show visible applied_limit =
    event (Show_more { visible; hidden = 7L; applied_limit })
  in
  assert (Result.is_ok (D.Expert.event_of_wire ~config:controlled (show 0L None)));
  assert (Result.is_error (D.Expert.event_of_wire ~config:controlled (show 1L None)));
  assert (
    Result.is_ok (D.Expert.event_of_wire ~config:D.Config.default (show 20L (Some 220L))));
  assert (
    Result.is_error
      (D.Expert.event_of_wire ~config:D.Config.default (show 20L (Some 21L))));
  let converted = D.Expert.event_of_wire ~config:controlled (event (Line line)) |> ok in
  print_s [%sexp (converted : D.Event.t)];
  [%expect
    {|
    ((source_revision 7) (source_generation 3)
     (observation
      (Line
       ((file
         ((index 1) (key (Path "\206\187")) (before_path (old))
          (after_path ("\206\187"))))
        (before (10)) (after (20)) (start_byte 50) (end_byte 52)
        (text "\206\187")))))
    |}]
;;
