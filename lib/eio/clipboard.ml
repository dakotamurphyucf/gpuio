open Core
module Text = Gpuio.Clipboard.Text
module Wire = Gpuio_protocol.Desktop_wire
module Error = Gpuio.Clipboard.Error

let error = function
  | Wire.Error.Invalid_request -> Error.Invalid_request
  | Not_ready -> Error.Not_ready
  | Unsupported -> Error.Unsupported
  | Unavailable -> Error.Unavailable
  | Denied -> Error.Denied
  | Busy -> Error.Busy
  | Closed -> Error.Closed
  | Already_configured | Native_failure -> Error.Native_failure
;;

let write_text app text =
  Bonsai.Effect.map
    (App.Expert.desktop app (Write_clipboard_text (Text.to_string text)))
    ~f:(function
      | Wire.Response.Requested -> Ok ()
      | Failed reason -> Error (error reason)
      | Configured | Capabilities _ | Links _ | Registered | Scrollbar_preference _ ->
        Error Error.Native_failure)
;;

module Copy = struct
  include Clipboard_component

  let create app = Clipboard_component.create (write_text app)
end
