open Core

(* CLI and package metadata. Start reading the application in Application.run. *)
let output env text = Gpuio_eio.Output.write (Eio.Stdenv.stdout env) text

let main () =
  let flag value = Array.exists (Sys.get_argv ()) ~f:(String.equal value) in
  let attachment_directory =
    let args = Sys.get_argv () in
    Option.map
      (Array.findi args ~f:(fun _ value -> String.equal value "--directory"))
      ~f:(fun (index, _) ->
        if index + 1 >= Array.length args
        then failwith "--directory requires an absolute path";
        Gpuio.File_path.of_string args.(index + 1) |> Or_error.ok_exn)
  in
  let motion =
    match flag "--reduced-motion", flag "--full-motion" with
    | true, true -> failwith "Choose only one motion override"
    | true, false -> Gpuio.Animation.Preference.Reduce
    | false, true -> Full
    | false, false -> System
  in
  Application.run
    ~attachment_directory
    ~motion
    ~self_test:(flag "--self-test")
    ~native_test:(flag "--native-test")
    ~workload_metrics:(flag "--workload-metrics")
;;

let () =
  if Array.exists (Sys.get_argv ()) ~f:(String.equal "--print-info-plist")
  then (
    let identity =
      Gpuio.Desktop.Identity.create
        ~identifier:"com.gpuio.agent-chat"
        ~name:"GPUIO Agent Workspace"
        ~schemes:[]
        ()
      |> Or_error.ok_exn
    in
    let package =
      Gpuio.Desktop_package.macos_info_plist
        identity
        ~executable:"gpuio-agent-chat"
        ~version:"0.1.0"
        ~build:"1"
      |> Or_error.ok_exn
    in
    Eio_main.run (fun env -> output env (Gpuio.Desktop_package.contents package)))
  else main ()
;;
