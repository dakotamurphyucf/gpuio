open Core
module W = Gpuio_protocol.Wire
module G = Gpuio_protocol.Editor_geometry_wire

let ok = Or_error.ok_exn
let geometry : G.t = { revision = 3L; x = -12.5; y = 40.; width = 0.; height = 20. }

let%expect_test "range bounds preserve revision and reject invalid native geometry" =
  let t = Gpuio.Editor_geometry.Expert.of_wire geometry |> ok in
  print_s
    [%sexp
      (( Gpuio.Text_input.Revision.to_int64 (Gpuio.Editor_geometry.revision t)
       , Gpuio.Editor_geometry.x t
       , Gpuio.Editor_geometry.y t
       , Gpuio.Editor_geometry.width t
       , Gpuio.Editor_geometry.height t )
       : int64 * float * float * float * float)];
  List.iter
    [ { geometry with revision = -1L }
    ; { geometry with x = Float.nan }
    ; { geometry with y = Float.infinity }
    ; { geometry with x = -1e9 -. 1. }
    ; { geometry with y = 1e9 +. 1. }
    ; { geometry with width = -1. }
    ; { geometry with width = 1e9 +. 1. }
    ; { geometry with height = 0. }
    ]
    ~f:(fun t -> assert (Result.is_error (Gpuio.Editor_geometry.Expert.of_wire t)));
  [%expect {| (3 -12.5 40 0 20) |}]
;;

let%expect_test "paired range envelopes reject malformed requests and observations" =
  Eio_main.run (fun env ->
    let load name =
      let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip in
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
    let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:2L |> ok in
    let request correlation revision anchor head =
      W.Message.Editor_command
        (correlation, window, node, Read_range_bounds (revision, { anchor; head }))
    in
    assert (
      String.equal
        (W.Message.encode (request 7L 3L 5L 1L) |> ok)
        (load "editor-range-read.hex"));
    List.iter
      [ request 0L 3L 5L 1L
      ; request 7L (-1L) 5L 1L
      ; request 7L 3L (-1L) 1L
      ; request 7L 3L 5L 262_145L
      ]
      ~f:(fun r -> assert (Result.is_error (W.Message.encode r)));
    let event id value = W.Event.Editor_result (id, window, node, Range_bounds value) in
    let events = [ event 7L None; event 8L (Some geometry) ] in
    let encode events =
      Bin_prot.Utils.bin_dump [%bin_writer: W.Event.t list] events |> Bigstring.to_string
    in
    let bytes = load "editor-range-events.hex" in
    assert (String.equal (encode events) bytes);
    assert (List.equal W.Event.equal events (W.Event.decode bytes |> ok));
    for n = 0 to String.length bytes - 1 do
      assert (Result.is_error (W.Event.decode (String.prefix bytes n)))
    done;
    assert (Result.is_error (W.Event.decode (bytes ^ "\000")));
    List.iter
      [ { geometry with revision = -1L }
      ; { geometry with width = Float.nan }
      ; { geometry with height = 0. }
      ; { geometry with x = 1e9 +. 1. }
      ]
      ~f:(fun value ->
        assert (Result.is_error (W.Event.decode (encode [ event 7L (Some value) ]))));
    assert (Result.is_error (W.Event.decode "\001\012\007\000\001\001\002\007\002")));
  [%expect {| |}]
;;
