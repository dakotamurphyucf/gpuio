open Core

module Format = struct
  type t =
    | Png
    | Jpeg
    | Webp
    | Gif
    | Svg
    | Bmp
    | Tiff
    | Ico
    | Pnm
  [@@deriving equal, compare, sexp_of]

  let mime_type = function
    | Png -> "image/png"
    | Jpeg -> "image/jpeg"
    | Webp -> "image/webp"
    | Gif -> "image/gif"
    | Svg -> "image/svg+xml"
    | Bmp -> "image/bmp"
    | Tiff -> "image/tiff"
    | Ico -> "image/ico"
    | Pnm -> "image/x-portable-anymap"
  ;;
end

module Source = struct
  type t =
    { format : Format.t
    ; data : string
    }
  [@@deriving equal]

  let max_bytes = 16 * 1024 * 1024

  let of_bytes ~format data =
    if String.is_empty data || String.length data > max_bytes
    then Or_error.error_string "encoded asset must contain 1..16777216 bytes"
    else Ok { format; data }
  ;;

  let format t = t.format
  let bytes t = t.data
  let byte_length t = String.length t.data

  let sexp_of_t t =
    [%sexp { format = (t.format : Format.t); byte_length = (byte_length t : int) }]
  ;;
end
