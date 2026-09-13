module View = struct
  type t = unit Bonsai.Effect.t Gpuio.View.t

  let text = Gpuio.View.text

  let button ?key ?style ?accessible_name ?disabled ~on_click text =
    Gpuio.View.button
      ?key
      ?style
      ?accessible_name
      ?disabled
      ~on_click:(fun () -> on_click)
      text
  ;;

  let checkbox ?key ?style ?accessible_name ?disabled ~state ~on_toggle text =
    Gpuio.View.checkbox
      ?key
      ?style
      ?accessible_name
      ?disabled
      ~state
      ~on_toggle:(fun () -> on_toggle)
      text
  ;;

  let switch ?key ?style ?accessible_name ?disabled ~checked ~on_toggle text =
    Gpuio.View.switch
      ?key
      ?style
      ?accessible_name
      ?disabled
      ~checked
      ~on_toggle:(fun () -> on_toggle)
      text
  ;;

  let row = Gpuio.View.row
  let column = Gpuio.View.column
  let grid = Gpuio.View.grid
end
