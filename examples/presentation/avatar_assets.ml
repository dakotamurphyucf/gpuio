open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Asset = Gpuio_eio.Asset
module B = Bonsai.Cont
module E = Bonsai.Effect

type t =
  { image : Gpuio.Asset.Handle.t
  ; invalid : Gpuio.Asset.Handle.t
  }

let load env app assets =
  let scope = App.scope app in
  Scope.start
    scope
    ~f:(fun () ->
      let clock = Eio.Stdenv.clock env in
      Eio.Time.with_timeout_exn clock 20. (fun () ->
        let register format bytes =
          let source = Asset.Source.of_bytes ~format bytes |> Or_error.ok_exn in
          let rec attempt () =
            let promise, resolver = Eio.Promise.create () in
            Scope.Expert.enqueue scope (fun () ->
              E.Expert.handle
                (E.map
                   (Asset.register app ~scope source)
                   ~f:(Eio.Promise.resolve resolver)));
            match Eio.Promise.await promise with
            | Ok asset -> Asset.handle asset
            | Error Not_ready ->
              Eio.Time.sleep clock 0.005;
              attempt ()
            | Error error -> raise_s [%sexp (error : Asset.Error.t)]
          in
          attempt ()
        in
        let image =
          register
            Svg
            {|<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="#667de8"/><path d="M0 48L64 8V64H0Z" fill="#4855a0"/><path d="M20 45L32 18L44 45M24 36H40" fill="none" stroke="white" stroke-width="4" stroke-linecap="round" stroke-linejoin="round"/></svg>|}
        in
        let invalid = register Pnm "Simulated invalid image data" in
        B.Expert.Var.set assets (Some { image; invalid })))
    ~on_result:(fun result -> E.of_thunk (fun () -> Or_error.ok_exn result))
  |> Or_error.ok_exn
  |> fun (_ : Scope.Task.t) -> ()
;;
