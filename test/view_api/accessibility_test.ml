open Core
open Gpuio
module A = Accessibility
module W = Gpuio_protocol.Accessibility_wire
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let field ?error () =
  A.Field.create ~label:"Email" ~help:"Private" ?error ~required:true () |> ok
;;

let check_fixture name bytes =
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected = Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip in
    assert (String.equal hex expected))
;;

let%expect_test "appended operation and role fixtures agree with Rust" =
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let config = A.field (field ~error:"Required" ()) |> A.Expert.to_wire in
  Wire.Message.encode
    (Apply
       { window
       ; base = 0L
       ; revision = 1L
       ; operations =
           [ Set_accessibility (node, Some config); Set_accessibility (node, None) ]
       })
  |> ok
  |> check_fixture "accessibility-request.hex";
  let roles =
    [ A.create ~role:Status ~label:"Saved" ()
    ; A.create ~role:Alert ~label:"Error" ()
    ; A.create ~role:(Heading 2) ~label:"Account" ()
    ; A.create ~role:Link ~label:"Open" ~description:"New window" ()
    ]
    |> List.map ~f:(fun value -> ok value |> A.Expert.to_wire)
  in
  Bin_prot.Utils.bin_dump [%bin_writer: W.Config.t list] roles
  |> Bigstring.to_string
  |> check_fixture "accessibility-roles.hex";
  print_endline "operation, reset, status, alert, heading and link fixtures match";
  [%expect {| operation, reset, status, alert, heading and link fixtures match |}]
;;

let%expect_test "validated semantic roles, live defaults and field text" =
  List.iter [ 0; 7; Int.max_value ] ~f:(fun level ->
    assert (Result.is_error (A.create ~role:(Heading level) ())));
  List.iter
    [ ""; "bad\000label"; "\255"; String.make 4097 'x' ]
    ~f:(fun label -> assert (Result.is_error (A.Field.create ~label ())));
  assert (Result.is_error (A.Field.create ~label:"Name" ~help:"" ()));
  assert (Result.is_error (A.Field.create ~label:"Name" ~error:"" ()));
  ignore (A.Field.create ~label:(String.make 4096 'x') () |> ok : A.Field.t);
  ignore (A.Field.create ~label:"名前 👩🏽‍💻" () |> ok : A.Field.t);
  List.iter [ A.Role.Status; Alert; Heading 2 ] ~f:(fun role ->
    let config = A.create ~role () |> ok |> A.Expert.to_wire in
    print_s [%sexp (config.live : W.Live.t)]);
  print_s [%sexp (field ~error:"Required" () : A.Field.t)];
  [%expect
    {|
    Polite
    Assertive
    Off
    ((label Email) (help (Private)) (error (Required)) (required true))
    |}]
;;

let%expect_test "field representation matches independent bounded Rust fixture" =
  let config = A.field (field ~error:"Required" ()) |> A.Expert.to_wire in
  let bytes =
    Bin_prot.Utils.bin_dump W.Config.bin_writer_t config |> Bigstring.to_string
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "accessibility-field.hex")
      |> String.strip
    in
    assert (String.equal hex expected));
  print_endline hex;
  [%expect {| 000000000105456d61696c0107507269766174650108526571756972656401 |}]
;;

let%expect_test "metadata changes preserve identity and reset independently of controls" =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let view =
    View.checkbox
      ~key:(Key.of_string "email" |> ok)
      ~state:Unchecked
      ~on_toggle:(fun () -> ())
      "Email"
  in
  let annotate error = View.with_accessibility view (A.field (field ?error ())) |> ok in
  let prepare view =
    Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok
  in
  let initial = prepare (annotate None) in
  Reconciler.accept reconciler initial |> ok;
  let same = prepare (annotate None) in
  assert (Option.is_none (Reconciler.message same));
  Reconciler.accept reconciler same |> ok;
  let error = prepare (annotate (Some "Required")) in
  (match Reconciler.message error with
   | Some (Apply { operations = [ Set_accessibility (_, Some config) ]; _ }) ->
     assert (String.equal (Option.value_exn config.field).label "Email");
     print_endline "metadata-only update"
   | _ -> assert false);
  Reconciler.accept reconciler error |> ok;
  let cleared = prepare view in
  (match Reconciler.message cleared with
   | Some (Apply { operations = [ Set_accessibility (_, None) ]; _ }) ->
     print_endline "metadata removed"
   | _ -> assert false);
  Reconciler.accept reconciler cleared |> ok;
  assert (
    Result.is_error
      (View.with_accessibility (View.text "wrong root") (A.field (field ()))));
  let link = A.create ~role:Link () |> ok in
  assert (Result.is_error (View.with_accessibility (View.column []) link));
  ignore
    (View.with_accessibility (View.button ~on_click:(fun () -> ()) "Open") link |> ok
     : unit View.t);
  let heading = A.create ~role:(Heading 2) () |> ok in
  assert (Result.is_error (View.with_accessibility view heading));
  print_endline "incompatible semantic roots rejected";
  [%expect
    {|
    metadata-only update
    metadata removed
    incompatible semantic roots rejected
    |}]
;;

let%expect_test "form layouts retain their control across optional help and errors" =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let control = View.checkbox ~state:Unchecked ~on_toggle:(fun () -> ()) "Email" in
  let view ?error ?help ?(layout = Form.Layout.Vertical) () =
    let config =
      Form.Field.create ~label:"名前 / Name" ?error ?help ~required:true () |> ok
    in
    Form.field config ~layout ~control () |> ok
  in
  let prepare view =
    Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok
  in
  let first = prepare (view ()) in
  let identity =
    match Reconciler.message first with
    | Some (Apply tx) ->
      List.find_map_exn tx.operations ~f:(function
        | Create (id, Checkbox, _, _) -> Some id
        | _ -> None)
    | _ -> assert false
  in
  Reconciler.accept reconciler first |> ok;
  List.iter
    [ view ~help:"Visible help" ()
    ; view ~help:"Visible help" ~error:"Invalid" ()
    ; view ~error:"New error" ~layout:Horizontal ()
    ; view ()
    ]
    ~f:(fun view ->
      let next = prepare view in
      let operations =
        match Reconciler.message next with
        | Some (Apply tx) -> tx.operations
        | _ -> assert false
      in
      List.iter operations ~f:(function
        | Remove id when Gpuio_protocol.Node_id.equal id identity ->
          failwith "control removed"
        | Create (_, Checkbox, _, _) -> failwith "control replaced"
        | Set_control _ -> failwith "control state reset"
        | _ -> ());
      Reconciler.accept reconciler next |> ok);
  let names = View.Expert.describe (view ()) in
  assert (List.length names.children = 2);
  assert (Result.is_error (Form.field (field ()) ~control:(View.column []) ()));
  print_endline "optional content and layout changes preserve the native control";
  [%expect {| optional content and layout changes preserve the native control |}]
;;
