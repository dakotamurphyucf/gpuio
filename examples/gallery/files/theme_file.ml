open Core
module Profile = Gpuio_gallery_model.Theme_profile

let load path =
  try
    Eio.Path.with_open_in path (fun flow ->
      match
        Eio.Buf_read.parse
          ~max_size:(Profile.maximum_bytes + 1)
          Eio.Buf_read.take_all
          flow
      with
      | Error (`Msg message) -> Or_error.error_string message
      | Ok contents -> Profile.decode contents)
  with
  | Eio.Io _ as exn -> Or_error.of_exn exn
;;
