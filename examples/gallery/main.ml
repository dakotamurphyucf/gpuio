open Core
open Gpuio

(* Entry point only. Application owns startup; Component wires Bonsai;
   Shell builds the stateless GPUIO layout. *)
let ok = Or_error.ok_exn

let () =
  if Array.exists (Sys.get_argv ()) ~f:(String.equal "--check-catalogs")
  then (
    Application.check_catalogs ();
    Eio_main.run (fun env ->
      Gpuio_eio.Output.write
        (Eio.Stdenv.stdout env)
        "GALLERY_CATALOGS_PASS counter=1 document_profile=1\n"))
  else if Array.exists (Sys.get_argv ()) ~f:(String.equal "--print-info-plist")
  then (
    let package =
      Desktop_package.macos_info_plist
        Desktop_session.identity
        ~executable:"gpuio-studio"
        ~version:"0.1.0"
        ~build:"1"
      |> ok
    in
    Eio_main.run (fun env ->
      Gpuio_eio.Output.write (Eio.Stdenv.stdout env) (Desktop_package.contents package)))
  else Application.run ()
;;
