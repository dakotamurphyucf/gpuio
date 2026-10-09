open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module E = Bonsai.Effect

module Modal = struct
  type t =
    | Closed
    | Dialog
    | Sheet
    | Confirm
end

let ok = Or_error.ok_exn
let px = Length.px_exn
let style = Style.create_exn

let component palette graph =
  let placement_preview = Placement_preview.component palette graph in
  let modal, set_modal = B.state Modal.Closed graph in
  let popover, set_popover = B.state false graph in
  let tinted_backdrop, toggle_backdrop = B.toggle ~default_model:true graph in
  let animate, toggle_animation = B.toggle ~default_model:true graph in
  let reserve_chrome, toggle_chrome = B.toggle ~default_model:false graph in
  let sheet_edge, set_sheet_edge = B.state Sheet.Edge.Right graph in
  let notice, set_notice = B.state "Nothing has been changed." graph in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr set_modal = set_modal
       and set_popover = set_popover in
       E.Many [ set_modal Modal.Closed; set_popover false ])
    graph;
  let%arr p = palette
  and placement_preview = placement_preview
  and modal = modal
  and set_modal = set_modal
  and popover = popover
  and set_popover = set_popover
  and tinted_backdrop = tinted_backdrop
  and toggle_backdrop = toggle_backdrop
  and animate = animate
  and toggle_animation = toggle_animation
  and reserve_chrome = reserve_chrome
  and toggle_chrome = toggle_chrome
  and sheet_edge = sheet_edge
  and set_sheet_edge = set_sheet_edge
  and notice = notice
  and set_notice = set_notice in
  let motion = if animate then Overlay.Motion.Enter else Immediate in
  let tooltip_motion = if animate then Tooltip.Motion.Enter_and_switch else Immediate in
  let sheet_insets =
    if reserve_chrome
    then Sheet.Insets.create ~top:56. ~right:16. ~bottom:16. ~left:16. () |> ok
    else Sheet.Insets.zero
  in
  let next_sheet_edge =
    match sheet_edge with
    | Sheet.Edge.Right -> Sheet.Edge.Bottom
    | Bottom -> Left
    | Left -> Top
    | Top -> Right
  in
  let close _ = set_modal Modal.Closed in
  let backdrop =
    if tinted_backdrop
    then Color.with_opacity (Palette.accent p) 0.28 |> ok
    else Color.rgba ~red:0 ~green:0 ~blue:0 ~alpha:0 |> ok
  in
  let panel_style =
    style
      [ Padding (px 24.)
      ; Gap (px 18.)
      ; Radius 14.
      ; Background (Background.solid (Palette.surface p))
      ; Foreground (Palette.foreground p)
      ]
  in
  let panel title children =
    V.column ~style:(style [ Gap (px 18.) ]) (Palette.text p ~size:22. title :: children)
  in
  let overlay label =
    Overlay.Config.create ~label ~width:440. ~dismiss_on_outside_pointer:true () |> ok
  in
  let content which f =
    match modal, which with
    | Modal.Dialog, Modal.Dialog | Sheet, Sheet | Confirm, Confirm -> Some (f ())
    | (Closed | Dialog | Sheet | Confirm), (Closed | Dialog | Sheet | Confirm) -> None
  in
  let tooltips =
    V.row
      ~style:(style [ Gap (px 12.); Wrap Wrap ])
      [ V.tooltip
          ~config:
            (Tooltip.Config.create ~label:"Helpful preview" ~motion:tooltip_motion ()
             |> ok)
          ~style:panel_style
          ~anchor:(Palette.button p "Focus for a tip" E.Ignore)
          ~content:
            (Palette.text
               p
               "Tooltips also appear when their trigger receives keyboard focus.")
          ()
      ; V.tooltip
          ~config:
            (Tooltip.Config.create ~label:"Streaming help" ~motion:tooltip_motion () |> ok)
          ~style:panel_style
          ~anchor:(Palette.button p "Streaming help" E.Ignore)
          ~content:
            (Palette.text
               p
               "Responses can stream while the native interface stays responsive.")
          ()
      ; V.tooltip
          ~config:
            (Tooltip.Config.create ~label:"Keyboard help" ~motion:tooltip_motion () |> ok)
          ~style:panel_style
          ~anchor:(Palette.button p "Keyboard help" E.Ignore)
          ~content:
            (Palette.text
               p
               "Move between these triggers to preview a native tooltip switch.")
          ()
      ; V.hover_card
          ~config:(Hover_card.Config.create ~label:"Contributor profile" () |> ok)
          ~style:panel_style
          ~anchor:(Palette.button p "Contributor" E.Ignore)
          ~content:
            (panel
               "Aster"
               [ Palette.text p "Building thoughtful native interfaces."
               ; Palette.button
                   p
                   "View profile"
                   (set_notice "Contributor profile selected")
               ])
          ()
      ]
  in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ Palette.card
        p
        ~title:"Keep the next step in context"
        [ Palette.button p ~selected:tinted_backdrop "Tinted backdrop" toggle_backdrop
        ; Palette.button p ~selected:animate "Animate opening" toggle_animation
        ; V.row
            ~style:(style [ Gap (px 12.); Wrap Wrap ])
            [ Palette.button p "Open dialog" (set_modal Dialog)
            ; Palette.button p "Open drawer" (set_modal Sheet)
            ; Palette.button p "Review confirmation" (set_modal Confirm)
            ; V.popover
                ~config:(overlay "Details popover")
                ~style:panel_style
                ~on_dismiss:(fun _ -> set_popover false)
                ~anchor:(Palette.button p "Show details" (set_popover true))
                (if popover
                 then
                   Some
                     (panel
                        "A closer look"
                        [ Palette.text p "This panel stays anchored to its trigger."
                        ; Palette.button p "Done with details" (set_popover false)
                        ])
                 else None)
            ]
        ; Palette.text p notice
        ]
    ; Palette.card p ~title:"Useful help, right where you need it" [ tooltips ]
    ; placement_preview
    ; V.dialog
        ~backdrop
        ~motion
        ~config:(overlay "Preview dialog")
        ~style:panel_style
        ~on_dismiss:close
        (content Dialog (fun () ->
           panel
             "A focused conversation"
             [ Palette.button p "Change backdrop" toggle_backdrop
             ; Palette.text
                 p
                 "Tab stays within this dialog. Escape returns you to the trigger."
             ; Palette.button p "Close dialog" (set_modal Closed)
             ]))
    ; V.sheet
        ~backdrop
        ~motion
        ~config:
          (Sheet.Config.create
             ~label:"Preview drawer"
             ~edge:sheet_edge
             ~insets:sheet_insets
             ~extent:380.
             ()
           |> ok)
        ~style:panel_style
        ~on_dismiss:close
        (content Sheet (fun () ->
           panel
             "Workspace details"
             [ Palette.text p "A drawer shares the same native modal focus behavior."
             ; V.switch
                 ~checked:reserve_chrome
                 ~on_toggle:toggle_chrome
                 "Reserve space for app chrome"
             ; Palette.button
                 p
                 "Move drawer to next edge"
                 (set_sheet_edge next_sheet_edge)
             ; Palette.text
                 p
                 "Reserved space stays covered by the modal backdrop. Resize to see the \
                  panel adapt."
             ; Palette.button p "Close drawer" (set_modal Closed)
             ]))
    ; V.alert_dialog
        ~backdrop
        ~motion
        ~config:(Alert_dialog.Config.create ~label:"Preview confirmation" () |> ok)
        ~style:panel_style
        ~on_dismiss:close
        (content Confirm (fun () ->
           panel
             "Reset this preview?"
             [ Palette.text p "This demonstrates confirmation without changing any files."
             ; Palette.button p "Keep preview" (set_modal Closed)
             ; Palette.button
                 p
                 "Confirm reset"
                 (E.Many [ set_notice "Preview reset confirmed."; set_modal Closed ])
             ]))
    ]
;;
