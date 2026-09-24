open Core
module App = Gpuio_eio.App
module Scope = Gpuio_eio.Scope
module Document = Gpuio_eio.Document
module Description = Gpuio.Document
module Source = Gpuio.Text_source
module View = Gpuio_bonsai.View
module B = Bonsai.Cont
module E = Bonsai.Effect

let component ~handles _window graph =
  let open B.Let_syntax in
  let%arr handles = B.Expert.Var.value handles in
  ignore graph;
  match handles with
  | None -> View.text "Registering documents…"
  | Some (markdown, code, diff) ->
    let document source mode label =
      Description.Config.create ~source ~mode ~label ~layout:(Viewport 220.) ()
      |> Or_error.ok_exn
      |> View.document
    in
    View.column
      [ View.text "OCaml owns the source; native views share revisioned snapshots."
      ; document markdown Markdown "Streaming answer"
      ; View.row
          [ document code (Code Description.Language.ocaml) "OCaml"
          ; document diff Diff "Proposed change"
          ]
      ]
;;

let () =
  let self_test = Array.exists (Sys.get_argv ()) ~f:(String.equal "--self-test") in
  let completed = ref false in
  App.run (fun env app ->
    let scope = App.scope app in
    let handles = B.Expert.Var.create None in
    let window =
      App.open_window
        app
        ~title:"GPUIO documents"
        ~width:900.
        ~height:640.
        (component ~handles)
      |> Or_error.ok_exn
    in
    Scope.start
      scope
      ~f:(fun () ->
        let clock = Eio.Stdenv.clock env in
        Eio.Time.with_timeout_exn clock 20. (fun () ->
          let on_ui ui_effect =
            let promise, resolver = Eio.Promise.create () in
            Scope.Expert.enqueue scope (fun () ->
              E.Expert.handle (E.map ui_effect ~f:(Eio.Promise.resolve resolver)));
            Eio.Promise.await promise
          in
          let create source =
            match on_ui (Document.create app ~scope source) with
            | Ok document -> document
            | Error error -> raise_s [%sexp (error : Document.Error.t)]
          in
          let markdown = create (Source.empty_stream ()) in
          let code =
            create (Source.of_string "let greeting = \"Hello, λ 👨‍👩‍👧‍👦\"\n" |> Or_error.ok_exn)
          in
          let diff =
            create
              (Source.of_string
                 "--- a/greeting.ml\n\
                  +++ b/greeting.ml\n\
                  @@ -1 +1 @@\n\
                  -let n = 1\n\
                  +let n = 2\n"
               |> Or_error.ok_exn)
          in
          B.Expert.Var.set
            handles
            (Some (Document.handle markdown, Document.handle code, Document.handle diff));
          let answer =
            "# Streaming answer\n\n\
             A native **Bonsai** document with λ and 👨‍👩‍👧‍👦.\n\n\
             ```ocaml\n\
             let answer = 42\n\
             ```\n\n\
             | Feature | Owner |\n\
             |---|---|\n\
             | Source | OCaml |\n\
             | Layout | Rust |\n\n\
             [GPUI](https://www.gpui.rs)\n"
          in
          (* A byte at a time deliberately splits every multi-byte character. *)
          String.iter answer ~f:(fun byte ->
            Document.push_bytes markdown (String.of_char byte) |> Or_error.ok_exn;
            if not self_test then Eio.Time.sleep clock 0.012);
          Document.finish markdown |> Or_error.ok_exn;
          let rec published () =
            if not (Document.is_published markdown)
            then (
              Option.iter (Document.error markdown) ~f:(fun error ->
                raise_s [%sexp (error : Document.Error.t)]);
              Eio.Time.sleep clock 0.005;
              published ())
          in
          published ();
          let source = Document.source markdown |> Option.value_exn in
          assert (String.equal (Source.to_string source) answer);
          assert (Source.Status.equal (Source.status source) Complete);
          assert (Document.pending_bytes markdown = 0);
          if self_test
          then (
            let rendered, resolve = Eio.Promise.create () in
            App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
              E.of_thunk (fun () -> Eio.Promise.resolve resolve ()))
            |> Or_error.ok_exn;
            Eio.Promise.await rendered;
            print_endline
              "GPUIO_DOCUMENT_PUBLIC_OK: scoped registration, byte-split Unicode \
               streaming, terminal publication, declarative native frame";
            Document.release markdown;
            Document.release code;
            Document.release diff;
            completed := true)))
      ~on_result:(fun result ->
        E.of_thunk (fun () ->
          Or_error.ok_exn result;
          if self_test then App.shutdown app))
    |> Or_error.ok_exn
    |> fun (_ : Scope.Task.t) -> ());
  if self_test then assert !completed
;;
