open! Core

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

let of_dark dark =
  let c light night = Gpuio.Color.rgb_exn (if dark then night else light) in
  { canvas = c 0xfafaf9 0x111318
  ; sidebar = c 0xf0f0ee 0x17191f
  ; surface = c 0xffffff 0x1b1e26
  ; raised = c 0xe9e9e7 0x242832
  ; line = c 0xe0e1df 0x2b2f39
  ; text = c 0x262832 0xe6e7ed
  ; muted = c 0x656a76 0x969dad
  ; faint = c 0x767c88 0x7c8496
  ; accent = c 0x6950bd 0xc2b1ff
  ; accent_surface = c 0xe8e2f6 0x302b43
  ; accent_ink = c 0xffffff 0x231a3b
  ; success = c 0x287a60 0x85c6a3
  }
;;
