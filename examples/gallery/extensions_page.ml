open Core
open Gpuio
module B = Bonsai.Cont
module V = Gpuio_bonsai.View
module Counter = Gpuio_example_counter
module State = Gpuio_gallery_model.Extension_state

let ok = Or_error.ok_exn
let style = Style.create_exn
let px = Length.px_exn

let component palette graph =
  let state, inject =
    B.state_machine0
      ~default_model:State.initial
      ~apply_action:(fun _ -> State.apply)
      graph
  in
  let open B.Let_syntax in
  B.Edge.lifecycle
    ~on_deactivate:
      (let%arr inject = inject in
       inject Depart)
    graph;
  let%arr p = palette
  and state = state
  and inject = inject in
  let instance =
    Counter.instance
      (Counter.Properties.create ~value:(State.value state) ~step:(State.step state) ()
       |> ok)
      ~generation:(State.generation state)
      ~disabled:(State.disabled state)
      ?set_value:(State.command state)
      ()
    |> ok
  in
  let pending = Option.is_some (State.command state) in
  let controls children = V.row ~style:(style [ Gap (px 8.); Wrap Wrap ]) children in
  V.column
    ~style:(style [ Gap (px 20.) ])
    [ Palette.card
        p
        ~title:"Make room for your own components"
        [ Palette.text
            p
            ~muted:true
            "A small native counter, packaged separately and used from ordinary OCaml."
        ; V.extension
            ~key:(Key.of_string_exn "extension-counter")
            ~style:
              (style
                 [ Display (if State.visible state then Flex else Hidden)
                 ; Width (Length.percent_exn 100.)
                 ])
            ~on_event:(fun event -> inject (Observe (State.generation state, event)))
            instance
        ; Palette.text p (sprintf "Observed counter: %d" (State.value state))
        ; Palette.text p (sprintf "Counter step: %d" (State.step state))
        ; Palette.text p (sprintf "Instance generation: %Ld" (State.generation state))
        ; Palette.text p (State.notice state)
        ; controls
            [ Palette.button
                p
                ~disabled:pending
                "Set property to 12"
                (inject Set_property)
            ; Palette.button
                p
                ~disabled:pending
                "Send command to 42"
                (inject Send_command)
            ; Palette.button
                p
                ~disabled:pending
                (if State.step state = 1 then "Use step 5" else "Use step 1")
                (inject Toggle_step)
            ]
        ; controls
            [ Palette.button
                p
                (if State.disabled state
                 then "Enable native input"
                 else "Disable native input")
                (inject Toggle_disabled)
            ; Palette.button
                p
                (if State.visible state
                 then "Hide native counter"
                 else "Show native counter")
                (inject Toggle_visible)
            ; Palette.button p "Reset native instance" (inject Reset)
            ]
        ]
    ; Palette.card
        p
        ~title:"A native component, an OCaml application"
        [ Palette.text p "Activate the counter with the pointer, Space or Enter."
        ; Palette.text
            p
            ~muted:true
            "The package owns the counter's drawing, input and accessible button. \
             Properties update it; commands request an explicit change; native events \
             report the result back to the application."
        ; Palette.text
            p
            ~muted:true
            "Hiding retains the instance. Disabling blocks user input while explicit \
             commands remain available. Reset starts a new instance at 7."
        ; Palette.text
            p
            ~muted:true
            "The counter keeps its own green artwork in both themes. This sample uses a \
             statically linked package; it does not load plugins at runtime."
        ]
    ]
;;
