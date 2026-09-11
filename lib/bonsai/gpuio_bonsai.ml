module View = struct
  type t = unit Bonsai.Effect.t Gpuio.View.t

  let text = Gpuio.View.text

  let button ?key ?style ?accessible_name ~on_click text =
    Gpuio.View.button ?key ?style ?accessible_name ~on_click:(fun () -> on_click) text
  ;;

  let row = Gpuio.View.row
  let column = Gpuio.View.column
  let grid = Gpuio.View.grid
end
