open Core
module F = Gpuio.Input_format

let ok = Or_error.ok_exn
let pattern source = F.Pattern.create source |> ok |> F.pattern

let number ?separator ?fraction_digits () =
  F.Number.create ?separator ?fraction_digits () |> ok |> F.number
;;

let print result = print_s [%sexp (result : (string, F.Error.t) Result.t)]

let%expect_test "raw slots never consume formatting literals or discard invalid input" =
  List.iter
    [ "(99)-AA", "12ab"
    ; "- *", "-"
    ; "界-*", "界"
    ; "**", "é"
    ; "*", "界"
    ; "(99)", "12"
    ; "(99)", ""
    ; "9A", "x"
    ; "99", "１２"
    ; "*", "界x"
    ]
    ~f:(fun (source, raw) -> print (F.format_raw (pattern source) raw));
  [%expect
    {|
    (Ok "(12)-ab")
    (Ok "- -")
    (Ok "\231\149\140-\231\149\140")
    (Ok "e\204\129")
    (Ok "\231\149\140")
    (Ok "(12")
    (Ok "")
    (Error Does_not_fit)
    (Error Does_not_fit)
    (Error Does_not_fit)
  |}]
;;

let%expect_test "formatted prefixes preserve literal positions and extract slots exactly" =
  let format = pattern "(99)-AA" in
  List.iter
    [ ""; "("; "(1"; "(12)"; "(12)-a"; "(12)-ab"; "12ab"; "(ab)-12" ]
    ~f:(fun formatted -> print (F.raw_of_formatted format formatted));
  print (F.raw_of_formatted (pattern "-*") "--");
  [%expect
    {|
    (Ok "")
    (Ok "")
    (Ok 1)
    (Ok 12)
    (Ok 12a)
    (Ok 12ab)
    (Error Does_not_fit)
    (Error Does_not_fit)
    (Ok -)
  |}]
;;

let%expect_test "decimal grouping preserves precision and permits incomplete drafts" =
  let format = number ~separator:" " ~fraction_digits:4 () in
  List.iter
    [ ""; "-"; "-."; ".5"; "0012345.2300"; "＋１２３４。５０"; "1.23456"; "1e3"; "1,234" ]
    ~f:(fun raw -> print (F.format_raw format raw));
  print (F.raw_of_formatted format "0 012 345.2300");
  print (F.raw_of_formatted format "1234.5");
  print (F.raw_of_formatted format "1 234.5 0");
  print (F.format_raw (number ~fraction_digits:0 ()) "1.");
  print (F.format_raw (number ~fraction_digits:2 ()) "1.234");
  [%expect
    {|
    (Ok "")
    (Ok -)
    (Ok -.)
    (Ok .5)
    (Ok "0\226\128\175012\226\128\175345.2300")
    (Ok "+1\226\128\175234.50")
    (Error Does_not_fit)
    (Error Does_not_fit)
    (Error Does_not_fit)
    (Ok 0012345.2300)
    (Error Does_not_fit)
    (Error Does_not_fit)
    (Error Does_not_fit)
    (Error Does_not_fit)
  |}]
;;

let%expect_test "format construction and expansion have explicit bounds" =
  List.iter
    [ ""; "\255"; "9\000"; "9\n"; String.make 257 '*' ]
    ~f:(fun source -> assert (Result.is_error (F.Pattern.create source)));
  ignore
    (F.Pattern.create (String.concat (List.init 256 ~f:(fun _ -> "😀"))) |> ok
     : F.Pattern.t);
  List.iter
    [ ""; "ab"; "\000"; "\n"; "."; "+"; "-"; "1"; "١"; "１"; "，"; "。" ]
    ~f:(fun separator -> assert (Result.is_error (F.Number.create ~separator ())));
  List.iter [ -1; 262145 ] ~f:(fun fraction_digits ->
    assert (Result.is_error (F.Number.create ~fraction_digits ())));
  let format = number ~separator:"界" () in
  print (F.format_raw format (String.make 262144 '1'));
  print (F.format_raw format (String.make 262145 '1'));
  print (F.format_raw (pattern "*") "\255");
  print (F.raw_of_formatted (pattern "*") "\000");
  [%expect
    {|
    (Error Limit_exceeded)
    (Error Limit_exceeded)
    (Error Invalid_text)
    (Error Invalid_text)
  |}]
;;

let%expect_test "raw conversion and extraction round-trip literal collisions and Unicode" =
  List.iter [ "*"; "-*"; "界-*"; "**"; "*-*"; "(*-*)" ] ~f:(fun source ->
    let format = pattern source in
    List.iter [ ""; "-"; "界"; "界-"; "--"; "a界"; "é" ] ~f:(fun raw ->
      match F.format_raw format raw with
      | Error _ -> ()
      | Ok formatted ->
        assert (F.accepts_formatted format formatted);
        assert (
          Result.equal
            String.equal
            F.Error.equal
            (F.raw_of_formatted format formatted)
            (Ok raw))));
  print_endline "raw slot values survive formatting, including literal collisions";
  [%expect {| raw slot values survive formatting, including literal collisions |}]
;;

let%expect_test "grouping round trips long decimals exactly and checks output expansion" =
  List.iter [ None; Some ","; Some "界"; Some " " ] ~f:(fun separator ->
    let format = number ?separator () in
    List.iter
      [ ""; "+"; "-."; ".500"; "000123456789012345678901234567890.001000" ]
      ~f:(fun raw ->
        let formatted = F.format_raw format raw |> Result.ok |> Option.value_exn in
        assert (
          Result.equal
            String.equal
            F.Error.equal
            (F.raw_of_formatted format formatted)
            (Ok raw))));
  let format = number ~separator:"," () in
  let raw = "+" ^ String.make 196608 '1' in
  let formatted = F.format_raw format raw |> Result.ok |> Option.value_exn in
  assert (String.length formatted = 262144);
  assert (
    Result.equal String.equal F.Error.equal (F.raw_of_formatted format formatted) (Ok raw));
  print (F.format_raw format (String.make 196609 '1'));
  [%expect {| (Error Limit_exceeded) |}]
;;

let%expect_test "independent format fixtures pair wire data and conversion semantics" =
  let unhex text =
    if String.equal text "-"
    then ""
    else
      String.init
        (String.length text / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub text ~pos:(i * 2) ~len:2)))
  in
  Eio_main.run (fun env ->
    let lines =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "input-format-values.tsv")
      |> String.split_lines
    in
    List.iter lines ~f:(fun line ->
      if not (String.is_prefix line ~prefix:"#")
      then (
        match String.split line ~on:'\t' with
        | [ name; config; operation; input; result; output ] ->
          let bytes = unhex config in
          let pos_ref = ref 0 in
          let wire =
            Gpuio_protocol.Input_format_wire.bin_read_t
              (Bigstring.of_string bytes)
              ~pos_ref
          in
          assert (!pos_ref = String.length bytes);
          let format = F.Expert.of_wire wire |> ok in
          let actual_bytes =
            Bin_prot.Utils.bin_dump
              Gpuio_protocol.Input_format_wire.bin_writer_t
              (F.Expert.to_wire format)
            |> Bigstring.to_string
          in
          assert (String.equal bytes actual_bytes);
          let actual =
            match operation with
            | "raw" -> F.format_raw format (unhex input)
            | "formatted" -> F.raw_of_formatted format (unhex input)
            | _ -> failwith "unknown fixture operation"
          in
          let expected =
            match result with
            | "ok" -> Ok (unhex output)
            | "fit" -> Error F.Error.Does_not_fit
            | "text" -> Error F.Error.Invalid_text
            | _ -> failwith "unknown fixture result"
          in
          if not (Result.equal String.equal F.Error.equal actual expected)
          then raise_s [%sexp (name : string), (actual : (string, F.Error.t) Result.t)]
        | _ -> failwith "malformed format fixture")));
  assert (String.equal Uucp.unicode_version "17.0.0");
  List.iter
    Gpuio_protocol.Input_format_wire.
      [ Pattern ""
      ; Pattern "9\000"
      ; Pattern (String.make 257 '*')
      ; Number { separator = Some "١"; fraction_digits = None }
      ; Number { separator = Some "，"; fraction_digits = None }
      ; Number { separator = None; fraction_digits = Some Int64.max_value }
      ; Number { separator = None; fraction_digits = Some (-1L) }
      ]
    ~f:(fun wire -> assert (Result.is_error (F.Expert.of_wire wire)));
  print_endline "paired format bytes, values and Unicode 17 separator semantics";
  [%expect {| paired format bytes, values and Unicode 17 separator semantics |}]
;;

let%expect_test "format operation uses independently specified paired bytes" =
  let module W = Gpuio_protocol.Wire in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:2L |> ok in
  let formats =
    [ None
    ; Some (pattern "*–99")
    ; Some (number ~separator:"," ~fraction_digits:2 ())
    ; Some (number ())
    ]
  in
  let message =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          List.map formats ~f:(fun format ->
            W.Op.Set_editor_format (node, Option.map format ~f:F.Expert.to_wire))
      }
  in
  let hex =
    W.Message.encode message
    |> ok
    |> String.to_list
    |> List.map ~f:(fun char -> sprintf "%02x" (Char.to_int char))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "input-format-operation.hex")
      |> String.strip
    in
    assert (String.equal hex expected));
  print_endline
    "format operation 76: clear, Unicode pattern, grouped and ungrouped decimal";
  [%expect
    {| format operation 76: clear, Unicode pattern, grouped and ungrouped decimal |}]
;;

let%expect_test "format config preserves legacy editor bytes and reconciles independently"
  =
  let open Gpuio in
  let module W = Gpuio_protocol.Wire in
  let format = pattern "99-99" in
  assert (
    Result.is_error (Text_input.Config.create ~mode:Multiline ~label:"Draft" ~format ()));
  let plain = Text_input.Config.create ~mode:Single_line ~label:"Draft" () |> ok in
  let formatted =
    Text_input.Config.create ~mode:Single_line ~label:"Draft" ~format () |> ok
  in
  assert (
    W.Editor.Config.equal
      (Text_input.Expert.config_to_wire plain)
      (Text_input.Expert.config_to_wire formatted));
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let render format =
    let config =
      Text_input.Config.create ~mode:Single_line ~label:"Draft" ?format () |> ok
    in
    let view =
      View.text_input
        ~controller:(Key.of_string_exn "format-draft")
        ~config
        ~initial_text:"incompatible existing draft"
        ~on_event:(fun _ -> ())
        ()
      |> ok
    in
    let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
    let message = Reconciler.message update in
    Reconciler.accept reconciler update |> ok;
    match message with
    | Some (W.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  let node =
    List.find_map_exn (render None) ~f:(function
      | W.Op.Create (id, Input, _, _) -> Some id
      | _ -> None)
  in
  List.iter
    [ Some format; Some (number ~fraction_digits:2 ()); None ]
    ~f:(fun format ->
      assert (
        List.equal
          W.Op.equal
          (render format)
          [ Set_editor_format (node, Option.map format ~f:F.Expert.to_wire) ]));
  assert (List.is_empty (render None));
  print_endline
    "single-line only; one retained editor; format-only updates; no seed replacement";
  [%expect
    {| single-line only; one retained editor; format-only updates; no seed replacement |}]
;;
