open Core
open Gpuio
module P = Choice_picker
module State = Gpuio_gallery_model.Picker_state

let ok = Or_error.ok_exn
let id value = Choice.Id.of_string value |> ok
let apply t actions = List.fold actions ~init:t ~f:State.apply

let%expect_test
    "controlled picker separates open intent from visibility and applies current \
     permission"
  =
  let t =
    apply State.initial [ Request_open true; Observe (Changed (false, Unavailable)) ]
  in
  assert (State.requested_open t && not (State.visible t));
  let t =
    apply t [ Observe (Changed (true, Application)); Request (Select (id "workspace")) ]
  in
  assert (State.visible t);
  let t =
    apply
      t
      [ Request (Select (id "archive"))
      ; Request (Toggle (id "downloads"))
      ; Request (Select (id "missing"))
      ]
  in
  print_endline (State.selected_label t);
  let t =
    apply
      t
      [ Toggle_permission
      ; Request_open true
      ; Request (Select (id "downloads"))
      ; Request Clear
      ]
  in
  assert ((not (State.can_open t)) && not (State.requested_open t));
  print_endline (State.selected_label t);
  let t =
    apply
      t
      [ Request_open false
      ; Observe (Changed (false, Application))
      ; Toggle_permission
      ; Request_open true
      ; Request (Select (id "downloads"))
      ; Request_open false
      ]
  in
  assert (not (State.requested_open t));
  print_endline (State.selected_label t);
  let t = State.apply t Reset in
  print_endline (State.selected_label t);
  [%expect
    {|
    Current workspace
    Current workspace
    Downloads
    Not chosen
    |}]
;;

let%expect_test
    "large and empty gallery catalogs produce valid public picker descriptions"
  =
  let config options selected =
    P.Config.create
      ~label:"Gallery workspaces"
      ~options
      ~selected
      ~search:Substring
      ~clearable:true
      ()
    |> ok
  in
  let selected = P.Selection.single (Some (id "workspace-4096")) in
  let large = config State.workspaces selected in
  let wire = P.Expert.to_wire large in
  (match wire.options with
   | Flat choices ->
     assert (List.length choices = State.workspace_count);
     print_s [%sexp (List.length choices : int), ((List.last_exn choices).id : string)]
   | Grouped _ -> assert false);
  List.iter
    [ large
    ; config State.empty (P.Selection.single None)
    ; config State.first_workspace (P.Selection.single None)
    ]
    ~f:(fun config ->
      let query = P.Query.create ~controller:(Key.of_string_exn "query") () |> ok in
      let description =
        P.Description.create
          ~config
          ~query
          ~empty:(View.text "Create a workspace")
          ~footer:(View.button ~on_click:(fun () -> ()) "Create workspace")
          ()
        |> ok
      in
      ignore (View.choice_picker ~on_event:(fun _ -> ()) description |> ok : unit View.t);
      ignore
        (P.Expert.description_to_wire description ~theme:Theme.default |> ok
         : Gpuio_protocol.Wire.Choice_picker_presentation.t));
  print_endline "large, empty and populated descriptions accepted";
  [%expect
    {|
    (4096 workspace-4096)
    large, empty and populated descriptions accepted
    |}]
;;
