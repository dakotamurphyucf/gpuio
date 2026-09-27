open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Asset = Gpuio_eio.Asset
module B = Bonsai.Cont
module E = Bonsai.Effect

let load env app icon =
  let scope = App.scope app in
  Scope.start
    scope
    ~f:(fun () ->
      let clock = Eio.Stdenv.clock env in
      Eio.Time.with_timeout_exn clock 20. (fun () ->
        let source =
          Asset.Source.of_bytes
            ~format:Svg
            {|<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M3 6h7l2 3h9v11H3z" fill="none" stroke="white" stroke-width="2" stroke-linejoin="round"/></svg>|}
          |> Or_error.ok_exn
        in
        let rec attempt () =
          let promise, resolver = Eio.Promise.create () in
          Scope.Expert.enqueue scope (fun () ->
            E.Expert.handle
              (E.map (Asset.register app ~scope source) ~f:(Eio.Promise.resolve resolver)));
          match Eio.Promise.await promise with
          | Ok asset ->
            Gpuio.Icon.Decoration.create ~asset:(Asset.handle asset) () |> Or_error.ok_exn
          | Error Not_ready ->
            Eio.Time.sleep clock 0.005;
            attempt ()
          | Error error -> raise_s [%sexp (error : Asset.Error.t)]
        in
        attempt ()))
    ~on_result:(fun result ->
      E.of_thunk (fun () -> B.Expert.Var.set icon (Some (Or_error.ok_exn result))))
  |> Or_error.ok_exn
  |> fun (_ : Scope.Task.t) -> ()
;;
