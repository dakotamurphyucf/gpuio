open Core
module Wire = Gpuio_protocol.Window_wire
module Chrome = Wire.Chrome
module Backend = Wire.Backend
module Capabilities = Wire.Capabilities
module Snapshot = Wire.Snapshot
module Command = Wire.Command
module Error = Wire.Error

module Config = struct
  type t = Wire.Config.t

  let create
        ?(focus = true)
        ?(chrome = Chrome.Standard)
        ?(resizable = true)
        ~title
        ~width
        ~height
        ()
    =
    if Wire.valid_title title && Wire.valid_size width height
    then Ok { Wire.Config.title; width; height; focus; chrome; resizable }
    else Or_error.error_string "invalid window title or logical size"
  ;;

  module Expert = struct
    let to_wire t = t
  end
end

module Close_reason = struct
  type t =
    | Window_close
    | Application_quit
  [@@deriving equal, sexp_of]
end

module Close_decision = struct
  type t =
    | Allow
    | Keep_open
  [@@deriving equal, sexp_of]
end
