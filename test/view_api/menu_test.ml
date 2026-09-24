open Core
open Gpuio
open Gpuio_protocol

let id name = Command.Id.of_string name |> Or_error.ok_exn
let menu items = Menu.create ~label:"Actions" items |> Or_error.ok_exn
let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let commands =
  Command.Registry.create
    [ Command.create ~id:(id "run") ~label:"Run" ~on_invoke:(fun () -> "run") ()
      |> Or_error.ok_exn
    ]
  |> Or_error.ok_exn
;;

let%expect_test "bounded menu trees and menu bars validate before native submission" =
  let leaf = menu [ Command (id "run") ] in
  let deepest =
    List.fold (List.init 7 ~f:Fn.id) ~init:leaf ~f:(fun child _ -> menu [ Submenu child ])
  in
  assert (Result.is_error (Menu.create ~label:"Too deep" [ Submenu deepest ]));
  assert (Result.is_error (Menu.create ~label:" " []));
  assert (
    Result.is_error
      (Menu.create ~label:"Items" (List.init 1025 ~f:(fun _ -> Menu.Item.Separator))));
  assert (Result.is_error (View.menu_bar (List.init 33 ~f:(fun _ -> leaf))));
  let half = menu (List.init 600 ~f:(fun _ -> Menu.Item.Command (id "run"))) in
  assert (Result.is_error (View.menu_bar [ half; half ]));
  [%expect {| |}]
;;

let%expect_test
    "menu references and platform-bar uniqueness survive physically shared children"
  =
  let shared = View.menu_button ~menu:(menu [ Command (id "run") ]) () in
  let reconciler = Reconciler.create window in
  let prepare view = Reconciler.prepare reconciler ~theme:Theme.default (Some view) in
  let first = prepare (View.command_scope ~commands [ shared ]) |> Or_error.ok_exn in
  Reconciler.accept reconciler first |> Or_error.ok_exn;
  assert (Result.is_error (prepare (View.column [ shared ])));
  let bar = View.menu_bar [ menu [ Command (id "run") ] ] |> Or_error.ok_exn in
  assert (Result.is_error (prepare (View.command_scope ~commands [ bar; bar ])));
  let plain_bar =
    View.menu_bar ~platform:false [ menu [ Command (id "run") ] ] |> Or_error.ok_exn
  in
  assert (Result.is_ok (prepare (View.command_scope ~commands [ bar; plain_bar ])));
  [%expect {| |}]
;;

let%expect_test
    "menu presentations and recursive definitions match independent Rust fixture"
  =
  let node slot = Node_id.create ~slot ~generation:1L |> Or_error.ok_exn in
  let handler = Handler_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let nested =
    Menu.create ~label:"More" ~disabled:true [ Command (id "run") ] |> Or_error.ok_exn
  in
  let definition =
    Menu.create ~label:"File" [ Command (id "run"); Separator; Submenu nested ]
    |> Or_error.ok_exn
  in
  let command =
    List.hd_exn (Command.Registry.to_list commands)
    |> Command.Expert.to_wire ~generation:1L
  in
  let operations =
    [ Wire.Op.Create (node 0L, Command_scope, "", Some handler)
    ; Set_commands (node 0L, [ command ])
    ]
    @ List.concat_map
        [ 1L, Wire.Menu_presentation.Button; 2L, Context; 3L, Bar; 4L, Platform_bar ]
        ~f:(fun (slot, presentation) ->
          [ Wire.Op.Create (node slot, Menu, "", None)
          ; Set_menu
              ( node slot
              , { Wire.Menu.presentation; menus = [ Menu.Expert.to_wire definition ] } )
          ])
    @ [ Create (node 5L, Text, "Anchor", None)
      ; Splice (node 2L, 0L, 0L, [ node 5L ])
      ; Splice (node 0L, 0L, 0L, List.map [ 1L; 2L; 3L; 4L ] ~f:node)
      ; Set_root (Some (node 0L))
      ]
  in
  let message = Wire.Message.Apply { window; base = 0L; revision = 1L; operations } in
  Eio_main.run (fun env ->
    let hex =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "menus-v1-request.hex") |> String.strip
    in
    let bytes =
      String.init
        (String.length hex / 2)
        ~f:(fun index ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(index * 2) ~len:2)))
    in
    assert (String.equal bytes (Wire.Message.encode message |> Or_error.ok_exn)));
  [%expect {| |}]
;;
