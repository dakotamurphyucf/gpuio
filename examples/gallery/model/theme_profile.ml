open Core

module Colors = struct
  type 'a t =
    { background : 'a
    ; surface : 'a
    ; foreground : 'a
    ; muted : 'a
    ; accent : 'a
    ; border : 'a
    ; on_solid : 'a
    ; success : 'a
    ; warning : 'a
    ; danger : 'a
    }
  [@@deriving equal, sexp]
end

module File = struct
  module Mode = struct
    type t =
      | Light
      | Dark
    [@@deriving sexp]
  end

  type t =
    { version : int
    ; name : string
    ; appearance : Mode.t
    ; colors : string Colors.t
    }
  [@@deriving sexp]
end

type t =
  { name : string
  ; appearance : Appearance.t
  ; colors : Gpuio.Color.t Colors.t
  }
[@@deriving equal]

let maximum_bytes = 16384
let name t = t.name
let appearance t = t.appearance
let background t = t.colors.background
let surface t = t.colors.surface
let foreground t = t.colors.foreground
let muted t = t.colors.muted
let accent t = t.colors.accent
let border t = t.colors.border

let presentation t =
  let c = t.colors in
  Gpuio.Presentation.Appearance.create
    ~surface:c.surface
    ~raised:c.background
    ~foreground:c.foreground
    ~muted:c.muted
    ~border:c.border
    ~on_solid:c.on_solid
    ~accent:c.accent
    ~success:c.success
    ~warning:c.warning
    ~danger:c.danger
;;

let bounded_structure sexp =
  let rec loop = function
    | [] -> true
    | (_, Sexp.Atom _) :: rest -> loop rest
    | (depth, Sexp.List children) :: rest ->
      depth <= 16
      && loop
           (List.fold children ~init:rest ~f:(fun rest child ->
              (depth + 1, child) :: rest))
  in
  loop [ 1, sexp ]
;;

let decode contents =
  let open Or_error.Let_syntax in
  if String.length contents > maximum_bytes
  then Or_error.error_string "Theme profile exceeds 16 KiB"
  else if (not (Stdlib.String.is_valid_utf_8 contents)) || String.contains contents '\000'
  then Or_error.error_string "Theme profile must be UTF-8 without NUL"
  else (
    let%bind sexp = Or_error.try_with (fun () -> Sexp.of_string contents) in
    if not (bounded_structure sexp)
    then Or_error.error_string "Theme profile nesting exceeds 16"
    else (
      let%bind file = Or_error.try_with (fun () -> File.t_of_sexp sexp) in
      if file.version <> 1
      then Or_error.error_string "Unsupported theme profile version"
      else if
        String.is_empty (String.strip file.name)
        || String.length file.name > 128
        || String.contains file.name '\000'
        || not (Stdlib.String.is_valid_utf_8 file.name)
      then Or_error.error_string "Theme name must contain 1..128 UTF-8 bytes without NUL"
      else (
        let color field value =
          Gpuio.Color_value.Rgba.of_hex value
          |> Or_error.map ~f:Gpuio.Color_value.Rgba.to_color
          |> Or_error.tag ~tag:("Theme color " ^ field)
        in
        let c = file.colors in
        let%bind background = color "background" c.background in
        let%bind surface = color "surface" c.surface in
        let%bind foreground = color "foreground" c.foreground in
        let%bind muted = color "muted" c.muted in
        let%bind accent = color "accent" c.accent in
        let%bind border = color "border" c.border in
        let%bind on_solid = color "on_solid" c.on_solid in
        let%bind success = color "success" c.success in
        let%bind warning = color "warning" c.warning in
        let%map danger = color "danger" c.danger in
        { name = file.name
        ; appearance =
            (match file.appearance with
             | Light -> Appearance.Light
             | Dark -> Dark)
        ; colors =
            { background
            ; surface
            ; foreground
            ; muted
            ; accent
            ; border
            ; on_solid
            ; success
            ; warning
            ; danger
            }
        })))
;;
