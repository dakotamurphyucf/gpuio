module Effect = Bonsai.Effect

let perform scope ui_effect =
  let promise, resolver = Eio.Promise.create () in
  Gpuio_eio.Scope.Expert.enqueue scope (fun () ->
    Effect.Expert.handle (Effect.map ui_effect ~f:(Eio.Promise.resolve resolver)));
  Eio.Promise.await promise
;;
