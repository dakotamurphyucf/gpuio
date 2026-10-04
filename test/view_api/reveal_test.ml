open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok

let config ~expanded ~hidden =
  Disclosure.Expert.motion_config Disclosure.Motion.standard ~expanded ~hidden
  |> Option.value_exn
;;

let%expect_test "reveal fixture keeps expansion, lifetime and spring independent" =
  let message =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_reveal (node, Some (config ~expanded:true ~hidden:Unmount))
          ; Set_reveal (node, Some (config ~expanded:false ~hidden:Retain))
          ; Set_reveal (node, None)
          ]
      }
  in
  let bytes = W.Message.encode message |> ok in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  let expected =
    Eio_main.run (fun env ->
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "reveal-operation.hex") |> String.strip)
  in
  assert (String.equal hex expected);
  assert (
    Option.is_none
      (Disclosure.Expert.motion_config
         Disclosure.Motion.immediate
         ~expanded:true
         ~hidden:Retain));
  print_endline
    "tag 85: spring, Retain/Unmount, expanded/collapsed and reset match independent bytes";
  [%expect
    {| tag 85: spring, Retain/Unmount, expanded/collapsed and reset match independent bytes |}]
;;

let%expect_test "reveal changes preserve panel identity and explicit descendant lifetime" =
  let r = Reconciler.create window in
  let view ?motion ~active ~hidden () =
    View.panel
      ~key:(Key.of_string_exn "panel")
      ~label:"Details"
      ~active
      ~hidden
      ?motion
      [ View.text ~key:(Key.of_string_exn "draft") "retained content" ]
  in
  let commit view =
    let update = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept r update |> ok;
    match Reconciler.message update with
    | Some (W.Message.Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let first = commit (view ~active:true ~hidden:Retain ()) in
  let panel =
    List.find_map_exn first ~f:(function
      | W.Op.Create (id, Panel, _, _) -> Some id
      | _ -> None)
  in
  assert (
    not
      (List.exists first ~f:(function
         | W.Op.Set_reveal _ -> true
         | _ -> false)));
  let animated =
    commit (view ~motion:Disclosure.Motion.standard ~active:true ~hidden:Retain ())
  in
  (match animated with
   | [ W.Op.Set_reveal (id, Some c) ] ->
     assert (Gpuio_protocol.Node_id.equal id panel && c.expanded && c.retain)
   | _ -> assert false);
  let closed =
    commit (view ~motion:Disclosure.Motion.standard ~active:false ~hidden:Retain ())
  in
  assert (
    List.exists closed ~f:(function
      | W.Op.Set_reveal (_, Some c) -> (not c.expanded) && c.retain
      | _ -> false));
  assert (
    not
      (List.exists closed ~f:(function
         | W.Op.Remove _ -> true
         | _ -> false)));
  let unmounted =
    commit (view ~motion:Disclosure.Motion.standard ~active:false ~hidden:Unmount ())
  in
  assert (
    List.exists unmounted ~f:(function
      | W.Op.Remove _ -> true
      | _ -> false));
  let reopened =
    commit (view ~motion:Disclosure.Motion.standard ~active:true ~hidden:Unmount ())
  in
  assert (
    List.exists reopened ~f:(function
      | W.Op.Create (_, Text, _, _) -> true
      | _ -> false));
  assert (
    not
      (List.exists reopened ~f:(function
         | W.Op.Create (_, Panel, _, _) -> true
         | _ -> false)));
  let reset = commit (view ~active:true ~hidden:Unmount ()) in
  (match reset with
   | [ W.Op.Set_reveal (id, None) ] -> assert (Gpuio_protocol.Node_id.equal id panel)
   | _ -> assert false);
  print_endline
    "default immediate; stable panel; Retain keeps children; Unmount removes them \
     immediately; reset clears motion";
  [%expect
    {| default immediate; stable panel; Retain keeps children; Unmount removes them immediately; reset clears motion |}]
;;

let%expect_test "all disclosure helpers forward motion to the same panel contract" =
  let motion = Disclosure.Motion.standard in
  let item = Choice.create ~id:(Choice.Id.of_string "one" |> ok) ~label:"One" () |> ok in
  let model =
    Disclosure.create
      ~items:(Choice.Collection.create [ item ] |> ok)
      ~mode:(Single { allow_empty = true })
      ~expanded:[]
      ()
    |> ok
  in
  let views =
    [ View.disclosure
        ~motion
        ~label:"One"
        ~expanded:false
        ~hidden:Retain
        ~on_toggle:ignore
        [ View.text "body" ]
    ; View.disclosure_with_header
        ~motion
        ~label:"One"
        ~expanded:false
        ~hidden:Retain
        ~header:[ View.text "title" ]
        ~trigger:(View.button ~on_click:ignore "Toggle")
        [ View.text "body" ]
      |> ok
    ; View.accordion
        ~motion
        ~model
        ~hidden:Retain
        ~on_request:ignore
        ~content:(fun _ -> [ View.text "body" ])
        ()
    ; View.accordion_with_labels
        ~motion
        ~model
        ~hidden:Retain
        ~labels:[]
        ~on_request:ignore
        ~content:(fun _ -> [ View.text "body" ])
        ()
      |> ok
    ]
  in
  let rec panels view =
    let description = View.Expert.describe view in
    Option.to_list description.reveal @ List.concat_map description.children ~f:panels
  in
  List.iter views ~f:(fun view ->
    match panels view with
    | [ config ] -> assert ((not config.expanded) && config.retain)
    | _ -> assert false);
  print_endline
    "plain, custom-header, accordion and rich accordion share motion/lifetime \
     configuration";
  [%expect
    {| plain, custom-header, accordion and rich accordion share motion/lifetime configuration |}]
;;
