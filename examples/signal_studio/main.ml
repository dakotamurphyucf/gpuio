open Core

(* CLI/package metadata only. Application starts services and windows;
   Component wires Bonsai state; Ui builds the GPUIO layout. *)
let ok = Or_error.ok_exn

let () =
  match Array.to_list (Sys.get_argv ()) with
  | [ _; "--print-info-plist" ] ->
    let text =
      Gpuio.Desktop_package.macos_info_plist
        Application_identity.value
        ~executable:"gpuio-signal"
        ~version:"0.1.0"
        ~build:"1"
      |> ok
      |> Gpuio.Desktop_package.contents
    in
    Eio_main.run (fun env -> Gpuio_eio.Output.write (Eio.Stdenv.stdout env) text)
  | [ _; "--print-desktop-entry"; executable ] ->
    let executable = Gpuio.File_path.of_string executable |> ok in
    let text =
      Gpuio.Desktop_package.linux_entry Application_identity.value ~executable ()
      |> ok
      |> Gpuio.Desktop_package.contents
    in
    Eio_main.run (fun env -> Gpuio_eio.Output.write (Eio.Stdenv.stdout env) text)
  | _ -> Application.run ()
;;
