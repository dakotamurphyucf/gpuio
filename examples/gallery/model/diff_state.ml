open Core
module Diff = Gpuio.Document.Diff

type t =
  { controlled : bool
  ; collapsed : Diff.File_key.t list
  ; limit : int option
  ; word_diff : bool
  ; notice : string
  }

module Action = struct
  type t =
    | Toggle_controlled
    | Toggle_words
    | Reset
    | Observe of Diff.Event.t
end

let initial =
  { controlled = false
  ; collapsed = []
  ; limit = Some 4
  ; word_diff = true
  ; notice = "Choose a file or a changed line to explore."
  }
;;

let controlled t = t.controlled
let word_diff t = t.word_diff
let notice t = t.notice

let config t =
  Diff.Config.create
    ~collapse:
      (if t.controlled
       then Controlled t.collapsed
       else Managed { initially_collapsed = [] })
    ~line_limit:
      (if t.controlled then Controlled t.limit else Managed { initial = Some 4; step = 4 })
    ~word_diff:t.word_diff
    ()
  |> Or_error.ok_exn
;;

let file_name (file : Diff.File.t) =
  match file.key with
  | Path name -> name
  | Unnamed -> "Unnamed file"
;;

let observe t (event : Diff.Event.t) =
  match event.observation with
  | Toggle_file { file; collapsed; applied } ->
    if
      Bool.equal applied t.controlled
      || (t.controlled
          && Bool.equal
               collapsed
               (List.mem t.collapsed file.key ~equal:Diff.File_key.equal))
    then t
    else (
      let keys =
        List.filter t.collapsed ~f:(fun key -> not (Diff.File_key.equal key file.key))
      in
      let keys = if collapsed then file.key :: keys else keys in
      { t with
        collapsed = (if t.controlled then keys else t.collapsed)
      ; notice =
          sprintf "%s %s" (if collapsed then "Collapsed" else "Expanded") (file_name file)
      })
  | Show_more { visible; hidden; applied_limit } ->
    let limit =
      if t.controlled
      then (
        match t.limit, applied_limit with
        | Some current, None when current = visible ->
          Some (Int.min 8192 (visible + Int.min 4 hidden))
        | _ -> None)
      else applied_limit
    in
    (match limit with
     | None -> t
     | Some limit ->
       { t with
         limit = (if t.controlled then Some limit else t.limit)
       ; notice = sprintf "Showing up to %d changed and context lines" limit
       })
  | Line line ->
    let number = Option.value_map ~default:"—" ~f:Int.to_string in
    { t with
      notice =
        sprintf
          "Selected %s · old %s → new %s · %s"
          (file_name line.file)
          (number line.before)
          (number line.after)
          line.text
    }
;;

let apply t = function
  | Action.Toggle_controlled ->
    { initial with controlled = not t.controlled; word_diff = t.word_diff }
  | Toggle_words -> { t with word_diff = not t.word_diff }
  | Reset -> { initial with controlled = t.controlled; word_diff = t.word_diff }
  | Observe event -> observe t event
;;
