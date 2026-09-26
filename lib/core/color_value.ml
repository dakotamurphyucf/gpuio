open Core

let zero value = if Float.equal value 0. then 0. else value
let unit_channel value = Float.is_finite value && Float.(value >= 0. && value <= 1.)

module Hsla = struct
  type t =
    { hue_degrees : float
    ; saturation : float
    ; lightness : float
    ; alpha : float
    }
  [@@deriving equal, sexp_of]

  let create ~hue_degrees ~saturation ~lightness ~alpha =
    if
      Float.is_finite hue_degrees
      && Float.(hue_degrees >= 0. && hue_degrees <= 360.)
      && List.for_all [ saturation; lightness; alpha ] ~f:unit_channel
    then
      Ok
        { hue_degrees = (if Float.equal hue_degrees 360. then 0. else zero hue_degrees)
        ; saturation = zero saturation
        ; lightness = zero lightness
        ; alpha = zero alpha
        }
    else Or_error.error_string "HSLA requires finite hue in 0..360 and channels in 0..1"
  ;;

  let hue_degrees t = t.hue_degrees
  let saturation t = t.saturation
  let lightness t = t.lightness
  let alpha t = t.alpha
end

module Rgba = struct
  type t =
    { red : int
    ; green : int
    ; blue : int
    ; alpha : int
    }
  [@@deriving equal, compare, sexp_of]

  let create ~red ~green ~blue ~alpha =
    if List.for_all [ red; green; blue; alpha ] ~f:(fun x -> x >= 0 && x <= 255)
    then Ok { red; green; blue; alpha }
    else Or_error.error_string "RGBA channels must be in 0..255"
  ;;

  let red t = t.red
  let green t = t.green
  let blue t = t.blue
  let alpha t = t.alpha

  let byte value =
    Float.to_int (Float.round_down ((Float.max 0. (Float.min 1. value) *. 255.) +. 0.5))
  ;;

  let of_hsla t =
    let hue = Hsla.hue_degrees t /. 60. in
    let lightness = Hsla.lightness t in
    let chroma = (1. -. Float.abs ((2. *. lightness) -. 1.)) *. Hsla.saturation t in
    let x = chroma *. (1. -. Float.abs (Float.mod_float hue 2. -. 1.)) in
    let red, green, blue =
      match Float.to_int hue with
      | 0 -> chroma, x, 0.
      | 1 -> x, chroma, 0.
      | 2 -> 0., chroma, x
      | 3 -> 0., x, chroma
      | 4 -> x, 0., chroma
      | _ -> chroma, 0., x
    in
    let offset = lightness -. (chroma /. 2.) in
    { red = byte (red +. offset)
    ; green = byte (green +. offset)
    ; blue = byte (blue +. offset)
    ; alpha = byte (Hsla.alpha t)
    }
  ;;

  let to_hsla t =
    let red = Float.of_int t.red /. 255. in
    let green = Float.of_int t.green /. 255. in
    let blue = Float.of_int t.blue /. 255. in
    let maximum = Float.max red (Float.max green blue) in
    let minimum = Float.min red (Float.min green blue) in
    let chroma = maximum -. minimum in
    let lightness = (maximum +. minimum) /. 2. in
    let hue_degrees, saturation =
      if Float.equal chroma 0.
      then 0., 0.
      else (
        let sector =
          if Float.equal maximum red
          then (green -. blue) /. chroma
          else if Float.equal maximum green
          then ((blue -. red) /. chroma) +. 2.
          else ((red -. green) /. chroma) +. 4.
        in
        let sector = if Float.(sector < 0.) then sector +. 6. else sector in
        (* Clamp only conversion roundoff, never caller input. *)
        sector *. 60., Float.min 1. (chroma /. (1. -. Float.abs ((2. *. lightness) -. 1.))))
    in
    Hsla.create ~hue_degrees ~saturation ~lightness ~alpha:(Float.of_int t.alpha /. 255.)
    |> Or_error.ok_exn
  ;;

  let to_color t =
    Color.rgba ~red:t.red ~green:t.green ~blue:t.blue ~alpha:t.alpha |> Or_error.ok_exn
  ;;

  let digit = function
    | '0' .. '9' as c -> Some (Char.to_int c - Char.to_int '0')
    | 'a' .. 'f' as c -> Some (Char.to_int c - Char.to_int 'a' + 10)
    | 'A' .. 'F' as c -> Some (Char.to_int c - Char.to_int 'A' + 10)
    | _ -> None
  ;;

  let hex_digits text =
    if String.is_prefix text ~prefix:"#" then String.drop_prefix text 1 else text
  ;;

  let of_hex text =
    if String.length text > 9
    then Or_error.error_string "hex color is too long"
    else (
      let digits = hex_digits text in
      let length = String.length digits in
      if
        (not (List.mem [ 3; 4; 6; 8 ] length ~equal:Int.equal))
        || not (String.for_all digits ~f:(fun c -> Option.is_some (digit c)))
      then Or_error.error_string "expected 3, 4, 6 or 8 ASCII hex digits with optional #"
      else (
        let at index = digit digits.[index] |> Option.value_exn in
        let channel index =
          if length <= 4
          then at index * 17
          else (at (index * 2) * 16) + at ((index * 2) + 1)
        in
        Ok
          { red = channel 0
          ; green = channel 1
          ; blue = channel 2
          ; alpha = (if length = 4 || length = 8 then channel 3 else 255)
          }))
  ;;

  let to_hex t =
    let rgb = sprintf "#%02X%02X%02X" t.red t.green t.blue in
    if t.alpha = 255 then rgb else rgb ^ sprintf "%02X" t.alpha
  ;;
end

module Value = struct
  type t =
    | Empty
    | Color of Rgba.t
  [@@deriving equal, sexp_of]
end

module Alpha_policy = struct
  type t =
    | Allow_alpha
    | Opaque_only
  [@@deriving equal, sexp_of]
end

module Hex_draft = struct
  module Error = struct
    type t =
      | Syntax
      | Too_long
    [@@deriving equal, sexp_of]
  end

  type t =
    | Empty
    | Incomplete
    | Invalid of Error.t
    | Valid of Rgba.t
  [@@deriving equal, sexp_of]

  let parse text =
    if String.is_empty text
    then Empty
    else if String.length text > 9
    then Invalid Too_long
    else (
      let digits = Rgba.hex_digits text in
      if not (String.for_all digits ~f:(fun c -> Option.is_some (Rgba.digit c)))
      then Invalid Syntax
      else (
        match String.length digits with
        | 0 | 1 | 2 | 5 | 7 -> Incomplete
        | 3 | 4 | 6 | 8 -> Valid (Rgba.of_hex text |> Or_error.ok_exn)
        | _ -> Invalid Too_long))
  ;;
end
