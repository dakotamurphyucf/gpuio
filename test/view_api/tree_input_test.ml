open Core
open Gpuio
module W = Gpuio_protocol.Tree_input_wire
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let fixture name =
  Eio_main.run (fun env ->
    let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip in
    String.init
      (String.length hex / 2)
      ~f:(fun i ->
        Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2))))
;;

let%expect_test "paired native input fixtures preserve ordered relative and keyed intent" =
  let requests =
    W.Request.
      [ Navigate (Next, Some (Range { extend = true }))
      ; Select (42L, Toggle)
      ; Focus 42L
      ; Set_expanded (42L, true)
      ; Activate 42L
      ; Select_active Replace
      ; Activate_active
      ]
  in
  let bytes =
    Bin_prot.Utils.bin_dump [%bin_writer: W.Request.t list] requests
    |> Bigstring.to_string
  in
  assert (String.equal bytes (fixture "tree-input-requests.hex"));
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok in
  let encoded =
    Wire.Message.encode
      (Apply
         { window
         ; base = 0L
         ; revision = 1L
         ; operations = [ Set_tree_input (node, true) ]
         })
    |> ok
  in
  assert (String.equal encoded (fixture "tree-input-transaction.hex"));
  let event = Wire.Event.Tree_input (window, node, handler, 1L, List.hd_exn requests) in
  let bytes = fixture "tree-input-event.hex" in
  assert (
    String.equal
      bytes
      (Bin_prot.Utils.bin_dump Wire.Event.bin_writer_t event |> Bigstring.to_string));
  assert (List.equal Wire.Event.equal [ event ] (Wire.Event.decode ("\001" ^ bytes) |> ok));
  for length = 0 to String.length bytes - 1 do
    assert (Result.is_error (Wire.Event.decode ("\001" ^ String.prefix bytes length)))
  done;
  assert (Result.is_error (Wire.Event.decode ("\001" ^ bytes ^ "\000")));
  let bad = Wire.Event.Tree_input (window, node, handler, 1L, Focus 0L) in
  let bad =
    Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] [ bad ]
    |> Bigstring.to_string
  in
  assert (Result.is_error (Wire.Event.decode bad));
  let mapped =
    List.filter_map requests ~f:(fun request ->
      Tree_input.Expert.of_wire request ~find_key:(fun id ->
        if Int64.equal id 42L then Some "stable" else None))
  in
  print_s [%sexp (mapped : string Tree_input.t list)];
  let obsolete =
    List.filter_map requests ~f:(fun request ->
      Tree_input.Expert.of_wire request ~find_key:(fun _ -> None))
  in
  assert (List.length obsolete = 3);
  [%expect
    {|
    ((Navigate Next ((Range (extend true)))) (Select stable Toggle)
     (Focus stable) (Set_expanded stable true) (Activate stable)
     (Select_active Replace) Activate_active)
    |}]
;;

let%expect_test "tree row focus appends a scroll target without changing existing tags" =
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let key = Key.of_string_exn "destination" in
  let request = Virtual_list.Scroll_request.focus_tree_row ~serial:7L key |> ok in
  let request =
    Virtual_list.Expert.scroll_to_wire request ~find_id:(fun found ->
      if Key.equal key found then Some 42L else None)
    |> ok
  in
  let bytes =
    Wire.Message.encode
      (Apply
         { window
         ; base = 0L
         ; revision = 1L
         ; operations = [ Scroll_list (node, request) ]
         })
    |> ok
  in
  assert (String.equal bytes (fixture "tree-focus-transaction.hex"));
  assert (Result.is_error (Virtual_list.Scroll_request.focus_tree_row ~serial:0L key));
  let request = Virtual_list.Scroll_request.focus_tree_row ~serial:1L key |> ok in
  assert (
    Result.is_error (Virtual_list.Expert.scroll_to_wire request ~find_id:(fun _ -> None)));
  [%expect {| |}]
;;

let%expect_test "exact selection fixture validates desired membership and target" =
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok in
  let event id =
    Wire.Event.Tree_input (window, node, handler, 1L, Set_selected (id, false))
  in
  let bytes = fixture "tree-selection-event.hex" in
  assert (
    String.equal
      bytes
      (Bin_prot.Utils.bin_dump Wire.Event.bin_writer_t (event 42L) |> Bigstring.to_string));
  assert (
    List.equal Wire.Event.equal [ event 42L ] (Wire.Event.decode ("\001" ^ bytes) |> ok));
  for length = 0 to String.length bytes - 1 do
    assert (Or_error.is_error (Wire.Event.decode ("\001" ^ String.prefix bytes length)))
  done;
  assert (Or_error.is_error (Wire.Event.decode ("\001" ^ bytes ^ "\000")));
  List.iter [ 0L; -1L ] ~f:(fun id ->
    let bytes =
      Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] [ event id ]
      |> Bigstring.to_string
    in
    assert (Or_error.is_error (Wire.Event.decode bytes)));
  assert (
    Or_error.is_error (Wire.Event.decode ("\001" ^ String.drop_suffix bytes 1 ^ "\002")));
  assert (
    Option.is_none
      (Tree_input.Expert.of_wire (Set_selected (42L, true)) ~find_key:(fun _ -> None)));
  print_s
    [%sexp
      (Tree_input.Expert.of_wire
         (Set_selected (42L, false))
         ~find_key:(fun _ -> Some "stable")
       : string Tree_input.t option)];
  [%expect {| ((Set_selected stable false)) |}]
;;

let%expect_test "bounded Unicode typeahead appends request tag seven" =
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok in
  let event text =
    Wire.Event.Tree_input
      (window, node, handler, 1L, Typeahead { text; reset = true; cycle = true })
  in
  let bytes = fixture "tree-typeahead-event.hex" in
  assert (
    String.equal
      bytes
      (Bin_prot.Utils.bin_dump Wire.Event.bin_writer_t (event "é") |> Bigstring.to_string));
  assert (
    List.equal Wire.Event.equal [ event "é" ] (Wire.Event.decode ("\001" ^ bytes) |> ok));
  List.iter
    [ ""; "\000"; "\xff"; "\xc2\x85"; String.make 257 'a' ]
    ~f:(fun text ->
      let bytes =
        Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] [ event text ]
        |> Bigstring.to_string
      in
      assert (Or_error.is_error (Wire.Event.decode bytes)));
  for length = 0 to String.length bytes - 1 do
    assert (Or_error.is_error (Wire.Event.decode ("\001" ^ String.prefix bytes length)))
  done;
  assert (Or_error.is_error (Wire.Event.decode ("\001" ^ bytes ^ "\000")));
  let request = W.Request.Typeahead { text = "é"; reset = true; cycle = true } in
  assert (Option.is_some (Tree_input.Expert.of_wire request ~find_key:(fun _ -> None)));
  [%expect {| |}]
;;

let%expect_test "move fixtures resolve and validate both stable endpoints" =
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok in
  let request source destination =
    W.Request.Move { source; destination; placement = Inside }
  in
  let event source destination =
    Wire.Event.Tree_input (window, node, handler, 1L, request source destination)
  in
  let bytes = fixture "tree-move-event.hex" in
  assert (
    String.equal
      bytes
      (Bin_prot.Utils.bin_dump Wire.Event.bin_writer_t (event 42L 43L)
       |> Bigstring.to_string));
  assert (
    List.equal
      Wire.Event.equal
      [ event 42L 43L ]
      (Wire.Event.decode ("\001" ^ bytes) |> ok));
  let message =
    Wire.Message.Apply
      { window; base = 0L; revision = 1L; operations = [ Set_tree_moves (node, true) ] }
  in
  assert (
    String.equal (Wire.Message.encode message |> ok) (fixture "tree-move-transaction.hex"));
  List.iter
    [ 0L, 43L; 42L, -1L; 42L, 42L ]
    ~f:(fun (source, destination) ->
      let encoded =
        Bin_prot.Utils.bin_dump
          [%bin_writer: Wire.Event.t list]
          [ event source destination ]
        |> Bigstring.to_string
      in
      assert (Or_error.is_error (Wire.Event.decode encoded)));
  for length = 0 to String.length bytes - 1 do
    assert (Or_error.is_error (Wire.Event.decode ("\001" ^ String.prefix bytes length)))
  done;
  assert (Or_error.is_error (Wire.Event.decode ("\001" ^ bytes ^ "\000")));
  assert (
    Or_error.is_error (Wire.Event.decode ("\001" ^ String.drop_suffix bytes 1 ^ "\003")));
  List.iter [ 42L; 43L ] ~f:(fun missing ->
    assert (
      Option.is_none
        (Tree_input.Expert.of_wire (request 42L 43L) ~find_key:(fun id ->
           if Int64.equal missing id then None else Some (Int64.to_string id)))));
  print_s
    [%sexp
      (Tree_input.Expert.of_wire (request 42L 43L) ~find_key:(fun id ->
         Some (Int64.to_string id))
       : string Tree_input.t option)];
  [%expect {| ((Move (source 42) (destination 43) (placement Inside))) |}]
;;
