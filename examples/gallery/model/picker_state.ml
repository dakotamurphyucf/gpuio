open Core
open Gpuio
module P = Choice_picker

let ok = Or_error.ok_exn
let id = Choice.Id.of_string

let collection choices =
  Choice.Collection.create
    (List.map choices ~f:(fun (key, label, disabled) ->
       Choice.create ~id:(id key |> ok) ~label ~disabled () |> ok))
  |> ok
  |> P.Collection.flat
;;

let destinations =
  collection
    [ "workspace", "Current workspace", false
    ; "downloads", "Downloads", false
    ; "archive", "Team archive · unavailable", true
    ]
;;

type t =
  { selected : P.Selection.t
  ; can_open : bool
  ; requested_open : bool
  ; visible : bool
  }
[@@deriving equal]

module Action = struct
  type t =
    | Request of P.Request.t
    | Request_open of bool
    | Observe of P.Visibility.t
    | Toggle_permission
    | Reset
end

let initial =
  { selected = P.Selection.single None
  ; can_open = true
  ; requested_open = false
  ; visible = false
  }
;;

let config t =
  P.Config.create
    ~label:"Export destination"
    ~options:destinations
    ~selected:t.selected
    ~disabled:(not t.can_open)
    ~open_state:(Controlled t.requested_open)
    ~clearable:true
    ~placeholder:"Choose a destination"
    ()
  |> ok
;;

let apply t = function
  | Action.Request request ->
    if t.can_open
    then { t with selected = P.Config.apply_request (config t) request }
    else t
  | Request_open requested_open ->
    { t with requested_open = requested_open && t.can_open }
  | Observe (Snapshot visible | Changed (visible, _)) -> { t with visible }
  | Toggle_permission ->
    let can_open = not t.can_open in
    { t with can_open; requested_open = t.requested_open && can_open }
  | Reset -> { t with selected = P.Selection.single None; requested_open = false }
;;

let can_open t = t.can_open
let requested_open t = t.requested_open
let visible t = t.visible

let selected_label t =
  match P.Selection.ids t.selected with
  | [] -> "Not chosen"
  | [ selected ] ->
    if String.equal (Choice.Id.to_string selected) "workspace"
    then "Current workspace"
    else "Downloads"
  | _ -> assert false
;;

let workspace_count = 4096

let workspaces =
  collection
    (List.init workspace_count ~f:(fun index ->
       let number = index + 1 in
       sprintf "workspace-%04d" number, sprintf "Workspace %04d" number, false))
;;

let first_workspace = collection [ "first", "My first workspace", false ]
let empty = collection []
