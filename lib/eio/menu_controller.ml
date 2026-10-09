open Core
module B = Bonsai.Cont
module Menu = Gpuio.Menu

type t =
  { key : Gpuio.Key.t
  ; snapshot : Menu.Snapshot.t option
  ; observe : Menu.Snapshot.t option -> unit Bonsai.Effect.t
  ; send :
      Menu.Snapshot.t
      -> Menu.Command.t
      -> (unit, Menu.Command_error.t) Result.t Bonsai.Effect.t
  }

let create window graph =
  let snapshot, observe = B.state_opt ~equal:Menu.Snapshot.equal graph in
  let key = B.path_id graph in
  let open B.Let_syntax in
  let%arr snapshot = snapshot
  and observe = observe
  and key = key in
  { key = Gpuio.Key.of_string_exn key
  ; snapshot
  ; observe
  ; send = App.Window.Expert.menu_command window
  }
;;

let key t = t.key
let snapshot t = t.snapshot
let observe t snapshot = t.observe (Some snapshot)
let reset t = t.observe None

let command t command =
  match t.snapshot with
  | None -> Bonsai.Effect.return (Error Menu.Command_error.Not_mounted)
  | Some snapshot -> t.send snapshot command
;;
