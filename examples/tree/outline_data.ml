open Core
module T = Gpuio.Tree
module I = Gpuio.Tree_interaction

let ok = Or_error.ok_exn
let id name = T.Id.of_string name |> ok

let initial () =
  let leaf name text = T.Node.create ~label:name ~children:Leaf text |> ok in
  let folder name children =
    T.Node.create
      ~label:name
      ~children:(Branch { ids = List.map children ~f:id; next = End })
      "A project folder"
    |> ok
  in
  T.create
    ~roots:(List.map [ "inbox"; "archive" ] ~f:id)
    [ id "inbox", folder "Inbox" [ "plan"; "notes" ]
    ; id "plan", leaf "Project plan" "A native workspace for the next idea."
    ; id "notes", leaf "Research notes" "Keep decisions close to the work."
    ; id "archive", folder "Archive" [ "release" ]
    ; id "release", leaf "Release checklist" "Build, validate, document, ship."
    ]
  |> ok
;;

let approve snapshot ~state proposal =
  let open Or_error.Let_syntax in
  if not (I.Move.is_current proposal snapshot ~state)
  then Or_error.error_string "Move expired: its source or destination changed."
  else (
    let tree = Gpuio.Tree_loading.Snapshot.tree snapshot in
    let source = I.Target.id (I.Move.source proposal) in
    let destination = I.Target.id (I.Move.destination proposal) in
    let old_parent = (T.position tree source |> Option.value_exn).parent in
    let new_parent =
      match I.Move.placement proposal with
      | Inside -> Some destination
      | Before | After -> (T.position tree destination |> Option.value_exn).parent
    in
    let%bind () =
      match new_parent with
      | None -> Ok ()
      | Some parent ->
        (match T.Node.children (T.find tree parent |> Option.value_exn) with
         | Branch { next = End; _ } -> Ok ()
         | Branch { next = More _; _ } | Leaf ->
           Or_error.error_string "Load the complete destination before moving an item.")
    in
    let update parent members =
      let members =
        if Option.equal T.Id.equal parent old_parent
        then List.filter members ~f:(fun member -> not (T.Id.equal member source))
        else members
      in
      if not (Option.equal T.Id.equal parent new_parent)
      then members
      else (
        match I.Move.placement proposal with
        | Inside -> members @ [ source ]
        | (Before | After) as placement ->
          List.concat_map members ~f:(fun member ->
            if T.Id.equal member destination
            then (
              match placement with
              | Before -> [ source; member ]
              | After -> [ member; source ]
              | Inside -> assert false)
            else [ member ]))
    in
    let%bind nodes =
      T.to_alist tree
      |> List.map ~f:(fun (id, node) ->
        match T.Node.children node with
        | Leaf -> Ok (id, node)
        | Branch { ids; next } ->
          let children = update (Some id) ids in
          if List.equal T.Id.equal children ids
          then Ok (id, node)
          else (
            let%map node =
              T.Node.create
                ~label:(T.Node.label node)
                ~disabled:(T.Node.is_disabled node)
                ~children:(Branch { ids = children; next })
                (T.Node.data node)
            in
            id, node))
      |> Or_error.all
    in
    T.replace tree ~roots:(update None (T.roots tree)) nodes)
;;
