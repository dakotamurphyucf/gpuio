module Bonsai = Bonsai.Cont

let component state actions _window _graph =
  let open Bonsai.Let_syntax in
  let%arr snapshot = Bonsai.Expert.Var.value state in
  Ui.view snapshot actions
;;
