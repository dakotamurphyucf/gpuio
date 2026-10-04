open Core
module File = Gpuio_gallery_files.Theme_file
module P = Gpuio_gallery_model.Theme_profile

let%expect_test "bounded file read and cancellation" =
  Eio_main.run (fun env ->
    let cwd = Eio.Stdenv.cwd env in
    (* Dune may symlink declared read-only inputs out of the test directory. *)
    let sample = Eio.Path.(Eio.Stdenv.fs env / "aurora.sexp") in
    let original = Eio.Path.load sample in
    print_endline (P.name (File.load sample |> Or_error.ok_exn));
    let dir = Eio.Path.(cwd / "theme-import-fixture") in
    Eio.Path.mkdir ~perm:0o700 dir;
    Exn.protect
      ~finally:(fun () -> Eio.Path.rmtree ~missing_ok:true dir)
      ~f:(fun () ->
        let path = Eio.Path.(dir / "profile.sexp") in
        List.iter
          [ P.maximum_bytes - 1
          ; P.maximum_bytes
          ; P.maximum_bytes + 1
          ; P.maximum_bytes * 2
          ]
          ~f:(fun size ->
            Eio.Path.save
              path
              ~create:(`Or_truncate 0o600)
              (original ^ String.make (size - String.length original) ' ');
            printf "%d: %b\n" size (Result.is_ok (File.load path)));
        Eio.Path.save path ~create:(`Or_truncate 0o600) "not-a-theme";
        assert (Result.is_error (File.load path));
        assert (Result.is_error (File.load Eio.Path.(dir / "missing")));
        let returned = ref false in
        (try
           Eio.Cancel.sub (fun cc ->
             Eio.Cancel.cancel cc Exit;
             ignore (File.load path : P.t Or_error.t);
             returned := true)
         with
         | Eio.Cancel.Cancelled _ -> ());
        printf "cancelled read returned a value: %b\n" !returned));
  [%expect
    {|
    Aurora
    16383: true
    16384: true
    16385: false
    32768: false
    cancelled read returned a value: false
  |}]
;;
