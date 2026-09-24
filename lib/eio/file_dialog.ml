open Core
module Open = Gpuio.File_dialog.Open
module Save = Gpuio.File_dialog.Save
module Error = Gpuio.File_dialog.Error
module Capabilities = Gpuio.File_dialog.Capabilities

let capabilities = App.Window.Expert.file_dialog_capabilities
let open_ window ~config = App.Window.Expert.file_dialog window (Open config)

let save window ~config =
  Bonsai.Effect.map (App.Window.Expert.file_dialog window (Save config)) ~f:(function
    | Error error -> Error error
    | Ok None -> Ok None
    | Ok (Some [ path ]) -> Ok (Some path)
    | Ok (Some ([] | _ :: _ :: _)) -> Error Error.Native_failure)
;;
