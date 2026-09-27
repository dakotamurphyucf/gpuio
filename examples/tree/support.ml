open Core
module E = Bonsai.Effect
module Scope = Gpuio_eio.Scope

let ok = Or_error.ok_exn

let start scope f =
  ignore
    (Scope.start scope ~f ~on_result:(fun result -> E.of_thunk (fun () -> ok result))
     |> ok
     : Scope.Task.t)
;;

let perform scope ui_effect =
  let promise, resolver = Eio.Promise.create () in
  ignore
    (Scope.start
       scope
       ~f:(fun () -> ())
       ~on_result:(fun result ->
         ok result;
         E.map ui_effect ~f:(Eio.Promise.resolve resolver))
     |> ok
     : Scope.Task.t);
  Eio.Promise.await promise
;;
