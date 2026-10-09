open Core

module Fallback = struct
  type t = string [@@deriving equal, sexp_of]

  let create text =
    if
      String.length text > 128
      || String.is_empty (String.strip text)
      || (not (Stdlib.String.is_valid_utf_8 text))
      || String.exists text ~f:(fun c -> Char.to_int c < 32 || Char.to_int c = 127)
    then
      Or_error.error_string
        "avatar fallback must be nonblank UTF-8, at most 128 bytes, without ASCII \
         controls"
    else Ok text
  ;;

  let text t = t
end

module Config = struct
  type t =
    { asset : Asset.Handle.t option
    ; fit : Image.Fit.t
    ; fallback : Fallback.t
    ; description : Image.Description.t
    }
  [@@deriving equal, sexp_of]

  let create ?asset ?(fit = Image.Fit.Cover) ~fallback ~description () =
    { asset; fit; fallback; description }
  ;;

  let fallback t = t.fallback
  let description t = t.description
end

module Expert = struct
  let image (t : Config.t) =
    Option.map t.asset ~f:(fun asset ->
      Image.Config.create ~asset ~fit:t.fit ~description:t.description ())
  ;;

  let to_wire (t : Config.t) ~owner : Gpuio_protocol.Avatar_wire.Config.t =
    let source =
      Option.map (image t) ~f:(fun image -> (Image.Expert.to_wire image ~owner).source)
    in
    let fit : Gpuio_protocol.Image_wire.Fit.t =
      match t.fit with
      | Fill -> Fill
      | Contain -> Contain
      | Cover -> Cover
      | Scale_down -> Scale_down
      | None -> None
    in
    { source
    ; fit
    ; label = Image.Expert.label t.description
    ; fallback = Fallback.text t.fallback
    }
  ;;
end

module Palette = struct
  module Appearance = struct
    type t =
      | Light
      | Dark
    [@@deriving equal, sexp_of]
  end

  type t =
    { index : int
    ; background : Color.t
    ; foreground : Color.t
    ; border : Color.t
    }
  [@@deriving equal, sexp_of]

  (* Fixed, quantized colors. Reproduce/check with scripts/audit_avatar_palette.py.
     The bytes, rather than runtime color-space floating point, define v1. *)
  let light =
    [| 0xffedf4, 0xa13760, 0xf8ced9
     ; 0xffeee9, 0xa63a2d, 0xfacfc7
     ; 0xfff1e0, 0x9c4900, 0xf4d4ba
     ; 0xfdf5dd, 0x835c00, 0xe7dab6
     ; 0xf2f9e1, 0x5a6e00, 0xd6e0bc
     ; 0xe7fcea, 0x007932, 0xc4e5ca
     ; 0xdffdf6, 0x007c66, 0xb8e6dc
     ; 0xdefcff, 0x007790, 0xb5e4ed
     ; 0xe3f9ff, 0x006aac, 0xbee0f9
     ; 0xecf5ff, 0x445ab5, 0xcedafd
     ; 0xf8f1ff, 0x714ba8, 0xdfd4f8
     ; 0xffeeff, 0x8f3e8a, 0xefcfeb
    |]
  ;;

  let dark =
    [| 0x42232d, 0xffa6c1, 0x572f3c
     ; 0x44241f, 0xffa99a, 0x59302a
     ; 0x402712, 0xf9b379, 0x54351a
     ; 0x382c0c, 0xe0c16d, 0x493b13
     ; 0x2b3113, 0xbbcf7c, 0x39421b
     ; 0x1a3520, 0x8fd89e, 0x24462c
     ; 0x07362f, 0x68dcc7, 0x0c473e
     ; 0x03343c, 0x60d8eb, 0x05454e
     ; 0x133144, 0x7eceff, 0x1c4159
     ; 0x242c47, 0xa8c1ff, 0x313b5d
     ; 0x322843, 0xcfb4ff, 0x423658
     ; 0x3c243a, 0xedaae6, 0x4f314c
    |]
  ;;

  let index_of_bytes bytes =
    let digest =
      String.fold bytes ~init:2166136261L ~f:(fun accumulator byte ->
        Int64.(
          bit_and
            (bit_xor accumulator (of_int (Char.to_int byte)) * 16777619L)
            0xffffffffL))
    in
    Int64.(to_int_exn (rem digest 12L))
  ;;

  let of_bytes bytes ~appearance =
    let index = index_of_bytes bytes in
    let background, foreground, border =
      (match appearance with
       | Appearance.Light -> light
       | Dark -> dark).(index)
    in
    { index
    ; background = Color.rgb_exn background
    ; foreground = Color.rgb_exn foreground
    ; border = Color.rgb_exn border
    }
  ;;

  let for_key key ~appearance = of_bytes (Key.to_string key) ~appearance
  let for_fallback fallback ~appearance = of_bytes (Fallback.text fallback) ~appearance
  let index t = t.index
  let background t = t.background
  let foreground t = t.foreground
  let border t = t.border

  let style t =
    Style.create_exn
      [ Background (Background.solid t.background)
      ; Foreground t.foreground
      ; Border_color t.border
      ]
  ;;
end
