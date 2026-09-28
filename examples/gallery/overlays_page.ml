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
  let modal, set_modal = B.state Modal.Closed graph in
  let popover, set_popover = B.state false graph in
  let notice, set_notice = B.state "Nothing has been changed." graph in
  let open B.Let_syntax in
  let%arr p = palette
  and modal = modal
  and set_modal = set_modal
  and popover = popover
  and set_popover = set_popover
  and notice = notice
  and set_notice = set_notice in
  let close _ = set_modal Modal.Closed in
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
          ~config:(Tooltip.Config.create ~label:"Helpful preview" () |> ok)
          ~style:panel_style
          ~anchor:(Palette.button p "Focus for a tip" E.Ignore)
          ~content:
            (Palette.text
               p
               "Tooltips also appear when their trigger receives keyboard focus.")
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
        [ V.row
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
    ; V.dialog
        ~config:(overlay "Preview dialog")
        ~style:panel_style
        ~on_dismiss:close
        (content Dialog (fun () ->
           panel
             "A focused conversation"
             [ Palette.text
                 p
                 "Tab stays within this dialog. Escape returns you to the trigger."
             ; Palette.button p "Close dialog" (set_modal Closed)
             ]))
    ; V.sheet
        ~config:(Sheet.Config.create ~label:"Preview drawer" ~extent:380. () |> ok)
        ~style:panel_style
        ~on_dismiss:close
        (content Sheet (fun () ->
           panel
             "Workspace details"
             [ Palette.text p "A drawer shares the same native modal focus behavior."
             ; Palette.button p "Close drawer" (set_modal Closed)
             ]))
    ; V.alert_dialog
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
