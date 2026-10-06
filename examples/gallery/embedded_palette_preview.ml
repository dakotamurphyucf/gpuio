open Core
open Gpuio
module B = Bonsai.Cont
module E = Bonsai.Effect
module V = Gpuio_bonsai.View
module Controller = Gpuio_eio.Palette_controller

let ok = Or_error.ok_exn
let id name = Command.Id.of_string name |> ok

module Action = struct
  type t =
    | Add
    | Reset
end

let component window palette graph =
  let count, inject =
    B.state_machine0
      ~default_model:0
      ~apply_action:(fun _ count -> function
         | Action.Add -> count + 1
         | Reset -> 0)
      graph
  in
  let cancellations, cancel =
    B.state_machine0 ~default_model:0 ~apply_action:(fun _ count () -> count + 1) graph
  in
  let hidden, toggle_hidden = B.toggle ~default_model:false graph in
  let controller = Controller.create window graph in
  let note =
    Gpuio_eio.Text_input.create
      window
      ~initial_text:"Retained workspace note"
      ~config:
        (B.return
           (Text_input.Config.create ~mode:Single_line ~label:"Workspace note" () |> ok))
      graph
  in
  let open B.Let_syntax in
  let%arr p = palette
  and count = count
  and inject = inject
  and cancellations = cancellations
  and cancel = cancel
  and hidden = hidden
  and toggle_hidden = toggle_hidden
  and controller = controller
  and note = note in
  let add = id "embedded-add" in
  let reset = id "embedded-reset" in
  let select = id "embedded-select-note" in
  let commands =
    Command.Registry.create
      [ Command.create
          ~id:add
          ~label:"Add a step"
          ~on_invoke:(fun () -> inject Action.Add)
          ()
        |> ok
      ; Command.create
          ~id:reset
          ~label:"Reset steps"
          ~on_invoke:(fun () -> inject Action.Reset)
          ()
        |> ok
      ; Command.native ~id:select ~label:"Select workspace note" Select_all |> ok
      ]
    |> ok
  in
  V.command_scope
    ~commands
    [ Palette.card
        p
        ~title:"Commands in your workspace"
        [ Palette.text
            p
            ~muted:true
            "Search and run commands without leaving your workspace. The browser stays \
             open."
        ; Gpuio_eio.Text_input.view note
        ; Palette.button
            p
            "Focus command browser"
            (E.map (Controller.command controller Focus) ~f:(fun _ -> ()))
        ; Palette.button
            p
            (if hidden then "Show command browser" else "Hide command browser")
            toggle_hidden
        ; V.command_palette
            ~key:(Controller.key controller)
            ~style:(Style.create_exn (if hidden then [ Display Hidden ] else []))
            ~config:
              (Command_palette.Config.create
                 ~label:"Workspace commands"
                 ~commands:[ add; reset; select ]
                 ~presentation:Embedded
                 ~escape:Clear_query_first
                 ~placeholder:"Find a workspace action…"
                 ()
               |> ok)
            ~on_change:(Controller.observe controller)
            ~on_dismiss:(function
              | Escape -> cancel ()
              | Selected _ | Outside_pointer -> E.Ignore)
            ()
        ; Palette.text p (sprintf "Steps added: %d" count)
        ; Palette.text
            p
            ~muted:true
            (sprintf "Browser cancellation requests: %d" cancellations)
        ; Palette.button p "Continue workspace" E.Ignore
        ]
    ]
;;
