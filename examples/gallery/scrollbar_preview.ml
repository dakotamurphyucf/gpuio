open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module E = Bonsai.Effect
module Scope = Gpuio_eio.Scope

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

type t =
  { controls : V.t
  ; viewport : V.t
  ; description : Scrollbar.t option
  }

let controls t = t.controls
let viewport t = t.viewport
let description t = t.description

let mode_label = function
  | Scrollbar.Mode.Scrolling -> "While scrolling"
  | Hover -> "On hover"
  | Always -> "Always visible"
;;

let axis_label = function
  | Scrollbar.Axis.Both -> "Both axes"
  | Horizontal -> "Horizontal only"
  | Vertical -> "Vertical only"
;;

let next_axis axis : Scrollbar.Axis.t =
  match axis with
  | Scrollbar.Axis.Both -> Horizontal
  | Horizontal -> Vertical
  | Vertical -> Both
;;

let appearance p =
  let fill =
    Background.linear_gradient_in
      Oklab
      ~angle:135.
      ~from:(Palette.accent p, 0.)
      ~to_:(Palette.muted p, 1.)
    |> ok
  in
  Scrollbar.Appearance.create
    ~track:(Scrollbar.Track.create ~background:(Palette.surface p) ~width:18. () |> ok)
    ~track_hover:(Scrollbar.Track.create ~border:(Palette.border p) () |> ok)
    ~track_pressed:(Scrollbar.Track.create ~background:(Palette.border p) () |> ok)
    ~thumb:
      (Scrollbar.Thumb.create ~background:fill ~width:8. ~radius:5. ~min_length:40. ()
       |> ok)
    ~thumb_hover:(Scrollbar.Thumb.create ~width:10. () |> ok)
    ~thumb_pressed:
      (Scrollbar.Thumb.create
         ~background:(Background.solid (Palette.accent p))
         ~width:10.
         ()
       |> ok)
    ()
;;

let motion =
  Scrollbar.Motion.create
    ~idle:(Time_ns.Span.of_sec 0.8)
    ~enter:(Time_ns.Span.of_ms 160.)
    ~exit:(Time_ns.Span.of_ms 240.)
    ~expand:(Time_ns.Span.of_ms 120.)
    ~entrance:Slide_and_fade
    ~thumb_hover_entrance:Fade
    ()
  |> ok
;;

let component app window palette graph =
  let mode = B.Expert.Var.create Scrollbar.Mode.Always in
  let busy = B.Expert.Var.create false in
  let status =
    B.Expert.Var.create "Read the system preference when you want to apply it."
  in
  let scope =
    Preview_scope.acquire
      window
      ~name:"scrollbar-preference"
      ~create:(fun scope ->
        E.of_thunk (fun () ->
          Scope.on_cancel scope (fun () -> B.Expert.Var.set busy false)
          |> Or_error.map ~f:(fun _ -> scope)))
      graph
  in
  let choose scope candidate =
    E.of_thunk (fun () ->
      if Scope.is_active scope && not (B.Expert.Var.get busy)
      then (
        B.Expert.Var.set mode candidate;
        B.Expert.Var.set status "Using your explicit choice."))
  in
  let refresh scope =
    let open E.Let_syntax in
    let%bind start =
      E.of_thunk (fun () ->
        if (not (Scope.is_active scope)) || B.Expert.Var.get busy
        then false
        else (
          B.Expert.Var.set busy true;
          true))
    in
    if not start
    then E.Ignore
    else (
      let%bind result = Gpuio_eio.Desktop.scrollbar_preference app in
      E.of_thunk (fun () ->
        if Scope.is_active scope
        then (
          B.Expert.Var.set busy false;
          match result with
          | Ok Auto_hide ->
            B.Expert.Var.set mode Scrolling;
            B.Expert.Var.set status "System preference applied: hide when idle."
          | Ok Always_visible ->
            B.Expert.Var.set mode Always;
            B.Expert.Var.set status "System preference applied: always visible."
          | Error Unsupported ->
            B.Expert.Var.set
              status
              "System preference unavailable on this backend. Your choice was kept."
          | Error error ->
            B.Expert.Var.set
              status
              ("Could not read the preference: "
               ^ Sexp.to_string_hum (Gpuio.Desktop.Error.sexp_of_t error)
               ^ ". Your choice was kept."))))
  in
  let axis, set_axis = B.state Scrollbar.Axis.Both graph in
  let custom, toggle_custom = B.toggle ~default_model:true graph in
  let styled, toggle_styled = B.toggle ~default_model:true graph in
  let animated, toggle_animated = B.toggle ~default_model:true graph in
  let rows, set_rows = B.state 24 graph in
  let open B.Let_syntax in
  let%arr p = palette
  and mode = B.Expert.Var.value mode
  and busy = B.Expert.Var.value busy
  and status = B.Expert.Var.value status
  and scope = scope
  and axis = axis
  and set_axis = set_axis
  and custom = custom
  and toggle_custom = toggle_custom
  and styled = styled
  and toggle_styled = toggle_styled
  and animated = animated
  and toggle_animated = toggle_animated
  and rows = rows
  and set_rows = set_rows in
  let active, refresh, choose =
    match scope with
    | Preview_scope.Ready scope -> true, refresh scope, choose scope
    | Loading | Failed _ -> false, E.Ignore, fun _ -> E.Ignore
  in
  let status =
    match scope with
    | Loading -> "Preparing scrollbar preferences…"
    | Failed _ -> "Scrollbar preference controls are unavailable."
    | Ready _ -> status
  in
  let description =
    Option.some_if
      custom
      (Scrollbar.create
         ~label:"Collection preview"
         ~axis
         ~mode
         ~appearance:(if styled then appearance p else Scrollbar.Appearance.default)
         ~motion:(if animated then motion else Scrollbar.Motion.default)
         ()
       |> ok)
  in
  let controls =
    Palette.card
      p
      ~title:"Scroll your way"
      [ V.row
          ~style:(style [ Gap (px 8.); Wrap Wrap ])
          (List.map [ Scrollbar.Mode.Scrolling; Hover; Always ] ~f:(fun candidate ->
             Palette.button
               p
               ~disabled:(busy || not active)
               ~selected:(Scrollbar.Mode.equal mode candidate)
               (mode_label candidate)
               (choose candidate)))
      ; Palette.button p ~disabled:(busy || not active) "Use system preference" refresh
      ; Palette.text p ~muted:true (if busy then "Reading system preference…" else status)
      ; V.row
          ~style:(style [ Gap (px 12.); Wrap Wrap ])
          [ Palette.button p (axis_label axis) (set_axis (next_axis axis))
          ; V.switch ~checked:custom ~on_toggle:toggle_custom "Custom scrollbars"
          ; V.switch ~checked:styled ~on_toggle:toggle_styled "Gradient thumbs"
          ; V.switch ~checked:animated ~on_toggle:toggle_animated "Animate scrollbars"
          ]
      ; Palette.text
          p
          ~muted:true
          "These settings apply to each preview. Hover or drag a thumb; focus a \
           scrollbar with Tab to use arrows, Page Up/Down, Home or End. Escape cancels a \
           drag. System preference is a snapshot; apply it again after changing your \
           settings."
      ]
  in
  let viewport =
    let content =
      V.column
        ~style:(style [ Width (px 900.); Shrink 0. ])
        (List.init rows ~f:(fun n ->
           V.row
             ~key:(Key.of_int n)
             ~style:
               (style
                  [ Height (px 38.)
                  ; Shrink 0.
                  ; Gap (px 20.)
                  ; Padding_left (px 16.)
                  ; Padding_right (px 16.)
                  ; Align_items Center
                  ; Border_bottom_width 1.
                  ; Border_color (Palette.border p)
                  ])
             [ Palette.text p ~muted:true (sprintf "%02d" (n + 1))
             ; Palette.text p "A little room to explore"
             ; Palette.text p ~muted:true "横にスクロール · Shift your perspective →"
             ]))
    in
    let scroll =
      V.column
        ~style:
          (style
             [ Height (px 235.)
             ; Shrink 0.
             ; Overflow_x Scroll
             ; Overflow_y Scroll
             ; Background (Background.solid (Palette.background p))
             ; Foreground (Palette.foreground p)
             ; Radius 10.
             ])
        [ content ]
      |> fun view -> V.with_scrollbar view description |> ok
    in
    Palette.card
      p
      ~title:"Room in both directions"
      [ V.row
          ~style:(style [ Gap (px 10.); Wrap Wrap ])
          [ Palette.button
              p
              ~disabled:(rows >= 60)
              "Add six rows"
              (set_rows (Int.min 60 (rows + 6)))
          ; Palette.button p ~disabled:(rows = 24) "Reset rows" (set_rows 24)
          ; Palette.text p (sprintf "%d rows" rows)
          ]
      ; scroll
      ; Palette.text
          p
          ~muted:true
          "Changing the appearance keeps your position. Axis settings choose the bars; \
           the content can still scroll in both directions."
      ]
  in
  { controls; viewport; description }
;;
