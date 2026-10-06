open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Menu = Gpuio.Menu
module Controller = Gpuio_eio.Menu_controller
module Command = Gpuio.Command

let run = Command.Id.of_string "run" |> Or_error.ok_exn
let position x y = Menu.Position.create ~x ~y |> Or_error.ok_exn

let create ~platform ~app window graph =
  let controller = Controller.create window graph in
  let other_controller = Controller.create window graph in
  let count, set_count = B.state 0 graph in
  let replaced, set_replaced = B.state false graph in
  let observing, set_observing = B.state true graph in
  let saved, set_saved = B.state_opt ~equal:Menu.Snapshot.equal graph in
  let status, set_status = B.state "Ready" graph in
  let artwork, set_artwork = B.state_opt graph in
  let released, set_released = B.state false graph in
  let show_icons, toggle_icons = B.toggle ~default_model:true graph in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_activate:
      (let%arr set_artwork = set_artwork
       and set_status = set_status in
       let source =
         Gpuio.Asset.Source.of_bytes
           ~format:Svg
           {|<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><path d="M3 2L14 8L3 14Z" fill="black"/></svg>|}
         |> Or_error.ok_exn
       in
       let open E.Let_syntax in
       let%bind result =
         Gpuio_eio.Asset.register app ~scope:(Gpuio_eio.App.Window.scope window) source
       in
       match result with
       | Ok asset -> set_artwork (Some asset)
       | Error error ->
         set_status
           ("Icon registration failed: "
            ^ Sexp.to_string (Gpuio_eio.Asset.Error.sexp_of_t error)))
    graph;
  let input =
    Gpuio_eio.Text_input.create
      window
      ~config:
        (B.return
           (Gpuio.Text_input.Config.create ~mode:Single_line ~label:"Draft" ()
            |> Or_error.ok_exn))
      graph
  in
  let%arr controller = controller
  and other_controller = other_controller
  and count = count
  and set_count = set_count
  and observing = observing
  and set_observing = set_observing
  and replaced = replaced
  and set_replaced = set_replaced
  and saved = saved
  and set_saved = set_saved
  and status = status
  and set_status = set_status
  and input = input
  and artwork = artwork
  and released = released
  and set_released = set_released
  and show_icons = show_icons
  and toggle_icons = toggle_icons in
  let decorate view =
    match artwork with
    | Some asset when platform && show_icons ->
      V.with_menu_item_icons
        view
        ~items:
          [ ( Menu.Item_path.of_list [ 0; 1 ] |> Or_error.ok_exn
            , Gpuio_eio.Asset.handle asset )
          ]
      |> Or_error.ok_exn
    | Some _ | None -> view
  in
  let report label operation =
    let open E.Let_syntax in
    let%bind result = operation in
    let result =
      match result with
      | Ok () -> "accepted"
      | Error error -> Sexp.to_string (Menu.Command_error.sexp_of_t error)
    in
    set_status (label ^ ": " ^ result)
  in
  let command label operation = report label (Controller.command controller operation) in
  let commands =
    Command.Registry.create
      [ Command.create
          ~id:run
          ~label:"Run"
          ~on_invoke:(fun () -> set_count (count + 1))
          ()
        |> Or_error.ok_exn
      ]
    |> Or_error.ok_exn
  in
  let menu =
    Menu.create
      ~label:(if replaced then "Replacement" else "Original")
      [ Label (if replaced then "Replacement actions" else "Original actions")
      ; Command run
      ]
    |> Or_error.ok_exn
  in
  let ready = Option.is_some (Controller.snapshot controller) in
  V.command_scope
    ~commands
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 20.); Gap (Gpuio.Length.px_exn 10.) ])
    [ V.text "Positioned context menus"
    ; V.text (sprintf "Total runs: %d" count)
    ; V.text status
    ; V.text
        (match saved, Controller.snapshot controller with
         | Some old, Some current when not (Menu.Expert.same_owner old current) ->
           "Menu subscription updated"
         | Some _, Some _ | None, _ | _, None -> "")
    ; V.context_menu
        ~platform
        ~key:(Controller.key controller)
        ?on_change:(if observing then Some (Controller.observe controller) else None)
        ~menu
        (Gpuio_eio.Text_input.view input)
      |> decorate
    ; V.context_menu
        ~platform
        ~key:(Controller.key other_controller)
        ~on_change:(Controller.observe other_controller)
        ~menu
        (V.text "Secondary popup anchor")
      |> decorate
    ; V.button
        ~disabled:(Option.is_none (Controller.snapshot other_controller))
        ~on_click:
          (report
             "Other menu"
             (Controller.command other_controller (Show (position 250. 150.))))
        "Show other owner"
    ; V.button
        ~disabled:(not ready)
        ~on_click:(command "Show first" (Show (position 100. 100.)))
        "Show first position"
    ; V.button
        ~disabled:(not ready)
        ~on_click:(command "Show second" (Show (position 400. 200.)))
        "Show second position"
    ; V.button ~disabled:(not ready) ~on_click:(command "Close" Close) "Close popup"
    ; V.button
        ~disabled:(not ready)
        ~on_click:(set_saved (Controller.snapshot controller))
        "Capture old menu"
    ; V.button ~on_click:(set_replaced (not replaced)) "Replace menu definition"
    ; V.button
        ~on_click:
          (E.Many
             [ set_observing (not observing)
             ; Controller.reset controller
             ; set_status
                 (if observing then "Observer detached" else "Observer attaching")
             ])
        (if observing then "Detach observer" else "Attach observer")
    ; V.button
        ~disabled:(Option.is_none saved)
        ~on_click:
          (match saved with
           | None -> E.Ignore
           | Some snapshot ->
             report
               "Saved menu"
               (Gpuio_eio.App.Window.Expert.menu_command
                  window
                  snapshot
                  (Show (position 100. 100.))))
        "Show captured menu"
    ; V.button
        ~on_click:toggle_icons
        (if show_icons then "Hide menu icons" else "Show menu icons")
    ; V.button
        ~disabled:(released || Option.is_none artwork)
        ~on_click:
          (match artwork with
           | None -> E.Ignore
           | Some asset ->
             E.Many
               [ E.of_thunk (fun () -> Gpuio_eio.Asset.release asset); set_released true ])
        (if released then "Icon source released" else "Release icon source")
    ; V.button
        ~on_click:(E.of_thunk (fun () -> Gpuio_eio.App.Window.close window))
        "Close window"
    ]
;;
