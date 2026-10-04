include Date_picker_component

let create window ~config ~value ~initial_month ~on_change graph =
  Date_picker_component.create
    (App.Window.Expert.calendar_command window)
    ~config
    ~value
    ~initial_month
    ~on_change
    graph
;;
