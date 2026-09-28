open Core
module W = Signal_studio_model.Workspace
module File = Signal_studio_files.Document_file

let with_directory f =
  Eio_main.run (fun env ->
    let dir = Eio.Path.(Eio.Stdenv.cwd env / "signal-document-fixture") in
    Eio.Path.mkdir ~perm:0o700 dir;
    Exn.protect
      ~f:(fun () -> f dir (Eio.Stdenv.secure_random env))
      ~finally:(fun () -> Eio.Path.rmtree ~missing_ok:true dir))
;;

let%expect_test "atomic snapshot replacement and bounded validation" =
  with_directory (fun dir random ->
    let path = Eio.Path.(dir / "workspace.signal") in
    let initial = W.create () in
    File.save path ~random initial |> Or_error.ok_exn;
    print_s [%sexp (String.equal (W.encode initial) (Eio.Path.load path) : bool)];
    let next = W.set_run initial 42 |> Or_error.ok_exn in
    File.save path ~random next |> Or_error.ok_exn;
    print_s [%sexp (W.run (File.load path |> Or_error.ok_exn) : int)];
    print_s [%sexp (Eio.Path.read_dir dir : string list)];
    Eio.Path.save path ~create:(`Or_truncate 0o600) (String.make ((16 * 1024) + 1) 'x');
    print_s [%sexp (Or_error.is_error (File.load path) : bool)];
    Eio.Path.save path ~create:(`Or_truncate 0o600) "(invalid document)";
    print_s [%sexp (Or_error.is_error (File.load path) : bool)];
    print_s [%sexp (Or_error.is_error (File.load Eio.Path.(dir / "missing")) : bool)]);
  [%expect
    {| 
    true
    42
    (workspace.signal)
    true
    true
    true
  |}]
;;

let%expect_test "failed replacement preserves destination and cleans temporary sibling" =
  with_directory (fun dir random ->
    let destination = Eio.Path.(dir / "existing-directory") in
    Eio.Path.mkdir destination ~perm:0o700;
    Eio.Path.save Eio.Path.(destination / "kept") ~create:(`Exclusive 0o600) "original";
    print_s
      [%sexp (Or_error.is_error (File.save destination ~random (W.create ())) : bool)];
    print_s [%sexp (Eio.Path.read_dir dir : string list)];
    print_s [%sexp (Eio.Path.load Eio.Path.(destination / "kept") : string)]);
  [%expect
    {|
    true
    (existing-directory)
    original
  |}]
;;

let%expect_test "exact size boundary and destination symlink replacement" =
  with_directory (fun dir random ->
    let target = Eio.Path.(dir / "target.signal") in
    let link = Eio.Path.(dir / "link.signal") in
    let initial = W.create () in
    let bytes = W.encode initial in
    Eio.Path.save
      target
      ~create:(`Exclusive 0o600)
      (bytes ^ String.make ((16 * 1024) - String.length bytes) ' ');
    print_s [%sexp (W.run (File.load target |> Or_error.ok_exn) : int)];
    Eio.Path.symlink ~link_to:"target.signal" link;
    File.save link ~random (W.set_run initial 51 |> Or_error.ok_exn) |> Or_error.ok_exn;
    print_s [%sexp (W.run (File.load link |> Or_error.ok_exn) : int)];
    print_s [%sexp (W.run (File.load target |> Or_error.ok_exn) : int)]);
  [%expect
    {| 
    0
    51
    0
  |}]
;;
