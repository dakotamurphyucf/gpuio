open Core
module V = Gpuio.Input_validation
module W = Gpuio_protocol.Input_validation_wire

let%expect_test "cancelled preparation leaves the preparer usable" =
  Eio_main.run (fun _ ->
    let source = V.Regex.Source.create "[a-z]*" |> Or_error.ok_exn in
    let cancelled =
      Eio.Cancel.sub (fun context ->
        Eio.Cancel.cancel context Exit;
        match Gpuio_eio.Input_validation.prepare_regex source with
        | _ -> false
        | exception Eio.Cancel.Cancelled _ -> true)
    in
    assert cancelled;
    assert (Result.is_ok (Gpuio_eio.Input_validation.prepare_regex source)));
  print_endline "cancellation is not converted into a validation failure";
  [%expect {| cancellation is not converted into a validation failure |}]
;;

let ok = Or_error.ok_exn

let unhex hex =
  String.init
    (String.length hex / 2)
    ~f:(fun i ->
      Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
;;

let%expect_test "prepared edit filters encode operation 77 and preserve editor identity" =
  let open Gpuio in
  let module Wire = Gpuio_protocol.Wire in
  Eio_main.run (fun env ->
    let prepare ?matching ?case_sensitive ?allow_empty pattern =
      let source = V.Regex.Source.create ?matching ?case_sensitive pattern |> ok in
      let regex =
        match Gpuio_eio.Input_validation.prepare_regex source with
        | Ok regex -> regex
        | Error error -> raise_s [%sexp (error : V.Error.t)]
      in
      V.regex ?allow_empty regex
    in
    let digits = prepare "[0-9]*" in
    let letters =
      prepare ~matching:Substring ~case_sensitive:false ~allow_empty:false "\\p{L}+"
    in
    let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
    let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:2L |> ok in
    let rules = [ None; Some digits; Some letters ] in
    let message =
      Wire.Message.Apply
        { window
        ; base = 0L
        ; revision = 1L
        ; operations =
            List.map rules ~f:(fun rule ->
              Wire.Op.Set_editor_validation (node, Option.map rule ~f:V.Expert.to_wire))
        }
    in
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "input-validation-operation.hex")
      |> String.strip
      |> unhex
    in
    assert (String.equal (Wire.Message.encode message |> ok) expected);
    assert (
      Result.is_error
        (Text_input.Config.create ~mode:Multiline ~label:"Draft" ~edit_filter:digits ()));
    let plain = Text_input.Config.create ~mode:Single_line ~label:"Draft" () |> ok in
    let filtered =
      Text_input.Config.create ~mode:Single_line ~label:"Draft" ~edit_filter:digits ()
      |> ok
    in
    assert (
      Wire.Editor.Config.equal
        (Text_input.Expert.config_to_wire plain)
        (Text_input.Expert.config_to_wire filtered));
    let reconciler = Reconciler.create window in
    let render edit_filter =
      let config =
        Text_input.Config.create ~mode:Single_line ~label:"Draft" ?edit_filter () |> ok
      in
      let view =
        View.text_input
          ~controller:(Key.of_string_exn "filtered-draft")
          ~config
          ~initial_text:"incompatible retained seed"
          ~on_event:(fun _ -> ())
          ()
        |> ok
      in
      let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
      let message = Reconciler.message update in
      Reconciler.accept reconciler update |> ok;
      match message with
      | Some (Wire.Message.Apply tx) -> tx.operations
      | _ -> []
    in
    let mounted =
      List.find_map_exn (render None) ~f:(function
        | Wire.Op.Create (id, Input, _, _) -> Some id
        | _ -> None)
    in
    List.iter [ Some digits; Some letters; None ] ~f:(fun rule ->
      assert (
        List.equal
          Wire.Op.equal
          (render rule)
          [ Set_editor_validation (mounted, Option.map rule ~f:V.Expert.to_wire) ]));
    assert (List.is_empty (render None)));
  print_endline
    "paired operation 77; single-line edit filters update without replacing the draft";
  [%expect
    {| paired operation 77; single-line edit filters update without replacing the draft |}]
;;

let%expect_test "bounded source data and independent bytes prepare through real Rust FFI" =
  Eio_main.run (fun env ->
    let rows =
      (* Dune's fixture is a symlink outside the test's cwd capability. The
         runner's filesystem capability permits resolving that declared input. *)
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "input-validation-source.tsv")
      |> String.split_lines
    in
    List.iter rows ~f:(fun row ->
      if not (String.is_prefix row ~prefix:"#")
      then (
        let name, bytes, expected =
          match String.split row ~on:'\t' with
          | [ name; hex; expected ] -> name, unhex hex, expected
          | _ -> failwith "invalid validation fixture"
        in
        let pos_ref = ref 0 in
        let wire = W.Source.bin_read_t (Bigstring.of_string bytes) ~pos_ref in
        assert (!pos_ref = String.length bytes);
        let matching =
          match wire.matching with
          | Whole_value -> V.Regex.Matching.Whole_value
          | Substring -> Substring
        in
        let source =
          V.Regex.Source.create ~matching ~case_sensitive:wire.case_sensitive wire.pattern
          |> ok
        in
        assert (
          String.equal (W.Source.encode (V.Expert.source_to_wire source) |> ok) bytes);
        let result = Gpuio_eio.Input_validation.prepare_regex source in
        let actual =
          match result with
          | Ok regex ->
            assert (V.Regex.Source.equal (V.Regex.source regex) source);
            "checked"
          | Error (Invalid_regex message) ->
            assert ((not (String.is_empty message)) && String.length message <= 1024);
            "invalid-regex"
          | Error error -> raise_s [%sexp (name : string), (error : V.Error.t)]
        in
        assert (String.equal actual expected))));
  print_endline
    "independent bytes, checked immutable sources and typed regex errors; no GPUI window";
  [%expect
    {| independent bytes, checked immutable sources and typed regex errors; no GPUI window |}]
;;

let%expect_test
    "source and preparation decoder enforce bounds before allocating diagnostics"
  =
  List.iter
    [ String.make 2049 'a'; "a\000"; "\255" ]
    ~f:(fun pattern -> assert (Result.is_error (V.Regex.Source.create pattern)));
  let source = V.Regex.Source.create (String.make 2048 'a') |> ok in
  assert (String.length (V.Regex.Source.pattern source) = 2048);
  List.iter
    [ ""; "0000"; "02"; "0103"; "0101fdffffff7f"; "010101ff"; "01010100"; "010104626164" ]
    ~f:(fun hex -> assert (Result.is_error (W.Preparation.decode (unhex hex))));
  List.iter
    [ "00", W.Preparation.Checked
    ; "0100", Failed Invalid_source
    ; "0102", Failed Too_complex
    ; "010103626164", Failed (Invalid_regex "bad")
    ]
    ~f:(fun (hex, expected) ->
      let result = W.Preparation.decode (unhex hex) |> ok in
      assert (W.Preparation.equal result expected);
      assert (
        String.equal
          (Bin_prot.Utils.bin_dump W.Preparation.bin_writer_t result
           |> Bigstring.to_string)
          (unhex hex)));
  print_endline
    "UTF-8, source limit, result tags, claimed diagnostic lengths and trailing bytes \
     checked";
  [%expect
    {| UTF-8, source limit, result tags, claimed diagnostic lengths and trailing bytes checked |}]
;;

let%expect_test
    "bounded native compilation rejects expansion and remains usable after failure"
  =
  Eio_main.run (fun _ ->
    let prepare text =
      V.Regex.Source.create text |> ok |> Gpuio_eio.Input_validation.prepare_regex
    in
    assert (
      Result.equal
        V.Regex.equal
        V.Error.equal
        (prepare "a{100000000}")
        (Error Too_complex));
    assert (Result.is_error (prepare "(?=a)"));
    let long_invalid_source = String.concat (List.init 682 ~f:(fun _ -> "界")) ^ "[" in
    (match prepare long_invalid_source with
     | Error (Invalid_regex message) ->
       assert (String.length message > 1000 && String.length message <= 1024);
       assert (Stdlib.String.is_valid_utf_8 message)
     | _ -> failwith "expected a bounded UTF-8 syntax diagnostic");
    assert (Result.is_ok (prepare "(?x)a # trailing comment"));
    Eio.Fiber.all
      [ (fun () -> assert (Result.is_ok (prepare "[0-9]*")))
      ; (fun () -> assert (Result.is_ok (prepare "\\p{L}+")))
      ; (fun () -> assert (Result.is_error (prepare "[")))
      ]);
  print_endline
    "compilation limits and concurrent preparations use typed results without poisoning \
     the preparer";
  [%expect
    {| compilation limits and concurrent preparations use typed results without poisoning the preparer |}]
;;
