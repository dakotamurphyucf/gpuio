open Core

let save path ~random contents =
  if String.length contents > 65536
  then Or_error.error_string "Settings export exceeds 64 KiB"
  else (
    match Eio.Path.split path with
    | None -> Or_error.error_string "A document needs a filename"
    | Some (parent, name) ->
      (try
         Eio.Path.with_open_dir parent (fun dir ->
           let bytes = Eio.Buf_read.take 16 (Eio.Buf_read.of_flow ~max_size:17 random) in
           let suffix =
             String.concat_map bytes ~f:(fun c -> sprintf "%02x" (Char.to_int c))
           in
           let temporary = Eio.Path.(dir / (".gpuio-save-" ^ suffix)) in
           Eio.Path.with_open_out ~create:(`Exclusive 0o600) temporary (fun flow ->
             Exn.protect
               ~f:(fun () ->
                 Eio.Flow.copy_string contents flow;
                 Eio.File.sync flow;
                 Eio.Path.rename temporary Eio.Path.(dir / name))
               ~finally:(fun () ->
                 Eio.Cancel.protect (fun () ->
                   try Eio.Path.unlink temporary with
                   | Eio.Io (Eio.Fs.E (Not_found _), _) -> ()))));
         Ok ()
       with
       | Eio.Io _ as exn -> Or_error.of_exn exn))
;;
