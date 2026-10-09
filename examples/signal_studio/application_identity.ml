open Core
module Workspace = Signal_studio_model.Workspace
module Desktop = Gpuio_eio.Desktop

let value =
  Desktop.Identity.create
    ~identifier:"com.gpuio.signal-studio"
    ~name:"GPUIO Signal Studio"
    ~schemes:[ Workspace.scheme ]
    ()
  |> Or_error.ok_exn
;;
