open Core
open Gpuio
module W = Gpuio_protocol.Link_wire

let ok = Or_error.ok_exn

let%expect_test "link fixture agrees with independent Rust bytes" =
  let config =
    Link.Config.create ~label:"Guide 世界" ~tab_stop:false ~tab_index:(-2) () |> ok
  in
  let wire = Link.Expert.to_wire config in
  let bytes = Bin_prot.Utils.bin_dump W.bin_writer_t wire in
  let pos_ref = ref 0 in
  let decoded = W.bin_read_t bytes ~pos_ref in
  assert (!pos_ref = Bigstring.length bytes);
  assert (Link.Config.equal config (Link.Expert.of_wire decoded |> ok));
  Bigstring.to_string bytes
  |> String.iter ~f:(fun byte -> printf "%02x" (Char.to_int byte));
  print_endline "";
  [%expect {| 0c477569646520e4b896e7958c0000fffe |}]
;;

let%expect_test "link configuration preserves explicit focus intent and defaults" =
  let defaults = Link.Config.create ~label:"Read the guide" () |> ok in
  assert (String.equal (Link.Config.label defaults) "Read the guide");
  assert (not (Link.Config.is_disabled defaults));
  assert (Link.Config.tab_stop defaults && Link.Config.tab_index defaults = 0);
  List.iter [ false; true ] ~f:(fun disabled ->
    List.iter [ false; true ] ~f:(fun tab_stop ->
      List.iter [ -1000000; -1; 0; 1000000 ] ~f:(fun tab_index ->
        let config =
          Link.Config.create ~label:"世界" ~disabled ~tab_stop ~tab_index () |> ok
        in
        assert (Bool.equal (Link.Config.is_disabled config) disabled);
        assert (Bool.equal (Link.Config.tab_stop config) tab_stop);
        assert (Link.Config.tab_index config = tab_index);
        assert (W.valid (Link.Expert.to_wire config)))));
  print_endline
    "enabled Tab default; signed indices and explicit tab policy survive disabled state";
  [%expect
    {| enabled Tab default; signed indices and explicit tab policy survive disabled state |}]
;;

let%expect_test "labels and wire values are validated at domain entry" =
  List.iter
    [ ""; " \t\n\r\011\012"; "a\000b"; "\255"; String.make 4097 'x' ]
    ~f:(fun label -> assert (Or_error.is_error (Link.Config.create ~label ())));
  ignore (Link.Config.create ~label:(String.make 4096 'x') () |> ok : Link.Config.t);
  let wire = Link.Config.create ~label:"Guide" () |> ok |> Link.Expert.to_wire in
  List.iter [ Int64.min_value; -1000001L; 1000001L; Int64.max_value ] ~f:(fun tab_index ->
    let invalid = { wire with tab_index } in
    assert (not (W.valid invalid));
    assert (Or_error.is_error (Link.Expert.of_wire invalid)));
  List.iter [ Int.min_value; -1000001; 1000001; Int.max_value ] ~f:(fun tab_index ->
    assert (Or_error.is_error (Link.Config.create ~label:"Guide" ~tab_index ())));
  assert (Or_error.is_error (Link.Expert.of_wire { wire with label = "\255" }));
  print_endline
    "invalid UTF-8/NUL/blank/oversize labels and out-of-range decoded indices rejected";
  [%expect
    {| invalid UTF-8/NUL/blank/oversize labels and out-of-range decoded indices rejected |}]
;;
