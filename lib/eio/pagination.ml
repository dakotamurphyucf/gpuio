include Pagination_component

let create window ~model ?layout ~on_request graph =
  Pagination_component.create
    (App.Window.Expert.number_input_command window)
    ~model
    ?layout
    ~on_request
    graph
;;
