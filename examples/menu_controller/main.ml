open Core

let () =
  let platform = not (Array.exists (Sys.get_argv ()) ~f:(String.equal "--drawn")) in
  let two_windows = Array.exists (Sys.get_argv ()) ~f:(String.equal "--two-windows") in
  Gpuio_eio.App.run (fun _env app ->
    if two_windows
    then
      List.iter [ "A"; "B" ] ~f:(fun name ->
        Gpuio_eio.App.open_window
          app
          ~title:("GPUIO menu window " ^ name)
          ~width:540.
          ~height:640.
          (Multiwindow.create ~name ~platform)
        |> Or_error.ok_exn
        |> fun (_ : Gpuio_eio.App.Window.t) -> ())
    else
      Gpuio_eio.App.open_window
        app
        ~title:"GPUIO positioned menus"
        ~width:660.
        ~height:700.
        (Component.create ~platform ~app)
      |> Or_error.ok_exn
      |> fun (_ : Gpuio_eio.App.Window.t) -> ())
;;
