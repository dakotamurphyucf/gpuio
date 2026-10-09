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

let%expect_test
    "section labels are bounded text, not command references or platform actions"
  =
  let labeled =
    menu [ Label "Actions · α"; Command (id "run"); Submenu (menu [ Label "More" ]) ]
  in
  assert (List.equal Command.Id.equal (Menu.Expert.command_ids labeled) [ id "run" ]);
  assert (Result.is_ok (View.menu_bar ~platform:false [ labeled ]));
  assert (Result.is_error (View.menu_bar [ labeled ]));
  List.iter
    [ ""; " \t"; "nul\000"; "\255"; String.make 4097 'x' ]
    ~f:(fun label ->
      assert (Result.is_error (Menu.create ~label:"Actions" [ Label label ])));
  assert (Result.is_ok (Menu.create ~label:"Actions" [ Label (String.make 4096 'x') ]));
  assert (
    Result.is_error
      (Menu.create
         ~label:"Actions"
         (List.init 64 ~f:(fun _ -> Menu.Item.Label (String.make 4096 'x')))));
  let wire = Menu.Expert.to_wire (menu [ Label "Section" ]) in
  let bytes =
    Bin_prot.Utils.bin_dump [%bin_writer: Wire.Menu_definition.t] wire
    |> Bigstring.to_string
  in
  print_endline
    (String.concat_map bytes ~f:(fun byte -> sprintf "%02x" (Char.to_int byte)));
  [%expect {| 07416374696f6e730001030753656374696f6e |}]
;;

let%expect_test "rich menu paths, passive content and replacement preserve the menu owner"
  =
  let ok = Or_error.ok_exn in
  let path = Fn.compose ok Menu.Item_path.of_list in
  List.iter
    [ []; [ 0 ]; [ -1; 0 ]; [ 32; 0 ]; [ 0; 1024 ]; List.init 10 ~f:(Fn.const 0) ]
    ~f:(fun indices -> assert (Result.is_error (Menu.Item_path.of_list indices)));
  let definition =
    menu
      [ Command (id "run")
      ; Submenu (menu [ Label "Details"; Command (id "run") ])
      ; Separator
      ]
  in
  let paths =
    Menu.Expert.item_paths [ definition ]
    |> List.map ~f:(fun (p, _) -> Menu.Item_path.to_list p)
  in
  assert (
    List.equal
      (List.equal Int.equal)
      paths
      [ [ 0; 0 ]; [ 0; 1 ]; [ 0; 1; 0 ]; [ 0; 1; 1 ]; [ 0; 2 ] ]);
  let plain = View.menu_button ~key:(Key.of_string_exn "menu") ~menu:definition () in
  let decorate view items = View.with_menu_item_content view ~items in
  let label = View.row [ View.text "Custom"; View.text "badge" ] in
  let rich =
    decorate plain [ path [ 0; 0 ], label; path [ 0; 1 ], View.text "Nested" ] |> ok
  in
  List.iter
    [ [ path [ 0; 9 ], label ]
    ; [ path [ 0; 2 ], label ]
    ; [ path [ 0; 0 ], label; path [ 0; 0 ], label ]
    ; [ path [ 0; 0 ], View.button ~on_click:(fun () -> "bad") "Nested action" ]
    ]
    ~f:(fun items -> assert (Result.is_error (decorate plain items)));
  assert (Result.is_error (decorate (View.text "Not a menu") []));
  let platform = View.menu_bar [ menu [ Command (id "run") ] ] |> ok in
  assert (Result.is_error (decorate platform [ path [ 0; 0 ], label ]));
  let too_many = View.column (List.init 4096 ~f:(fun _ -> View.text "x")) in
  assert (Result.is_error (decorate plain [ path [ 0; 0 ], too_many ]));
  let context =
    View.context_menu
      ~menu:definition
      (View.text ~key:(Key.of_string_exn "menu-content/0/0") "Target")
  in
  let context = decorate context [ path [ 0; 0 ], label ] |> ok in
  assert (
    Result.is_ok
      (Reconciler.prepare
         (Reconciler.create window)
         ~theme:Theme.default
         (Some (View.command_scope ~commands [ context ]))));
  let reconciler = Reconciler.create window in
  let commit view =
    let update =
      Reconciler.prepare
        reconciler
        ~theme:Theme.default
        (Some (View.command_scope ~commands [ view ]))
      |> ok
    in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply { operations; _ }) -> operations
    | _ -> []
  in
  ignore (commit plain : Wire.Op.t list);
  List.iter
    [ rich; decorate rich [] |> ok; rich ]
    ~f:(fun view ->
      let operations = commit view in
      assert (
        not
          (List.exists operations ~f:(function
             | Wire.Op.Create (_, Menu, _, _) -> true
             | _ -> false))));
  [%expect {| |}]
;;

let%expect_test "public rich menu transactions admit in the native tree" =
  let ok = Or_error.ok_exn in
  let definition = menu [ Command (id "run"); Submenu (menu [ Command (id "run") ]) ] in
  let decorate view =
    View.with_menu_item_content
      view
      ~items:[ Menu.Item_path.of_list [ 0; 0 ] |> ok, View.text "Fancy" ]
    |> ok
  in
  let view =
    View.command_scope
      ~commands
      [ decorate (View.menu_button ~menu:definition ())
      ; decorate (View.context_menu ~menu:definition (View.text "Target"))
      ; decorate (View.menu_bar ~platform:false [ definition ] |> ok)
      ]
  in
  let update =
    Reconciler.prepare (Reconciler.create window) ~theme:Theme.default (Some view) |> ok
  in
  let message = Reconciler.message update |> Option.value_exn in
  let bytes = Wire.Message.encode message |> ok in
  let hex = String.concat_map bytes ~f:(fun c -> sprintf "%02x" (Char.to_int c)) in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "menu-content-public.hex")
      |> String.strip
    in
    assert (String.equal expected hex));
  [%expect {| |}]
;;
