open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Input = Gpuio_eio.Text_input

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn
let id s = Split_group.Id.of_string s |> ok
let full = Length.percent_exn 100.

let counter graph =
  B.state_machine0
    ~default_model:0L
    ~apply_action:(fun _ value () ->
      if Int64.equal value Int64.max_value then value else Int64.succ value)
    graph
;;

let component window palette graph =
  let reverse, toggle_reverse = B.toggle ~default_model:false graph in
  let inspector, toggle_inspector = B.toggle ~default_model:true graph in
  let outline, toggle_outline = B.toggle ~default_model:false graph in
  let vertical, toggle_vertical = B.toggle ~default_model:false graph in
  let limited, toggle_limited = B.toggle ~default_model:false graph in
  let grips, toggle_grips = B.toggle ~default_model:false graph in
  let serial, request = counter graph in
  let generation, reset = counter graph in
  let observation, observe =
    B.state "Drag a divider, or focus it and use the arrow keys." graph
  in
  let editor =
    Input.create
      window
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Multiline ~label:"Resizable workspace draft" ()
            |> ok))
      ~initial_text:
        "A retained draft. Reorder the panels, hide the inspector, or add an outline \
         without losing your edits."
      graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and reverse = reverse
  and toggle_reverse = toggle_reverse
  and inspector = inspector
  and toggle_inspector = toggle_inspector
  and outline = outline
  and toggle_outline = toggle_outline
  and vertical = vertical
  and toggle_vertical = toggle_vertical
  and limited = limited
  and toggle_limited = toggle_limited
  and grips = grips
  and toggle_grips = toggle_grips
  and serial = serial
  and request = request
  and generation = generation
  and reset = reset
  and observation = observation
  and observe = observe
  and editor = editor in
  let names = [ "files"; "draft"; "inspector" ] @ if outline then [ "outline" ] else [] in
  let names = if reverse then List.rev names else names in
  let label = function
    | "files" -> "Files"
    | "draft" -> "Draft"
    | "inspector" -> "Inspector"
    | _ -> "Outline"
  in
  let panels =
    List.map names ~f:(fun name ->
      Split_group.Panel.create
        (id name)
        ~label:(label name)
        ~initial_size:(if String.equal name "draft" then 150. else 100.)
        ~minimum_size:60.
        ~maximum_size:(if limited then 200. else 800.)
        ~visible:((not (String.equal name "inspector")) || inspector)
        ()
      |> ok)
  in
  let resize =
    if Int64.equal serial 0L
    then None
    else Some (Split_group.Resize_request.create (id "draft") ~size:320. ~serial |> ok)
  in
  let config =
    Split_group.Config.create
      ~label:"Resizable workspace"
      ~axis:(if vertical then Vertical else Horizontal)
      ~reset_generation:generation
      ?resize
      panels
    |> ok
  in
  let pane name =
    V.column
      ~style:
        (style
           [ Width full
           ; Height full
           ; Min_width (px 0.)
           ; Padding (px 14.)
           ; Gap (px 10.)
           ; Overflow_y Scroll
           ])
      [ Palette.text p ~size:16. (label name)
      ; (match name with
         | "draft" ->
           Input.view
             ~style:(style [ Width full; Height (px 120.); Min_width (px 0.) ])
             editor
         | "files" ->
           V.column
             ~style:(style [ Gap (px 8.) ])
             [ Palette.text p "workspace.ml"
             ; Palette.text p ~muted:true "notes.md"
             ; Palette.text p ~muted:true "theme.ml"
             ]
         | "inspector" ->
           Palette.text
             p
             ~muted:true
             "All panels have their own size limits. Hidden panels keep their size and \
              state."
         | _ -> Palette.text p ~muted:true "Overview\nDraft\nReview\nNext steps")
      ]
  in
  let handle_style =
    style [ Background (Background.solid (Palette.border p)); Radius 3. ]
    |> fun style ->
    Style.with_state_exn
      style
      Hovered
      [ Background (Background.solid (Palette.accent p)) ]
    |> fun style ->
    Style.with_state_exn
      style
      Focused
      [ Background (Background.solid (Palette.accent p)) ]
    |> fun style ->
    Style.with_state_exn
      style
      Pressed
      [ Background (Background.solid (Palette.accent p)) ]
  in
  let appearance =
    Split_group.Appearance.create ~thickness:3. ~hit_extent:16. ~handle_style () |> ok
  in
  Palette.card
    p
    ~title:"A workspace that adapts to you"
    [ V.row
        ~style:(style [ Gap (px 8.); Wrap Wrap ])
        [ Palette.button p "Reorder panels" toggle_reverse
        ; Palette.button
            p
            (if inspector then "Hide inspector" else "Show inspector")
            toggle_inspector
        ; Palette.button
            p
            (if outline then "Remove outline" else "Add outline")
            toggle_outline
        ; Palette.button
            p
            (if vertical then "Horizontal layout" else "Vertical layout")
            toggle_vertical
        ; Palette.button p "Resize draft to 320 px" (request ())
        ; Palette.button p "Reset sizes" (reset ())
        ]
    ; V.row
        ~style:(style [ Gap (px 16.); Wrap Wrap ])
        [ V.switch
            ~checked:limited
            ~on_toggle:toggle_limited
            "Constrain each panel to 200 px"
        ; V.switch ~checked:grips ~on_toggle:toggle_grips "Custom divider grips"
        ]
    ; V.split_group
        ~key:(Key.of_string_exn "flat-workspace")
        ~config
        ~appearance
        ~style:
          (style
             [ Width full
             ; Height (px (if vertical then 360. else 220.))
             ; Border_width 1.
             ; Border_color (Palette.border p)
             ; Radius 12.
             ])
        ~panels:(List.map names ~f:(fun name -> id name, pane name))
        ~handles:
          (if grips
           then
             List.map names ~f:(fun name ->
               ( id name
               , V.text
                   ~style:(style [ Foreground (Palette.accent p); Font_size 12. ])
                   (if vertical then "···" else "⋮") ))
           else [])
        ~on_resize:(fun snapshot ->
          observe
            ("Completed sizes: "
             ^ String.concat
                 ~sep:" · "
                 (List.map (Split_group.Snapshot.sizes snapshot) ~f:(fun (id, size) ->
                    sprintf "%s %.0f px" (Split_group.Id.to_string id) size))))
        ()
      |> ok
    ; Palette.text p ~muted:true observation
    ]
;;
