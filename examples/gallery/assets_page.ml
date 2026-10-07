open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Registered = Gpuio_eio.Asset
module Copy = Gpuio_eio.Clipboard.Copy

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

module Sources = struct
  let landscape =
    {|<svg xmlns="http://www.w3.org/2000/svg" width="480" height="240" viewBox="0 0 480 240"><defs><linearGradient id="sky" x2="0" y2="1"><stop stop-color="#c0e9eb"/><stop offset="1" stop-color="#f4e5c9"/></linearGradient></defs><rect width="480" height="240" fill="url(#sky)"/><circle cx="363" cy="65" r="29" fill="#fff4d6"/><path d="M0 166Q90 78 208 160Q345 218 480 133V240H0Z" fill="#5a9995"/><path d="M0 200Q128 126 261 192Q370 228 480 163V240H0Z" fill="#246b68"/><path d="M0 232Q178 171 354 220Q420 230 480 209V240H0Z" fill="#154c50"/></svg>|}
  ;;

  let check =
    {|<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M5 12l4 4L19 6" fill="none" stroke="white" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/></svg>|}
  ;;
end

module Assets = struct
  type t =
    { landscape : Asset.Handle.t
    ; raster : Asset.Handle.t
    ; invalid : Asset.Handle.t
    ; check : Asset.Handle.t
    }

  let create app scope =
    let register format bytes =
      E.map
        (Registered.register app ~scope (Asset.Source.of_bytes ~format bytes |> ok))
        ~f:(fun result ->
          Result.map result ~f:Registered.handle
          |> Result.map_error ~f:(fun e ->
            Error.create_s [%sexp (e : Registered.Error.t)]))
    in
    E.bind (register Svg Sources.landscape) ~f:(function
      | Error e -> E.return (Error e)
      | Ok landscape ->
        E.bind (register Pnm Image_samples.gradient_pnm) ~f:(function
          | Error e -> E.return (Error e)
          | Ok raster ->
            E.bind (register Pnm "Intentionally malformed gallery image") ~f:(function
              | Error e -> E.return (Error e)
              | Ok invalid ->
                E.map
                  (register Svg Sources.check)
                  ~f:(Result.map ~f:(fun check -> { landscape; raster; invalid; check })))))
  ;;
end

let fit_label = function
  | Image.Fit.Contain -> "Contain"
  | Cover -> "Cover"
  | Fill -> "Fill"
  | Scale_down -> "Scale down"
  | None -> "Intrinsic size"
;;

let state_label = function
  | Image.State.Loading -> "Image loading…"
  | Ready metadata ->
    sprintf
      "Image ready: %d × %d pixels · %d frame(s)"
      (Image.Metadata.width_px metadata)
      (Image.Metadata.height_px metadata)
      (Image.Metadata.frames metadata)
  | Failed e -> "Image failed: " ^ Sexp.to_string_hum [%sexp (e : Image.Error.t)]
;;

let component app window palette graph =
  let resources =
    Preview_scope.acquire window ~name:"gallery-assets" ~create:(Assets.create app) graph
  in
  let raster, toggle_raster = B.toggle ~default_model:false graph in
  let broken, toggle_broken = B.toggle ~default_model:false graph in
  let fit, set_fit = B.state Image.Fit.Contain graph in
  let state, set_state = B.state Image.State.Loading graph in
  let icon_preset, set_icon_preset = B.state Icon_transform_sample.Default graph in
  let approvals, approve =
    B.state_machine0
      ~default_model:0
      ~apply_action:(fun _ count () -> if count = Int.max_value then count else count + 1)
      graph
  in
  let open B.Let_syntax in
  let copied, set_copied = B.state "Nothing copied yet." graph in
  let on_copied =
    B.map set_copied ~f:(fun set_copied text ->
      set_copied ("Copied: " ^ Gpuio.Clipboard.Text.to_string text))
  in
  let literal =
    Copy.create
      app
      ~text:(B.return (Gpuio.Clipboard.Text.of_string "Hello from GPUIO · λ 世界\n" |> ok))
      ~on_copied
      graph
  in
  let current =
    Copy.create
      app
      ~text:
        (B.map approvals ~f:(fun count ->
           Gpuio.Clipboard.Text.of_string (sprintf "Approval %d · λ 世界\n" count) |> ok))
      ~on_copied
      graph
  in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr set_state = set_state in
       set_state Image.State.Loading)
    graph;
  let%arr p = palette
  and resources = resources
  and raster = raster
  and toggle_raster = toggle_raster
  and broken = broken
  and toggle_broken = toggle_broken
  and fit = fit
  and set_fit = set_fit
  and state = state
  and set_state = set_state
  and icon_preset = icon_preset
  and set_icon_preset = set_icon_preset
  and approvals = approvals
  and approve = approve
  and literal = literal
  and current = current
  and copied = copied in
  match resources with
  | Preview_scope.Loading -> Palette.text p "Registering image samples…"
  | Failed e -> Palette.text p ("Images unavailable: " ^ Error.to_string_hum e)
  | Ready assets ->
    let image asset label fit =
      Image.Config.create
        ~asset
        ~description:(Image.Description.label label |> ok)
        ~fit
        ()
    in
    let transform = Icon_transform_sample.transform icon_preset in
    let decoration = Icon.Decoration.create ~asset:assets.check ?transform () |> ok in
    V.column
      ~style:(style [ Gap (px 20.) ])
      [ Palette.card
          p
          ~title:"A view worth keeping"
          [ V.row
              ~style:(style [ Gap (px 8.); Wrap Wrap ])
              [ Palette.button
                  p
                  (if raster then "Show vector landscape" else "Show raster gradient")
                  toggle_raster
              ; Palette.button
                  p
                  (if broken then "Restore image" else "Simulate decode failure")
                  toggle_broken
              ]
          ; V.row
              ~style:(style [ Gap (px 6.); Wrap Wrap ])
              (List.map
                 [ Image.Fit.Contain; Cover; Fill; Scale_down; None ]
                 ~f:(fun candidate ->
                   Palette.button
                     p
                     ~selected:(Image.Fit.equal fit candidate)
                     (fit_label candidate)
                     (set_fit candidate)))
          ; V.image
              ~key:(Key.of_string_exn "main-gallery-image")
              ~on_change:set_state
              ~style:
                (style
                   [ Width (Length.percent_exn 100.)
                   ; Height (px (Palette.size p 210.))
                   ; Radius 12.
                   ; Background (Background.solid (Palette.background p))
                   ])
              (image
                 (if broken
                  then assets.invalid
                  else if raster
                  then assets.raster
                  else assets.landscape)
                 "Gallery image"
                 fit)
          ; Palette.text p (state_label state)
          ; Palette.text p ~muted:true ("Image fit: " ^ fit_label fit)
          ]
      ; Palette.card
          p
          ~title:"Copy from your application"
          [ V.row
              ~style:(style [ Gap (px 8.); Wrap Wrap ])
              [ Copy.view
                  literal
                  ~label:"Copy literal"
                  ~copied_label:"Copied literal"
                  ~style:
                    (style
                       [ Padding (px 12.)
                       ; Radius 8.
                       ; Background (Background.solid (Palette.surface p))
                       ; Foreground (Palette.accent p)
                       ])
                  ()
              ; Copy.view
                  current
                  ~label:"Copy current approval"
                  ~copied_label:"Copied current approval"
                  ~style:
                    (style
                       [ Padding (px 12.)
                       ; Radius 8.
                       ; Background (Background.solid (Palette.surface p))
                       ; Foreground (Palette.accent p)
                       ])
                  ()
              ; Palette.button p "Advance approval value" (approve ())
              ]
          ; Palette.text p (sprintf "Current approval: %d" approvals)
          ; Palette.text p copied
          ; Palette.text
              p
              ~muted:true
              "Plain text is copied by the native UI. Current-value actions use the \
               latest application state; copied feedback clears after two seconds."
          ; (match Copy.error literal, Copy.error current with
             | None, None -> V.text ""
             | Some error, _ | _, Some error ->
               Palette.text
                 p
                 ("Copy request failed: "
                  ^ Sexp.to_string ([%sexp_of: Gpuio.Clipboard.Error.t] error)))
          ]
      ; Palette.card
          p
          ~title:"Small details, clear actions"
          [ Icon_transform_sample.controls
              p
              ~selected:icon_preset
              ~on_select:set_icon_preset
          ; Palette.text p ("Icon transform: " ^ Icon_transform_sample.label icon_preset)
          ; V.row
              ~style:(style [ Gap (px 20.); Align_items Center; Wrap Wrap ])
              [ V.image
                  ~style:(style [ Width (px 140.); Height (px 80.); Radius 10. ])
                  (image assets.raster "Gradient thumbnail" Cover)
              ; V.icon
                  ~style:
                    (style
                       [ Width (px (Palette.size p 40.))
                       ; Height (px (Palette.size p 40.))
                       ; Foreground (Palette.accent p)
                       ])
                  (Icon.Config.create
                     ~asset:assets.check
                     ?transform
                     ~description:(Image.Description.label "Check mark" |> ok)
                     ()
                   |> ok)
              ; V.button
                  ~leading_icon:decoration
                  ~on_click:(approve ())
                  ~style:
                    (style
                       [ Padding (px 12.)
                       ; Radius 8.
                       ; Foreground (Palette.accent p)
                       ; Background (Background.solid (Palette.background p))
                       ])
                  "Approve sample"
              ]
          ; Palette.text p (sprintf "Sample approvals: %d" approvals)
          ; Palette.text
              p
              ~muted:true
              "SVG keeps its colors as an image; an icon inherits its foreground. \
               Decorative button icons share the button's accessible name."
          ; Palette.text
              p
              ~muted:true
              "Transform the artwork while the button keeps its size, focus and action. \
               Default restores the original icon; Collapse hides its pixels."
          ]
      ]
;;
