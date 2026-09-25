open Core
open Gpuio_protocol

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let unhex text =
  String.init
    (String.length text / 2)
    ~f:(fun i ->
      Char.of_int_exn (Int.of_string ("0x" ^ String.sub text ~pos:(i * 2) ~len:2)))
;;

let%expect_test "native split request and event field order" =
  let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let handler = Handler_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let config : Split_wire.Config.t =
    { label = "x"
    ; axis = Horizontal
    ; initial_first = 160.
    ; minimum_first = 80.
    ; maximum_first = 280.
    ; minimum_second = 80.
    ; keyboard_step = 16.
    ; reset_generation = 0L
    }
  in
  print_endline
    (Wire.Message.encode
       (Apply
          { window
          ; base = 0L
          ; revision = 1L
          ; operations =
              [ Create (node, Split_pane, "", Some handler); Set_split (node, config) ]
          })
     |> Or_error.ok_exn
     |> hex);
  let events =
    Wire.Event.decode (unhex "0125000100010001010000000000000064400000000000006e40")
    |> Or_error.ok_exn
  in
  (match events with
   | [ Split_resized (w, n, h, revision, generation, snapshot) ] ->
     assert (
       Window_id.equal w window && Node_id.equal n node && Handler_id.equal h handler);
     print_s
       [%sexp
         (revision : int64), (generation : int64), (snapshot : Split_wire.Snapshot.t)]
   | _ -> failwith "split event");
  [%expect
    {|
    0300010001020000011d000100012200010178000000000000006440000000000000544000000000008071400000000000005440000000000000304000
    (1 0 ((first 160) (second 240)))
    |}]
;;
