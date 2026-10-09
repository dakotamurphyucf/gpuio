open Core
module W = Gpuio_protocol.Wire.Scrollbar
module Axis = W.Axis
module Mode = W.Mode
module Entrance = W.Entrance

module Track = struct
  type t =
    { background : Color.t option
    ; border : Color.t option
    ; width : float option
    }
  [@@deriving equal, sexp_of]

  let create ?background ?border ?width () =
    if Option.for_all width ~f:W.dimension
    then Ok { background; border; width }
    else Or_error.error_string "scrollbar track width must be finite and in 0..16384"
  ;;

  let empty = create () |> Or_error.ok_exn
end

module Thumb = struct
  type t =
    { background : Background.t option
    ; width : float option
    ; inset : float option
    ; radius : float option
    ; min_length : float option
    }
  [@@deriving equal, sexp_of]

  let create ?background ?width ?inset ?radius ?min_length () =
    if
      List.for_all [ width; inset; radius; min_length ] ~f:(Option.for_all ~f:W.dimension)
    then Ok { background; width; inset; radius; min_length }
    else Or_error.error_string "scrollbar thumb dimensions must be finite and in 0..16384"
  ;;

  let empty = create () |> Or_error.ok_exn
end

module Appearance = struct
  type t =
    { track : Track.t
    ; track_hover : Track.t
    ; track_pressed : Track.t
    ; thumb : Thumb.t
    ; thumb_hover : Thumb.t
    ; thumb_pressed : Thumb.t
    }
  [@@deriving equal, sexp_of]

  let create
        ?(track = Track.empty)
        ?(track_hover = Track.empty)
        ?(track_pressed = Track.empty)
        ?(thumb = Thumb.empty)
        ?(thumb_hover = Thumb.empty)
        ?(thumb_pressed = Thumb.empty)
        ()
    =
    { track; track_hover; track_pressed; thumb; thumb_hover; thumb_pressed }
  ;;

  let default = create ()
end

module Motion = struct
  type t = W.Motion.t [@@deriving equal, sexp_of]

  let create
        ?(idle = Time_ns.Span.of_sec 2.)
        ?(enter = Time_ns.Span.zero)
        ?(exit = Time_ns.Span.zero)
        ?(expand = Time_ns.Span.zero)
        ?(entrance = Entrance.Fade)
        ?(thumb_hover_entrance = Entrance.Fade)
        ()
    =
    let milliseconds span =
      let ms = Time_ns.Span.to_ms span in
      if Float.is_finite ms && Float.(ms >= 0. && ms <= 60_000.)
      then Ok (Float.iround_up_exn ms |> Int64.of_int)
      else Or_error.error_string "scrollbar durations must be in 0..60 seconds"
    in
    let open Or_error.Let_syntax in
    let%bind idle_ms = milliseconds idle in
    let%bind enter_ms = milliseconds enter in
    let%bind exit_ms = milliseconds exit in
    let%map expand_ms = milliseconds expand in
    { W.Motion.idle_ms; enter_ms; exit_ms; expand_ms; entrance; thumb_hover_entrance }
  ;;

  let default = create () |> Or_error.ok_exn
end

type t =
  { label : string
  ; axis : Axis.t
  ; mode : Mode.t
  ; appearance : Appearance.t
  ; motion : Motion.t
  }
[@@deriving equal, sexp_of]

let create
      ~label
      ?(axis = Axis.Both)
      ?(mode = Mode.Scrolling)
      ?(appearance = Appearance.default)
      ?(motion = Motion.default)
      ()
  =
  if
    String.length label <= 1024
    && (not (String.is_empty (String.strip label)))
    && Stdlib.String.is_valid_utf_8 label
    && not (String.contains label '\000')
  then Ok { label; axis; mode; appearance; motion }
  else
    Or_error.error_string
      "scrollbar label must be nonblank UTF-8 without NUL, at most 1024 bytes"
;;

let label t = t.label

module Expert = struct
  let to_wire t ~theme =
    let open Or_error.Let_syntax in
    let resolve f = function
      | None -> Ok None
      | Some value -> Or_error.map (f value) ~f:Option.some
    in
    let track (t : Track.t) =
      let%bind background = resolve (Theme.resolve theme) t.background in
      let%map border = resolve (Theme.resolve theme) t.border in
      { W.Track.background; border; width = t.width }
    in
    let thumb (t : Thumb.t) =
      let%map background =
        resolve
          (fun background -> Style.Expert.background_to_wire background ~theme)
          t.background
      in
      { W.Thumb.background
      ; width = t.width
      ; inset = t.inset
      ; radius = t.radius
      ; min_length = t.min_length
      }
    in
    let%bind track_base = track t.appearance.track in
    let%bind track_hover = track t.appearance.track_hover in
    let%bind track_pressed = track t.appearance.track_pressed in
    let%bind thumb_base = thumb t.appearance.thumb in
    let%bind thumb_hover = thumb t.appearance.thumb_hover in
    let%map thumb_pressed = thumb t.appearance.thumb_pressed in
    { W.label = t.label
    ; axis = t.axis
    ; mode = t.mode
    ; motion = t.motion
    ; appearance =
        { track = track_base
        ; track_hover
        ; track_pressed
        ; thumb = thumb_base
        ; thumb_hover
        ; thumb_pressed
        }
    }
  ;;
end
