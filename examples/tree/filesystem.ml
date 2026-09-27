open Core
module T = Gpuio.Tree
module L = Gpuio.Tree_loading

module Entry = struct
  type t =
    { relative : string
    ; kind : Eio.File.Stat.kind
    }
end

let root_id = T.Id.of_string "root" |> Or_error.ok_exn
let id relative = T.Id.of_string ("path:" ^ relative)

let initial () =
  let node =
    T.Node.create
      ~label:"Workspace"
      ~children:(Branch { ids = []; next = More None })
      { Entry.relative = ""; kind = `Directory }
    |> Or_error.ok_exn
  in
  T.create ~roots:[ root_id ] [ root_id, node ] |> Or_error.ok_exn
;;

let relative parent =
  if T.Id.equal parent root_id
  then Ok ""
  else (
    match String.chop_prefix (T.Id.to_string parent) ~prefix:"path:" with
    | Some path
      when not
             (List.exists (String.split path ~on:'/') ~f:(fun part ->
                String.is_empty part || String.equal part "." || String.equal part ".."))
      -> Ok path
    | None | Some _ -> Or_error.error_string "invalid filesystem tree parent")
;;

let load root request =
  let open Or_error.Let_syntax in
  let%bind parent = relative (L.Request.parent request) in
  let%bind offset =
    match L.Request.cursor request with
    | None -> Ok 0
    | Some cursor -> Or_error.try_with (fun () -> Int.of_string cursor)
  in
  if offset < 0 || offset > T.max_nodes
  then Or_error.error_string "invalid directory page cursor"
  else (
    (* Scope cancellation is allowed to propagate. Only filesystem I/O errors
       become a retryable page failure. No filesystem call runs during render. *)
    try
      let names = Eio.Path.read_dir Eio.Path.(root / parent) in
      if List.length names > T.max_nodes
      then Or_error.error_string "directory exceeds example entry budget"
      else (
        let page =
          List.sub
            names
            ~pos:(Int.min offset (List.length names))
            ~len:(Int.min 128 (Int.max 0 (List.length names - offset)))
        in
        let%map nodes =
          List.map page ~f:(fun name ->
            let path = if String.is_empty parent then name else parent ^ "/" ^ name in
            let%bind key = id path in
            let kind = (Eio.Path.stat ~follow:false Eio.Path.(root / path)).kind in
            let children =
              match kind with
              | `Directory -> T.Children.Branch { ids = []; next = More None }
              | `Unknown
              | `Fifo
              | `Character_special
              | `Block_device
              | `Regular_file
              | `Symbolic_link
              | `Socket -> Leaf
            in
            let%map node =
              T.Node.create ~label:name ~children { Entry.relative = path; kind }
            in
            key, node)
          |> Or_error.all
        in
        let next_offset = offset + List.length nodes in
        { L.Page.roots = List.map nodes ~f:fst
        ; nodes
        ; next =
            (if next_offset >= List.length names
             then End
             else More (Some (Int.to_string next_offset)))
        })
    with
    | Eio.Io _ as exn -> Or_error.of_exn exn)
;;
