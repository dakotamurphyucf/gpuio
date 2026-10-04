include Color_picker_component

let create window ~config ~value ~on_change graph =
  Color_picker_component.create
    (App.Window.Expert.color_input_command window)
    ~config
    ~value
    ~on_change
    graph
;;
