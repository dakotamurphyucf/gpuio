open Core

type op =
  | Clear
  | Set_text of int64 * string
  | Set_children of int64 * int64 list
  | Listen of int64 * bool * int64 option
  | Scale of float
[@@deriving bin_io, equal]

type batch =
  { version : int
  ; revision : int64
  ; ops : op list
  }
[@@deriving bin_io, equal]

let fixture =
  { version = 1
  ; revision = 65536L
  ; ops =
      [ Clear
      ; Set_text (127L, "Hello λ 🦀\000")
      ; Set_children
          ( 128L
          , [ -129L
            ; -128L
            ; -1L
            ; 0L
            ; 255L
            ; 256L
            ; 32767L
            ; 32768L
            ; Int64.min_value
            ; Int64.max_value
            ] )
      ; Listen (2147483648L, true, Some 42L)
      ; Listen (0L, false, None)
      ; Scale 1.25
      ]
  }
;;

let%expect_test "independent OCaml and Rust codec fixture" =
  Eio_main.run (fun env ->
    let path = Eio.Path.(Eio.Stdenv.cwd env / "codec-v1.hex") in
    let hex = Eio.Path.load path |> String.strip in
    let golden =
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    let encoded =
      Bin_prot.Utils.bin_dump bin_writer_batch fixture |> Bigstring.to_string
    in
    Expect_test_helpers_core.require_equal [%here] (module String) golden encoded;
    let pos_ref = ref 0 in
    let decoded = bin_read_batch (Bigstring.of_string golden) ~pos_ref in
    Expect_test_helpers_core.require [%here] (equal_batch fixture decoded);
    Expect_test_helpers_core.require_equal
      [%here]
      (module Int)
      !pos_ref
      (String.length golden);
    print_s [%sexp ("CODEC_PASS" : string), (String.length encoded : int)]);
  [%expect {| (CODEC_PASS 96) |}]
;;
