open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module App = Gpuio_eio.App
module Dialog = Gpuio_eio.File_dialog
module Scope = Gpuio_eio.Scope
module View = Gpuio_bonsai.View

let error_text error = Sexp.to_string_hum (Dialog.Error.sexp_of_t error)

let display_path path =
  let bytes = Gpuio.File_path.to_string path in
  if Stdlib.String.is_valid_utf_8 bytes then bytes else String.escaped bytes
;;

let component env window graph =
  let content, set_content = B.state "Open a UTF-8 text file (up to 64 KiB)." graph in
  (* Use an explicit absolute hint; an Eio cwd capability may expose ".". *)
  let directory = Gpuio.File_path.of_string "/tmp" |> Or_error.ok_exn in
  let open_config = Dialog.Open.create ~directory () |> Or_error.ok_exn in
  let save_config =
    Dialog.Save.create ~directory ~suggested_name:"notes.txt" () |> Or_error.ok_exn
  in
  let open B.Let_syntax in
  let%arr content = content
  and set_content = set_content in
  let open_text =
    let open E.Let_syntax in
    let%bind result = Dialog.open_ window ~config:open_config in
    match result with
    | Error error -> set_content (error_text error)
    | Ok None -> E.Ignore
    | Ok (Some [ path ]) ->
      E.of_thunk (fun () ->
        Scope.start
          (App.Window.scope window)
          ~f:(fun () ->
            Eio.Path.with_open_in
              Eio.Path.(Eio.Stdenv.fs env / Gpuio.File_path.to_string path)
              (fun flow -> Eio.Buf_read.(take_all (of_flow flow ~max_size:65_537))))
          ~on_result:(fun result ->
            let text =
              match result with
              | Error error -> Error.to_string_hum error
              | Ok text when String.length text > 65_536 -> "This file exceeds 64 KiB."
              | Ok text
                when Stdlib.String.is_valid_utf_8 text
                     && not (String.contains text '\000') -> text
              | Ok _ -> "This file is not UTF-8 text."
            in
            set_content text)
        |> Or_error.ok_exn
        |> fun (_ : Scope.Task.t) -> ())
    | Ok (Some ([] | _ :: _ :: _)) -> set_content "Unexpected file selection"
  in
  let choose_destination =
    let open E.Let_syntax in
    let%bind result = Dialog.save window ~config:save_config in
    match result with
    | Error error -> set_content (error_text error)
    | Ok None -> E.Ignore
    | Ok (Some path) ->
      set_content ("Chosen destination (not written): " ^ display_path path)
  in
  View.column
    ~style:
      (Gpuio.Style.create_exn
         [ Padding (Gpuio.Length.px_exn 24.); Gap (Gpuio.Length.px_exn 12.) ])
    [ View.text "Native file dialogs"
    ; View.button ~on_click:open_text "Open text file"
    ; View.button ~on_click:choose_destination "Choose save destination"
    ; View.text content
    ]
;;

let observe action target =
  E.Expert.handle (E.map action ~f:(fun result -> target := Some result))
;;

let read_self_test directory =
  let directory = Gpuio.File_path.of_string directory |> Or_error.ok_exn in
  let outcome = ref None in
  App.run (fun env app ->
    let window =
      App.open_window
        app
        ~title:"GPUIO file read test"
        ~width:520.
        ~height:360.
        (component env)
      |> Or_error.ok_exn
    in
    let clock = Eio.Stdenv.clock env in
    let selected, resolve = Eio.Promise.create () in
    let config =
      Dialog.Open.create ~directory ~accept_label:"Read test file" () |> Or_error.ok_exn
    in
    let frame, rendered = Eio.Promise.create () in
    let rec await_frame () =
      match
        App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
          E.of_thunk (fun () -> Eio.Promise.resolve rendered ()))
      with
      | Ok () -> Eio.Promise.await frame
      | Error _ ->
        Eio.Time.sleep clock 0.005;
        await_frame ()
    in
    Scope.start
      (App.scope app)
      ~f:(fun () ->
        Eio.Time.with_timeout_exn clock 30. (fun () ->
          await_frame ();
          E.Expert.handle
            (E.map (Dialog.open_ window ~config) ~f:(Eio.Promise.resolve resolve));
          match Eio.Promise.await selected with
          | Ok (Some [ path ]) ->
            let bytes = Gpuio.File_path.to_string path in
            assert (String.equal bytes (Gpuio.File_path.to_string directory ^ "/LICENSE"));
            let fs = Eio.Stdenv.fs env in
            let text =
              Eio.Path.with_open_in
                Eio.Path.(fs / bytes)
                (fun flow -> Eio.Buf_read.(take_all (of_flow flow ~max_size:65_537)))
            in
            assert (String.is_substring text ~substring:"Apache License");
            assert (String.length text < 65_536)
          | Ok (Some ([] | _ :: _ :: _)) | Ok None ->
            failwith "expected one selected file"
          | Error error -> failwith (error_text error)))
      ~on_result:(fun result ->
        E.of_thunk (fun () ->
          outcome := Some result;
          App.shutdown app))
    |> Or_error.ok_exn
    |> fun (_ : Scope.Task.t) -> ());
  Option.value_exn !outcome |> Or_error.ok_exn;
  Eio.traceln
    "GPUIO_FILE_DIALOG_READ_OK: native selection -> correlated Bonsai effect -> explicit \
     Eio file read"
;;

let () =
  match Array.to_list (Sys.get_argv ()) with
  | [ _; "--read-self-test"; directory ] -> read_self_test directory
  | _ ->
    let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
    let before_open = ref None in
    let busy = ref None in
    let window_closed = ref None in
    let already_closed = ref None in
    let shutdown_closed = ref None in
    let failures = ref [] in
    App.run (fun env app ->
      let first =
        App.open_window
          app
          ~title:"GPUIO file dialogs"
          ~width:720.
          ~height:480.
          (component env)
        |> Or_error.ok_exn
      in
      if self_test
      then (
        let second =
          App.open_window
            app
            ~title:"GPUIO shutdown picker test"
            ~width:420.
            ~height:280.
            (component env)
          |> Or_error.ok_exn
        in
        let config = Dialog.Open.create () |> Or_error.ok_exn in
        observe (Dialog.open_ first ~config) before_open;
        let clock = Eio.Stdenv.clock env in
        let start f on_success =
          Scope.start
            (App.scope app)
            ~f:(fun () -> Eio.Time.with_timeout_exn clock 15. f)
            ~on_result:(function
              | Ok () -> E.of_thunk on_success
              | Error error ->
                E.of_thunk (fun () ->
                  failures := error :: !failures;
                  App.shutdown app))
          |> Or_error.ok_exn
          |> fun (_ : Scope.Task.t) -> ()
        in
        let frame window =
          let promise, resolver = Eio.Promise.create () in
          let rec request () =
            match
              App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
                E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
            with
            | Ok () -> Eio.Promise.await promise
            | Error _ ->
              Eio.Time.sleep clock 0.005;
              request ()
          in
          request ()
        in
        start
          (fun () ->
             frame first;
             frame second)
          (fun () ->
             observe (Dialog.open_ first ~config) window_closed;
             observe (Dialog.open_ first ~config) busy;
             start
               (fun () -> Eio.Time.sleep clock 0.3)
               (fun () ->
                  App.Window.close first;
                  observe (Dialog.open_ first ~config) already_closed;
                  start
                    (fun () ->
                       while Option.is_none !window_closed do
                         Eio.Time.sleep clock 0.005
                       done)
                    (fun () ->
                       observe (Dialog.open_ second ~config) shutdown_closed;
                       start
                         (fun () -> Eio.Time.sleep clock 0.3)
                         (fun () -> App.shutdown app))))));
    if self_test
    then (
      List.iter !failures ~f:Error.raise;
      let error_is expected = function
        | Some (Error actual) -> Dialog.Error.equal expected actual
        | Some (Ok _) | None -> false
      in
      assert (error_is Not_ready !before_open);
      assert (error_is Busy !busy);
      assert (error_is Closed !window_closed);
      assert (error_is Closed !already_closed);
      assert (error_is Closed !shutdown_closed);
      Eio.traceln
        "GPUIO_FILE_DIALOG_PUBLIC_OK: pre-open/overlap errors, native window-close and \
         shutdown cancellation delivered through correlated Bonsai/Eio effects")
;;
