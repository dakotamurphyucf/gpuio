open Core
open Gpuio
open Gpuio_protocol

let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
let scope = Focus_scope.create ~trap:true ~auto_focus:false ~restore_focus:true ()

let request =
  Wire.Message.Apply
    { window
    ; base = 0L
    ; revision = 1L
    ; operations =
        [ Create (node, Focus_scope, "", None)
        ; Set_focus_scope (node, Focus_scope.Expert.to_wire scope)
        ; Set_root (Some node)
        ]
    }
;;

let events = [ Wire.Event.Editor_result (7L, window, node, Failed Focus_blocked) ]

let%expect_test "focus scope request and blocked-focus result match Rust" =
  Eio_main.run (fun env ->
    let load name =
      let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip in
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    assert (
      String.equal
        (load "focus-v1-request.hex")
        (Wire.Message.encode request |> Or_error.ok_exn));
    let bytes = load "focus-v1-events.hex" in
    assert (
      String.equal
        bytes
        (Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events
         |> Bigstring.to_string));
    assert (List.equal Wire.Event.equal events (Wire.Event.decode bytes |> Or_error.ok_exn));
    print_s
      [%sexp (Text_input.Expert.error_of_wire Focus_blocked : Text_input.Command_error.t)]);
  [%expect {| Focus_blocked |}]
;;

let%expect_test "scope policy changes retain descendant editor identity and content" =
  let controller = Key.of_string_exn "editor" in
  let editor =
    View.text_input
      ~controller
      ~config:
        (Text_input.Config.create ~mode:Single_line ~label:"Name" () |> Or_error.ok_exn)
      ~initial_text:"initial"
      ~on_event:Fn.id
      ()
    |> Or_error.ok_exn
  in
  let reconciler = Reconciler.create window in
  let commit config =
    let update =
      Reconciler.prepare
        reconciler
        ~theme:Theme.default
        (Some (View.focus_scope ~key:(Key.of_string_exn "scope") ~config [ editor ]))
      |> Or_error.ok_exn
    in
    Reconciler.accept reconciler update |> Or_error.ok_exn;
    match Reconciler.message update with
    | Some (Wire.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  ignore (commit (Focus_scope.create ()) : Wire.Op.t list);
  print_s [%sexp (commit scope : Wire.Op.t list)];
  print_s [%sexp (commit scope : Wire.Op.t list)];
  [%expect
    {|
    ((Set_focus_scope ((slot 0) (generation 1))
      ((trap true) (auto_focus false) (restore_focus true))))
    ()
    |}]
;;
