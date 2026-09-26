open Core
module T = Gpuio.Tree
module L = Gpuio.Tree_loading
module I = Gpuio.Tree_interaction

let ok = Or_error.ok_exn
let id name = T.Id.of_string name |> ok
let leaf label text = T.Node.create ~label ~children:Leaf text |> ok

let folder label ids next =
  T.Node.create ~label ~children:(Branch { ids; next }) "A simulated source collection"
  |> ok
;;

let initial () =
  T.create
    ~roots:(List.map [ "project"; "notes"; "archive" ] ~f:id)
    [ id "project", folder "Project sources" [] (More None)
    ; id "notes", folder "Research notes" [] (More None)
    ; id "archive", folder "Archive" [] End
    ]
  |> ok
;;

let empty () = T.create ~roots:[] [] |> ok

let large () =
  let nodes =
    List.init 99_997 ~f:(fun index ->
      let number = index + 1 in
      ( id (sprintf "source-%06d" number)
      , leaf
          (sprintf "Source %06d" number)
          (sprintf "Generated source %d. This is in-memory sample data." number) ))
  in
  T.create
    ~roots:(List.map [ "project"; "notes"; "archive" ] ~f:id)
    ((id "project", folder "Project sources" (List.map nodes ~f:fst) End)
     :: (id "notes", folder "Research notes" [] End)
     :: (id "archive", folder "Archive" [] End)
     :: nodes)
  |> ok
;;

let load ~attempt request =
  if Option.is_some (L.Request.cursor request)
  then Or_error.error_string "Unexpected sample cursor"
  else if T.Id.equal (L.Request.parent request) (id "notes") && attempt = 1
  then Or_error.error_string "Sample notes could not load. Retry to continue."
  else (
    let nodes =
      if T.Id.equal (L.Request.parent request) (id "project")
      then
        Ok
          [ ( id "app"
            , leaf
                "App.ml"
                "The simulated application entry point. Builds a native window and wires \
                 its scoped services." )
          ; ( id "theme"
            , leaf
                "Theme.ml"
                "The simulated semantic palette. Shares light and dark colors across the \
                 workspace." )
          ; ( id "readme"
            , leaf
                "README.md"
                "The simulated project guide. Explains how to build, test and explore \
                 the sample." )
          ]
      else if T.Id.equal (L.Request.parent request) (id "notes")
      then
        Ok
          [ ( id "decisions"
            , leaf
                "Design decisions.md"
                "Keep native input and rendering in Rust; keep application data in OCaml."
            )
          ; ( id "checks"
            , leaf
                "Validation.md"
                "Validate keyboard, pointer, accessibility, cancellation and resource \
                 cleanup." )
          ]
      else Or_error.error_string "This sample collection has no lazy children"
    in
    let%map.Or_error nodes = nodes in
    { L.Page.roots = List.map nodes ~f:fst; nodes; next = End })
;;

let approve snapshot ~state proposal =
  let open Or_error.Let_syntax in
  if not (I.Move.is_current proposal snapshot ~state)
  then Or_error.error_string "Move expired: the source or destination changed."
  else (
    let tree = L.Snapshot.tree snapshot in
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
           Or_error.error_string "Load the full destination before moving a source.")
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
        | Before ->
          List.concat_map members ~f:(fun member ->
            if T.Id.equal member destination then [ source; member ] else [ member ])
        | After ->
          List.concat_map members ~f:(fun member ->
            if T.Id.equal member destination then [ member; source ] else [ member ]))
    in
    let%bind nodes =
      T.to_alist tree
      |> List.map ~f:(fun (id, node) ->
        match T.Node.children node with
        | Leaf -> Ok (id, node)
        | Branch { ids; next } ->
          let children = update (Some id) ids in
          if List.equal T.Id.equal ids children
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
