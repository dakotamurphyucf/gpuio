open Core

let () =
  let platform = not (Array.exists (Sys.get_argv ()) ~f:(String.equal "--drawn")) in
  Gpuio_eio.App.run (fun _env app ->
    Gpuio_eio.App.open_window
      app
      ~title:"GPUIO positioned menus"
      ~width:660.
      ~height:700.
      (Component.create ~platform ~app)
    |> Or_error.ok_exn
    |> fun (_ : Gpuio_eio.App.Window.t) -> ())
;;
