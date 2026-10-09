open Core
module File = Gpuio_gallery_files.Settings_file

let with_directory f =
  Eio_main.run (fun env ->
    let dir = Eio.Path.(Eio.Stdenv.cwd env / "settings-export-fixture") in
    Eio.Path.mkdir ~perm:0o700 dir;
    Exn.protect
      ~f:(fun () -> f dir (Eio.Stdenv.secure_random env))
      ~finally:(fun () -> Eio.Path.rmtree ~missing_ok:true dir))
;;

let%expect_test "atomic export, size limit and failed destination preserve earlier data" =
  with_directory (fun dir random ->
    let path = Eio.Path.(dir / "settings.sexp") in
    File.save path ~random "initial" |> Or_error.ok_exn;
    File.save path ~random "updated" |> Or_error.ok_exn;
    assert (String.equal (Eio.Path.load path) "updated");
    assert (Or_error.is_error (File.save path ~random (String.make 65537 'x')));
    assert (String.equal (Eio.Path.load path) "updated");
    let directory = Eio.Path.(dir / "destination") in
    Eio.Path.mkdir directory ~perm:0o700;
    Eio.Path.save Eio.Path.(directory / "kept") ~create:(`Exclusive 0o600) "original";
    assert (Or_error.is_error (File.save directory ~random "replacement"));
    assert (String.equal (Eio.Path.load Eio.Path.(directory / "kept")) "original");
    print_s
      [%sexp (List.sort (Eio.Path.read_dir dir) ~compare:String.compare : string list)];
    let link = Eio.Path.(dir / "link") in
    Eio.Path.symlink ~link_to:"settings.sexp" link;
    File.save link ~random "independent" |> Or_error.ok_exn;
    assert (String.equal (Eio.Path.load link) "independent");
    assert (String.equal (Eio.Path.load path) "updated");
    File.save path ~random (String.make 65536 'x') |> Or_error.ok_exn;
    assert (String.length (Eio.Path.load path) = 65536));
  [%expect {| (destination settings.sexp) |}]
;;
