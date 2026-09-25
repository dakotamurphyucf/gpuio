(** The reference app's semantic palette, independent of native resource state. *)
type t =
  { canvas : Gpuio.Color.t
  ; sidebar : Gpuio.Color.t
  ; surface : Gpuio.Color.t
  ; raised : Gpuio.Color.t
  ; line : Gpuio.Color.t
  ; text : Gpuio.Color.t
  ; muted : Gpuio.Color.t
  ; faint : Gpuio.Color.t
  ; accent : Gpuio.Color.t
  ; accent_surface : Gpuio.Color.t
  ; accent_ink : Gpuio.Color.t
  ; success : Gpuio.Color.t
  }

val of_dark : bool -> t
