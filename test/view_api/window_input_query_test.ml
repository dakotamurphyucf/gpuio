open Core
open Gpuio
module P = Gpuio_protocol

let ok = Or_error.ok_exn
let window generation = P.Window_id.create ~slot:0L ~generation |> ok
let node generation = P.Node_id.create ~slot:3L ~generation |> ok

let%expect_test "focus observations match exact text-input generations and kind" =
  let snapshot =
    Text_input.Expert.snapshot
      ~window:(window 1L)
      ~node:(node 1L)
      ~revision:(Text_input.Revision.of_int64 0L |> ok)
      ~text:"secret"
      ~selection:(Text_input.Selection.create ~anchor:0 ~head:0 |> ok)
      ~composition:None
      ~focused:true
    |> ok
  in
  List.iter
    [ 1L, 1L, P.Window_wire.Input_kind.Input; 2L, 1L, Input; 1L, 2L, Input; 1L, 1L, Otp ]
    ~f:(fun (w, n, kind) ->
      let observed =
        Window.Input.Expert.of_wire ~window:(window w) { node = node n; kind }
      in
      print_s [%sexp (Window.Input.same_text_input observed snapshot : bool)]);
  [%expect
    {|
    true
    false
    false
    false
    |}]
;;
