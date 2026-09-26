open Core
open Gpuio
module W = Gpuio_protocol.Navigation_stack_wire

let ok = Or_error.ok_exn

let%expect_test "native presentation policy keeps history positions under Unmount" =
  let entries =
    List.init 3 ~f:(fun i ->
      let label = Int.to_string i in
      let id = Navigation_stack.Id.of_string label |> ok in
      Navigation_stack.Entry.create ~id ~label () |> ok)
  in
  let stack = Navigation_stack.create entries |> ok in
  let config =
    Navigation_stack.Expert.presentation_config
      stack
      ~hidden:Content_policy.Retain
      ~motion:Navigation_stack.Motion.default
  in
  assert (W.Config.valid_children config ~count:3);
  let unmounted =
    Navigation_stack.Expert.presentation_config
      (Navigation_stack.pop stack)
      ~hidden:Content_policy.Unmount
      ~motion:Navigation_stack.Motion.immediate
  in
  print_s [%sexp (config : W.Config.t), (unmounted : W.Config.t)];
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "navigation-stack-config.hex")
      |> String.strip
    in
    let bytes =
      Bin_prot.Utils.bin_dump [%bin_writer: W.Config.t] config |> Bigstring.to_string
    in
    let hex =
      String.to_list bytes
      |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
      |> String.concat
    in
    assert (String.equal expected hex));
  [%expect
    {|
    (((selected (2)) (retain true) (motion Slide) (duration_ms 200))
     ((selected (1)) (retain false) (motion Immediate) (duration_ms 0)))
    |}]
;;

let%expect_test "bounded motion duration and empty history" =
  List.iter [ -1.; 10_001. ] ~f:(fun ms ->
    assert (Result.is_error (Navigation_stack.Motion.slide (Time_ns.Span.of_ms ms))));
  List.iter [ 0.; 0.1; 10_000. ] ~f:(fun ms ->
    let motion = Navigation_stack.Motion.fade (Time_ns.Span.of_ms ms) |> ok in
    let config =
      Navigation_stack.Expert.presentation_config
        Navigation_stack.empty
        ~hidden:Content_policy.Retain
        ~motion
    in
    assert (W.Config.valid_children config ~count:0);
    assert (not (W.Config.valid_children config ~count:1));
    print_s [%sexp (config : W.Config.t)]);
  [%expect
    {|
    ((selected ()) (retain true) (motion Fade) (duration_ms 0))
    ((selected ()) (retain true) (motion Fade) (duration_ms 1))
    ((selected ()) (retain true) (motion Fade) (duration_ms 10000)) |}]
;;
