open Core
module S = Gpuio_gallery_model.Diff_state
module D = Gpuio.Document.Diff
module W = Gpuio_protocol.Document_diff_wire

let observe state observation =
  let event =
    D.Expert.event_of_wire
      ~config:(S.config state)
      { W.Event.config_epoch = 1L
      ; source_revision = 1L
      ; source_generation = 1L
      ; observation
      }
    |> Or_error.ok_exn
  in
  S.apply state (Observe event)
;;

let file : W.File.t =
  { index = 0L
  ; key = Path "greeting.ml"
  ; before_path = Some "greeting.ml"
  ; after_path = Some "greeting.ml"
  }
;;

let show state =
  print_s
    [%sexp
      (D.Config.collapse (S.config state) : D.Collapse.t)
    , (D.Config.line_limit (S.config state) : D.Line_limit.t)
    , (S.word_diff state : bool)
    , (S.notice state : string)]
;;

let%expect_test
    "native observations preserve managed seeds; controlled intents update application \
     state"
  =
  let native =
    observe S.initial (Toggle_file { file; collapsed = true; applied = true })
  in
  let native =
    observe native (Show_more { visible = 4L; hidden = 4L; applied_limit = Some 8L })
  in
  show native;
  let controlled = S.apply native Toggle_controlled in
  let controlled =
    observe controlled (Toggle_file { file; collapsed = true; applied = false })
  in
  let controlled =
    observe controlled (Show_more { visible = 4L; hidden = 1L; applied_limit = None })
  in
  show controlled;
  show (S.apply controlled Toggle_words);
  show (S.apply controlled Reset);
  [%expect
    {|
    ((Managed (initially_collapsed ())) (Managed (initial (4)) (step 4)) true
     "Showing up to 8 changed and context lines")
    ((Controlled ((Path greeting.ml))) (Controlled (5)) true
     "Showing up to 5 changed and context lines")
    ((Controlled ((Path greeting.ml))) (Controlled (5)) false
     "Showing up to 5 changed and context lines")
    ((Controlled ()) (Controlled (4)) true
     "Choose a file or a changed line to explore.")
    |}]
;;

let%expect_test "delayed intents cannot mutate state after an ownership change" =
  let controlled = S.apply S.initial Toggle_controlled in
  let event =
    D.Expert.event_of_wire
      ~config:(S.config controlled)
      { W.Event.config_epoch = 1L
      ; source_revision = 1L
      ; source_generation = 1L
      ; observation = Toggle_file { file; collapsed = true; applied = false }
      }
    |> Or_error.ok_exn
  in
  let native = S.apply controlled Toggle_controlled in
  show (S.apply native (Observe event));
  [%expect
    {|
    ((Managed (initially_collapsed ())) (Managed (initial (4)) (step 4)) true
     "Choose a file or a changed line to explore.")
    |}]
;;
