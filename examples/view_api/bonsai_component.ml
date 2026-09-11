open Core
module Bonsai = Bonsai.Cont

let component graph =
  let value, set_value = Bonsai.state 0 graph in
  let open Bonsai.Let_syntax in
  let%arr value = value
  and set_value = set_value in
  Components.counter ~value ~on_increment:(fun () -> set_value (value + 1))
;;

(* The convenience adapter also accepts effects directly. *)
let reset on_reset = Gpuio_bonsai.View.button ~on_click:on_reset "Reset"
