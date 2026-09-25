open Core
open Gpuio
module S = Slider
module W = Gpuio_protocol.Slider_wire

let ok = Or_error.ok_exn
let domain = Numeric.Domain.create ~min:(-2.) ~max:8. ~step:0.5 |> ok

let config =
  S.Config.create
    ~domain
    ~label:"Temperature"
    ~lower_label:"Minimum"
    ~upper_label:"Maximum"
    ~axis:Vertical
    ()
  |> ok
;;

let range lower upper = S.Value.range ~lower ~upper |> ok

let snapshot =
  { W.Snapshot.revision = 7L
  ; value = Range { lower = 1.5; upper = 7. }
  ; committed = Range { lower = 2.; upper = 7. }
  ; dragging = Some Lower
  }
;;

let%expect_test "slider contracts validate labels, logarithmic domain and ordered values" =
  List.iter [ Float.nan; Float.infinity ] ~f:(fun v ->
    assert (Result.is_error (S.Value.single v)));
  assert (Result.is_error (S.Value.range ~lower:3. ~upper:2.));
  List.iter
    [ ""; " \t\011"; "bad\000label"; "\255"; String.make 4097 'a' ]
    ~f:(fun label -> assert (Result.is_error (S.Config.create ~domain ~label ())));
  assert (Result.is_error (S.Config.create ~domain ~label:"Log" ~scale:Logarithmic ()));
  assert (Numeric.Domain.equal domain (S.Config.domain config));
  assert (Result.is_error (S.Revision.of_int64 (-1L)));
  let observed = S.Expert.snapshot_of_wire snapshot |> ok in
  assert (S.Value.equal (S.Snapshot.value observed) (range 1.5 7.));
  assert (S.Value.equal (S.Snapshot.committed observed) (range 2. 7.));
  assert (Option.equal S.Thumb.equal (S.Snapshot.dragging observed) (Some Lower));
  print_endline
    "finite ordered values, bounded labels, positive logarithmic minimum, distinct \
     preview/commit";
  [%expect
    {| finite ordered values, bounded labels, positive logarithmic minimum, distinct preview/commit |}]
;;

let%expect_test "malformed snapshots and lifecycle phases cannot enter public types" =
  List.iter
    [ { snapshot with revision = -1L }
    ; { snapshot with revision = 0L }
    ; { snapshot with dragging = None }
    ; { snapshot with dragging = Some Single }
    ; { snapshot with committed = Single 1. }
    ; { snapshot with value = Range { lower = 1.5; upper = 6. } }
    ]
    ~f:(fun s -> assert (Result.is_error (S.Expert.snapshot_of_wire s)));
  List.iter
    [ W.Event.Drag_started snapshot
    ; Committed (Pointer, snapshot)
    ; Cancelled (Escape, snapshot)
    ]
    ~f:(fun e -> assert (Result.is_error (S.Expert.event_of_wire e)));
  ignore (S.Expert.event_of_wire (Preview snapshot) |> ok : S.Event.t);
  print_endline "revision, mode, stationary thumb and lifecycle invariants enforced";
  [%expect {| revision, mode, stationary thumb and lifecycle invariants enforced |}]
;;

let hex s =
  String.to_list s
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "independent slider config, preview and guarded replacement fixtures" =
  let command =
    S.Command.Replace
      { value = range 0. 6.; if_revision = Some (S.Revision.of_int64 7L |> ok) }
  in
  let values =
    [ ( "slider-config.hex"
      , Bin_prot.Utils.bin_dump W.Config.bin_writer_t (S.Expert.config_to_wire config) )
    ; ( "slider-preview.hex"
      , Bin_prot.Utils.bin_dump W.Event.bin_writer_t (Preview snapshot) )
    ; ( "slider-replace.hex"
      , Bin_prot.Utils.bin_dump W.Command.bin_writer_t (S.Expert.command_to_wire command)
      )
    ]
  in
  Eio_main.run (fun env ->
    List.iter values ~f:(fun (name, bytes) ->
      assert (
        String.equal
          (hex (Bigstring.to_string bytes))
          (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip))));
  print_endline "three independently authored OCaml/Rust slider fixtures agree";
  [%expect {| three independently authored OCaml/Rust slider fixtures agree |}]
;;
