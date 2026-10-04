include Search_bar_component

let create window ~editor graph =
  let target =
    Bonsai.Cont.map editor ~f:(fun editor ->
      { search = Text_input.search_snapshot editor
      ; command = Text_input.search_command editor
      })
  in
  Search_bar_component.create (App.Window.Expert.editor_command window) ~target graph
;;
